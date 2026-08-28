//! `plateau-convert` — a CityGML 2.0 -> 3.0 converter for PLATEAU city models.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use plateau_converter_core::convert::{Converter, Options};
use plateau_converter_core::dataset::{Dataset, Staging};
use plateau_converter_core::profile::Rules;
use plateau_converter_core::xml::{self, Indent};
use plateau_converter_core::{PROFILES, detect, report::Report};

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

    /// i-UR version to produce. Defaults to the only one a built-in profile
    /// targets, and errors listing the choices once there is more than one.
    #[arg(long, value_name = "VERSION")]
    target_iur: Option<String>,

    /// Conversion profile (TOML), overriding both detection and --target-iur.
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

    /// i-UR version to produce, as for `convert`.
    #[arg(long, value_name = "VERSION")]
    target_iur: Option<String>,

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

    // The profile depends on the data, so it can only be chosen once the input
    // has been resolved into a tree.
    let rules = resolve_profile(
        args.profile.as_deref(),
        args.target_iur.as_deref(),
        &dataset,
    )?;
    eprintln!(
        "profile:      {} ({} -> {})",
        rules.name(),
        rules.source().label,
        rules.target().label
    );

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

    let declared = declared_namespaces(&dataset)?;
    let iur: Vec<&str> = declared
        .iter()
        .filter(|ns| ns.starts_with("https://www.geospatial.jp/iur/"))
        .map(String::as_str)
        .collect();
    println!(
        "i-UR:    {}",
        if iur.is_empty() {
            "none declared".to_string()
        } else {
            iur.join(", ")
        }
    );

    let mut candidates = built_in_profiles()?;
    println!(
        "targets: {}",
        detect::target_versions(&candidates).join(", ")
    );
    if let Some(version) = args.target_iur.as_deref() {
        candidates = detect::with_target(candidates, version)?;
    }
    match detect::select(&candidates, &declared) {
        Ok(found) => {
            let rules = &candidates[found.index];
            println!("profile: {} (built-in)", rules.name());
            println!("source:  {}", rules.source().label);
            println!("target:  {}", rules.target().label);
            if found.matched.is_empty() {
                println!("         (nothing in the data selected it; this is the fallback)");
            }
        }
        Err(error) => println!("profile: none; {error}"),
    }
    Ok(())
}

fn staging(path: Option<&std::path::Path>) -> Staging {
    match path {
        Some(path) => Staging::At(path.to_owned()),
        None => Staging::Temporary,
    }
}

/// Resolves which profile to convert `dataset` with.
///
/// `--profile` wins outright, but the input is still checked against what the
/// file says it accepts: a profile aimed at the wrong i-UR version converts the
/// CityGML half and leaves every `uro:` element behind, which looks like a
/// successful run. Without `--profile` the version is read from the data.
fn resolve_profile(
    explicit: Option<&std::path::Path>,
    target_iur: Option<&str>,
    dataset: &Dataset,
) -> Result<Rules> {
    let declared = declared_namespaces(dataset)?;

    if let Some(path) = explicit {
        if target_iur.is_some() {
            bail!("--profile already fixes the target; drop --target-iur");
        }
        let source =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let rules =
            Rules::from_toml(&source).with_context(|| format!("loading {}", path.display()))?;
        if let Some(warning) = detect::check(&rules, &declared) {
            eprintln!("warning: {warning}");
        }
        return Ok(rules);
    }

    let mut candidates = built_in_profiles()?;
    if let Some(version) = target_iur {
        candidates = detect::with_target(candidates, version)?;
    }
    let found = detect::select(&candidates, &declared)?;
    let rules = candidates[found.index].clone();
    if found.matched.is_empty() {
        eprintln!(
            "note: the input declares no i-UR namespace; using `{}` ({})",
            rules.name(),
            rules.source().label
        );
    }
    Ok(rules)
}

fn built_in_profiles() -> Result<Vec<Rules>> {
    PROFILES
        .iter()
        .map(|(name, toml)| {
            Rules::from_toml(toml).with_context(|| format!("loading built-in profile `{name}`"))
        })
        .collect()
}

/// The namespaces the dataset's own documents declare, taken from the first
/// `.gml` found. Every file in a package shares one i-UR version.
fn declared_namespaces(dataset: &Dataset) -> Result<Vec<String>> {
    for feature_type in dataset.feature_types()? {
        if let Some(path) = dataset.gml_files(&feature_type)?.first() {
            return xml::read_root_namespaces(path)
                .with_context(|| format!("reading {}", path.display()));
        }
    }
    Ok(Vec::new())
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
