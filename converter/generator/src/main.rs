//! Derives the i-UR half of a conversion profile from the schemas themselves.
//!
//! The mapping from i-UR 3.x to 4.0 is not a judgement call: both sides are
//! published XML Schema documents, and what moved where is written in them. Six
//! hundred hand-written rules would be unreviewable and would rot; this reads
//! them instead, and records which schema revisions it read.
//!
//! Two things it deliberately does *not* do:
//!
//! * **Guess.** A name that is absent from 4.0 entirely, or present in both `uro`
//!   and `urc`, is reported for a human rather than mapped.
//! * **Read one revision.** i-UR publishes patch revisions in place under the
//!   same minor-version URL, and they are not compatible with one another --
//!   `uro` 3.0.4 and 3.0.5 differ by 34 names. Rules are therefore generated
//!   from the *union* of every known patch of a minor: a rule for a name absent
//!   from the revision at hand never fires, whereas a missing rule silently
//!   writes the element with no namespace.

mod xsd;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;
use xsd::Schema;

/// i-UR modules, and the file each one is published as.
const MODULES: &[(&str, &str)] = &[
    ("uro", "urbanObject"),
    ("urc", "urbanCore"),
    ("urf", "urbanFunction"),
    ("urg", "statisticalGrid"),
    ("urt", "publicTransit"),
];

const BEGIN: &str = "# BEGIN GENERATED -- i-UR rules";
const END: &str = "# END GENERATED -- i-UR rules";

#[derive(Parser, Debug)]
#[command(
    name = "plateau-converter-gen",
    about = "Generate the i-UR rules of a conversion profile from the schemas"
)]
struct Cli {
    /// Source i-UR minor version, e.g. `3.1`.
    #[arg(long, value_name = "MINOR")]
    source: String,

    /// Target i-UR minor version.
    #[arg(long, value_name = "MINOR", default_value = "4.0")]
    target: String,

    /// Splice the result into this profile between the generated-section
    /// markers instead of printing it.
    #[arg(long, value_name = "FILE")]
    write: Option<PathBuf>,

    /// Repository root holding `fixtures/schemas/`.
    #[arg(long, value_name = "DIR", default_value = ".")]
    root: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let sources = read_sources(&cli.root, &cli.source)?;
    let targets = read_targets(&cli.root, &cli.target)?;
    let existing = cli
        .write
        .as_deref()
        .map(hand_written_rules)
        .transpose()?
        .unwrap_or_default();
    let generated = generate(&cli.source, &cli.target, &sources, &targets, &existing)?;

    match &cli.write {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?;
            std::fs::write(path, splice(&text, &generated)?)
                .with_context(|| format!("writing {}", path.display()))?;
            eprintln!("updated {}", path.display());
        }
        None => print!("{generated}"),
    }
    Ok(())
}

/// Every vendored patch revision of one source minor, keyed by module.
///
/// `fixtures/schemas/sources` holds files named `<module>-<patch>-<hash>.xsd`; a module's
/// declarations are the union across its revisions of the requested minor.
fn read_sources(root: &Path, minor: &str) -> Result<BTreeMap<String, (Schema, Vec<String>)>> {
    let dir = root.join("fixtures/schemas/sources");
    let mut out: BTreeMap<String, (Schema, Vec<String>)> = BTreeMap::new();
    for entry in std::fs::read_dir(&dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some((module, rest)) = name.split_once('-') else {
            continue;
        };
        let Some((patch, _)) = rest.split_once('-') else {
            continue;
        };
        if !patch.starts_with(&format!("{minor}.")) {
            continue;
        }
        let schema = Schema::read(&entry.path())?;
        let slot = out.entry(module.to_owned()).or_default();
        slot.1.push(patch.to_owned());
        for (name, decl) in schema.decls {
            slot.0.decls.entry(name).or_insert(decl);
        }
    }
    if out.is_empty() {
        bail!(
            "no vendored source schemas for i-UR {minor} under {}",
            dir.display()
        );
    }
    for (_, revisions) in out.values_mut() {
        revisions.sort();
    }
    Ok(out)
}

fn read_targets(root: &Path, minor: &str) -> Result<BTreeMap<String, Schema>> {
    let mut out = BTreeMap::new();
    for (module, file) in MODULES {
        let path = root.join(format!("fixtures/schemas/iur/{module}/{minor}/{file}.xsd"));
        if path.is_file() {
            out.insert((*module).to_owned(), Schema::read(&path)?);
        }
    }
    if out.is_empty() {
        bail!("no vendored target schemas for i-UR {minor}");
    }
    Ok(out)
}

