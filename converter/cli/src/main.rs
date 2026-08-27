//! `plateau-convert` — a CityGML 2.0 -> 3.0 converter for PLATEAU city models.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use plateau_converter_core::convert::{Converter, Options};
use plateau_converter_core::dataset::{Dataset, Staging};
use plateau_converter_core::profile::Rules;
use plateau_converter_core::xml::Indent;
use plateau_converter_core::{DEFAULT_PROFILE, report::Report};

#[derive(Parser, Debug)]
#[command(
    name = "plateau-convert",
    version,
    about = "Convert PLATEAU CityGML 2.0 city models to CityGML 3.0",
    long_about = "Convert PLATEAU CityGML 2.0 city models to CityGML 3.0.

INPUT may be:
  * a PLATEAU package directory (holding udx/, codelists/, schemas/)
  * a zip of one
  * several zips, one per part, which are reassembled before converting"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Print per-file progress. Repeat for parser-level detail.
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Convert a dataset.
    Convert(ConvertArgs),
    /// Report how the inputs were understood, converting nothing.
    Inspect(InspectArgs),
}

#[derive(Args, Debug)]
struct ConvertArgs {
    /// Package directories or zips.
    #[arg(required = true, value_name = "INPUT")]
    inputs: Vec<PathBuf>,

    /// Directory to write the converted package into.
    #[arg(short, long, value_name = "DIR")]
    output: PathBuf,

    /// udx feature types to convert. Repeatable. `all` converts everything present.
    #[arg(short = 't', long = "type", value_name = "TYPE", default_values_t = [String::from("bldg")])]
    types: Vec<String>,

    /// Conversion profile (TOML). Defaults to the built-in CityGML 2.0 -> 3.0 profile.
    #[arg(long, value_name = "FILE")]
    profile: Option<PathBuf>,

    /// Reassemble multi-part input here instead of a temporary directory.
    #[arg(long, value_name = "DIR")]
    staging: Option<PathBuf>,

    /// Keep the temporary staging directory and print its path.
    #[arg(long)]
    keep_staging: bool,

    /// Leave geometries without a gml:id, which GML 3.2 requires.
    #[arg(long)]
    no_gml_ids: bool,

    /// Leave child elements in their input order.
    #[arg(long)]
    no_reorder: bool,

    /// Do not copy codelists/, schemas/ and friends into the output.
    #[arg(long)]
    no_support_files: bool,

    /// Output indentation.
    #[arg(long, value_enum, default_value_t = IndentArg::Tab)]
    indent: IndentArg,

    /// Worker threads. 0 uses one per core.
    #[arg(short = 'j', long, default_value_t = 0)]
    jobs: usize,

    /// Overwrite an existing output directory.
    #[arg(long)]
    force: bool,
}

#[derive(Args, Debug)]
struct InspectArgs {
    #[arg(required = true, value_name = "INPUT")]
    inputs: Vec<PathBuf>,

    /// Reassemble multi-part input here instead of a temporary directory.
    #[arg(long, value_name = "DIR")]
    staging: Option<PathBuf>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum IndentArg {
    Tab,
    Two,
    Four,
    None,
}

impl From<IndentArg> for Indent {
    fn from(value: IndentArg) -> Self {
        match value {
            IndentArg::Tab => Indent::Tab,
            IndentArg::Two => Indent::Spaces(2),
            IndentArg::Four => Indent::Spaces(4),
            IndentArg::None => Indent::None,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match &cli.command {
        Command::Convert(args) => convert(args),
        Command::Inspect(args) => inspect(args),
    }
}

fn init_tracing(verbose: u8) {
    let default = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .init();
}

fn convert(args: &ConvertArgs) -> Result<()> {
    if args.output.exists() && !args.force {
        let empty = args
            .output
            .read_dir()
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if !empty {
            bail!(
                "{} already exists and is not empty; pass --force to write into it",
                args.output.display()
            );
        }
    }

    if args.jobs > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.jobs)
            .build_global()
            .context("configuring the worker pool")?;
    }

    let rules = load_profile(args.profile.as_deref())?;
    let feature_types = if args.types.iter().any(|t| t == "all") {
        Vec::new()
    } else {
        args.types.clone()
    };
    let options = Options {
        feature_types,
        generate_gml_ids: !args.no_gml_ids,
        reorder: !args.no_reorder,
        indent: args.indent.into(),
        copy_support_files: !args.no_support_files,
        parallel: true,
    };

    let dataset = Dataset::open_with(&args.inputs, &staging(args.staging.as_deref()))
        .context("resolving the input dataset")?;
    eprintln!("dataset root: {}", dataset.root().display());
    if dataset.is_staged() {
        eprintln!("  (reassembled from {} input(s))", args.inputs.len());
    }

    let converter = Converter::new(rules, options)?;
    let report = converter
        .convert_dataset(&dataset, &args.output)
        .context("converting the dataset")?;

    print_report(&report, &args.output);

    if args.keep_staging && dataset.is_staged() {
        eprintln!("staging kept at: {}", dataset.keep().display());
    }
    Ok(())
}

fn inspect(args: &InspectArgs) -> Result<()> {
    let dataset = Dataset::open_with(&args.inputs, &staging(args.staging.as_deref()))
        .context("resolving the input dataset")?;

    println!("root:    {}", dataset.root().display());
    println!(
        "staged:  {}",
        if dataset.is_staged() {
            "yes"
        } else {
            "no (used in place)"
        }
    );
    println!("parts:   {}", join(&dataset.parts()));

    for feature_type in dataset.feature_types()? {
        let files = dataset.gml_files(&feature_type)?;
        println!("  udx/{feature_type:<6} {} file(s)", files.len());
    }
    Ok(())
}

fn staging(path: Option<&std::path::Path>) -> Staging {
    match path {
        Some(path) => Staging::At(path.to_owned()),
        None => Staging::Temporary,
    }
}

fn load_profile(path: Option<&std::path::Path>) -> Result<Rules> {
    let (source, label) = match path {
        Some(path) => (
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
            path.display().to_string(),
        ),
        None => (DEFAULT_PROFILE.to_string(), "built-in profile".to_string()),
    };
    Rules::from_toml(&source).with_context(|| format!("loading {label}"))
}

fn print_report(report: &Report, output: &std::path::Path) {
    eprintln!(
        "converted {} file(s), {} feature(s); copied {} support file(s) -> {}",
        report.converted,
        report.features,
        report.copied,
        output.display()
    );
    if !report.warnings.is_empty() {
        eprintln!("\n{} caveat(s):", report.warnings.len());
        eprint!("{}", report.warnings);
    }
}

fn join(items: &[&str]) -> String {
    if items.is_empty() {
        "(none)".to_string()
    } else {
        items.join(", ")
    }
}