/// The `from` names of `[[element]]` rules written by hand, i.e. outside the
/// generated block.
///
/// A generated rule must never overrule a decision someone made deliberately --
/// the schemas cannot know that `uro:DataQualityAttribute` should become the
/// *Exterior* variant, for instance -- and two rules for one name is an error,
/// so the generator yields.
fn hand_written_rules(profile: &Path) -> Result<BTreeMap<String, String>> {
    let Ok(text) = std::fs::read_to_string(profile) else {
        return Ok(BTreeMap::new());
    };
    let outside = match (text.find(BEGIN), text.find(END)) {
        (Some(start), Some(end)) => format!("{}{}", &text[..start], &text[end..]),
        _ => text,
    };
    // `from` and `to` sit on consecutive lines in every rule this file writes.
    let lines: Vec<&str> = outside.lines().map(str::trim).collect();
    let mut rules = BTreeMap::new();
    for pair in lines.windows(2) {
        if let (Some(from), Some(to)) = (
            pair[0].strip_prefix("from = "),
            pair[1].strip_prefix("to = "),
        ) {
            rules.insert(
                from.trim().trim_matches('"').to_owned(),
                to.trim().trim_matches('"').to_owned(),
            );
        }
    }
    Ok(rules)
}

fn generate(
    source_minor: &str,
    target_minor: &str,
    sources: &BTreeMap<String, (Schema, Vec<String>)>,
    targets: &BTreeMap<String, Schema>,
    hand_written: &BTreeMap<String, String>,
) -> Result<String> {
    let mut moved: Vec<(String, String, String)> = Vec::new(); // from module, name, to module
    let mut ambiguous: Vec<(String, String)> = Vec::new();
    let mut unmapped: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut overridden: Vec<String> = Vec::new();

    for (module, (schema, _)) in sources {
        let Some(home) = targets.get(module) else {
            continue;
        };
        for (name, decl) in &schema.decls {
            if home.has(name) {
                continue; // the bulk namespace bump already handles it
            }
            // A global declaration is an ADE property in its own right; a nested
            // one is a field of some class. They are different things that
            // happen to share a spelling, so a global never moves to a nested
            // name or the reverse.
            let elsewhere: Vec<&String> = targets
                .iter()
                .filter(|(m, s)| {
                    *m != module && s.decls.get(name).is_some_and(|d| d.global == decl.global)
                })
                .map(|(m, _)| m)
                .collect();

            // A nested name is only evidence of a move when the class holding
            // it moved the same way. Several modules declare `lod1MultiSurface`
            // and `language`; that a name happens to exist in one other module
            // says nothing about where *this* one belongs.
            let coherent = |to: &str| match (&decl.owner, decl.global) {
                (_, true) => true,
                (Some(owner), _) => {
                    // Types are declared as `<name>Type`; the class element that
                    // carries them is the bare name.
                    let class = owner.strip_suffix("Type").unwrap_or(owner);
                    let moved_in_schema =
                        |n: &str| !home.has(n) && targets.get(to).is_some_and(|s| s.has(n));
                    // A class may have been placed by hand -- i-UR 3.0's
                    // per-feature-type classes have no counterpart to read off --
                    // and its fields follow it just the same.
                    let moved_by_hand = hand_written
                        .get(&format!("{module}:{class}"))
                        .is_some_and(|target| target.starts_with(&format!("{to}:")));
                    moved_in_schema(owner) || moved_in_schema(class) || moved_by_hand
                }
                (None, false) => false,
            };

            if hand_written.contains_key(&format!("{module}:{name}")) {
                overridden.push(format!("{module}:{name}"));
                continue;
            }

            // Several modules may spell a nested name the same way -- `key` is
            // declared by both urc and urg. The class holding it settles which
            // one is meant, so narrow before giving up.
            let fits: Vec<&String> = elsewhere
                .iter()
                .copied()
                .filter(|to| coherent(to))
                .collect();

            match fits.as_slice() {
                [to] => moved.push((module.clone(), name.clone(), (*to).clone())),
                [] if elsewhere.is_empty() => {
                    unmapped
                        .entry(module.clone())
                        .or_default()
                        .insert(name.clone());
                }
                _ => ambiguous.push((module.clone(), name.clone())),
            }
        }
    }

    let mut out = String::new();
    out.push_str(BEGIN);
    out.push('\n');
    out.push_str(&format!(
        "#\n# Derived from the schemas, not written by hand. Regenerate with:\n\
         #   cargo run -p plateau-converter-gen -- --source {source_minor} \\\n\
         #       --write profiles/iur-{source_minor}-to-{target_minor}.toml\n#\n",
    ));
    for (module, (_, revisions)) in sources {
        out.push_str(&format!(
            "# source {module} {source_minor}: union of revisions {}\n",
            revisions.join(", ")
        ));
    }
    for (module, schema) in targets {
        out.push_str(&format!(
            "# target {module} {}: {} declarations\n",
            schema.version,
            schema.decls.len()
        ));
    }

    out.push_str(
        "\n# Elements that changed module. Same local name, so the only question is\n\
         # which module now declares it, and exactly one does.\n",
    );
    for (from, name, to) in &moved {
        out.push_str(&format!(
            "[[element]]\nfrom = \"{from}:{name}\"\nto = \"{to}:{name}\"\n"
        ));
    }

    out.push_str(&format!(
        "\n# Where an i-UR class attaches in CityGML 3.0. 2.0 used a property named by\n\
         # the extension ({}); 3.0 uses one general hook per host class, so the\n\
         # wrapper is chosen by the class inside it rather than by its own name.\n\
         [ade_hooks]\n",
        "uro:buildingIDAttribute"
    ));
    for (module, schema) in targets {
        for (name, decl) in schema.globals() {
            if let Some(hook) = resolve_hook(schema, targets, decl) {
                out.push_str(&format!("\"{module}:{name}\" = \"{hook}\"\n"));
            }
        }
    }

    if !unmapped.is_empty() || !ambiguous.is_empty() {
        out.push_str("\n# Names with no rule, listed so they are not mistaken for oversights.\n");
        for (module, names) in &unmapped {
            out.push_str(&format!(
                "#   {module}: {} name(s) declared in {source_minor} and in no {target_minor} module\n",
                names.len()
            ));
            for name in names {
                out.push_str(&format!("#     {name}\n"));
            }
        }
        for (module, name) in &ambiguous {
            out.push_str(&format!(
                "#   {module}:{name}: no single {target_minor} module is both a match and \
consistent with the class holding it\n"
            ));
        }
    }
    if !overridden.is_empty() {
        out.push_str("\n# Left to the rule written by hand elsewhere in this file:\n");
        for name in &overridden {
            out.push_str(&format!("#   {name}\n"));
        }
    }
    out.push_str(END);
    out.push('\n');
    Ok(out)
}

/// The CityGML property an i-UR class hangs off, following `substitutionGroup`
/// up to a CityGML `ADEOf…` head.
///
/// The head is a class; the property that carries it is the same name with a
/// lower-case first letter (`bldg:ADEOfAbstractBuilding` ->
/// `bldg:adeOfAbstractBuilding`), which holds for every hook CityGML 3.0
/// declares.
fn resolve_hook(
    schema: &Schema,
    targets: &BTreeMap<String, Schema>,
    decl: &xsd::Decl,
) -> Option<String> {
    let mut group = decl.substitution_group.clone()?;
    for _ in 0..8 {
        let (prefix, local) = group.split_once(':')?;
        if let Some(rest) = local.strip_prefix("ADEOf") {
            let mut property = String::from("adeOf");
            property.push_str(rest);
            return Some(format!("{prefix}:{property}"));
        }
        // Still inside i-UR: climb to the parent class.
        let owner = targets.get(prefix).unwrap_or(schema);
        group = owner.decls.get(local)?.substitution_group.clone()?;
    }
    None
}

/// Replaces the generated block of `profile`, keeping everything else.
fn splice(profile: &str, generated: &str) -> Result<String> {
    let (Some(start), Some(end)) = (profile.find(BEGIN), profile.find(END)) else {
        // First run: append, so a profile need not be prepared by hand.
        return Ok(format!("{}\n{generated}", profile.trim_end()));
    };
    Ok(format!(
        "{}{generated}{}",
        &profile[..start],
        &profile[end + END.len() + 1..]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(src: &str) -> Schema {
        Schema::parse(src)
    }

    #[test]
    fn a_hook_is_the_head_class_with_a_lower_case_first_letter() {
        let s = schema(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
                 <xs:element name="BuildingIDAttribute" substitutionGroup="bldg:ADEOfAbstractBuilding"/>
               </xs:schema>"#,
        );
        let targets = BTreeMap::new();
        let decl = &s.decls["BuildingIDAttribute"];
        assert_eq!(
            resolve_hook(&s, &targets, decl).as_deref(),
            Some("bldg:adeOfAbstractBuilding")
        );
    }

    /// A class may substitute into another i-UR class; the hook is the one at
    /// the top of the chain.
    #[test]
    fn a_hook_is_followed_through_an_i_ur_parent() {
        let urc = schema(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
                 <xs:element name="DataQualityAttribute" substitutionGroup="core:ADEOfAbstractCityObject"/>
                 <xs:element name="ExteriorDataQualityAttribute" substitutionGroup="urc:DataQualityAttribute"/>
               </xs:schema>"#,
        );
        let mut targets = BTreeMap::new();
        targets.insert("urc".to_string(), urc.clone());
        let decl = &urc.decls["ExteriorDataQualityAttribute"];
        assert_eq!(
            resolve_hook(&urc, &targets, decl).as_deref(),
            Some("core:adeOfAbstractCityObject")
        );
    }

    #[test]
    fn splicing_replaces_only_the_generated_block() {
        let profile = format!("keep me\n{BEGIN}\nold\n{END}\nkeep me too\n");
        let out = splice(&profile, &format!("{BEGIN}\nnew\n{END}\n")).unwrap();
        assert!(out.contains("keep me"));
        assert!(out.contains("keep me too"));
        assert!(out.contains("new"));
        assert!(!out.contains("old"));
    }

    #[test]
    fn splicing_appends_when_there_is_no_block_yet() {
        let out = splice("only this\n", &format!("{BEGIN}\nnew\n{END}\n")).unwrap();
        assert!(out.starts_with("only this"));
        assert!(out.contains("new"));
    }
}
