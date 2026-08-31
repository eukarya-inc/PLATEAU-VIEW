//! The top-level driver: dataset in, converted tree out.

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::app::AppearanceRewrite;
use crate::bldg::BuildingRewrite;
use crate::common::CommonRewrite;
use crate::dataset::Dataset;
use crate::error::{Error, Result};
use crate::iur::IurRewrite;
use crate::lod4::Lod4Rewrite;
use crate::profile::{Lod4Fallback, Rules};
use crate::report::{FileReport, Report, Warnings};
use crate::transform::{self, IdGen};
use crate::xml::{self, Chunk, Element, Indent, Node, Reader, Writer};

/// Directories copied through unchanged when converting a whole dataset.
///
/// `schemas/` is deliberately absent: the input's copy describes i-UR 3.x, and
/// carrying it into a 4.0 package would leave a schema tree nothing references.
/// It is replaced by [`write_iur_schemas`] instead. `codelists/` is absent for
/// the same reason and is rebuilt by [`write_codelists`].
const COPIED_PARTS: &[&str] = &["metadata", "specification"];

#[derive(Debug, Clone)]
pub struct Options {
    /// `udx` subdirectories to convert. Empty means every one present.
    pub feature_types: Vec<String>,
    /// Mint `gml:id` for geometries that lack one, which GML 3.2 requires.
    pub generate_gml_ids: bool,
    /// Sort children into the order the 3.0 content models declare.
    pub reorder: bool,
    pub indent: Indent,
    /// Copy `codelists/`, `schemas/` and friends alongside the converted `udx/`.
    pub copy_support_files: bool,
    /// Convert files concurrently.
    pub parallel: bool,
    /// Where LOD4 goes when the data does not say. `None` takes the profile's
    /// `[lod4] fallback`.
    pub lod4_fallback: Option<Lod4Fallback>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            feature_types: vec!["bldg".to_string()],
            generate_gml_ids: true,
            reorder: true,
            indent: Indent::Tab,
            copy_support_files: true,
            parallel: true,
            lod4_fallback: None,
        }
    }
}

pub struct Converter {
    rules: Rules,
    common: CommonRewrite,
    app: AppearanceRewrite,
    lod4: Lod4Rewrite,
    bldg: BuildingRewrite,
    iur: IurRewrite,
    gml_ns: String,
    options: Options,
}

impl Converter {
    pub fn new(rules: Rules, options: Options) -> Result<Self> {
        let common = CommonRewrite::new(&rules)?;
        let app = AppearanceRewrite::new(&rules)?;
        let lod4 = Lod4Rewrite::new(&rules, options.lod4_fallback)?;
        let bldg = BuildingRewrite::new(&rules)?;
        let iur = IurRewrite::new(&rules)?;
        let gml_ns = rules.output_ns("gml")?.to_owned();
        Ok(Converter {
            rules,
            common,
            app,
            lod4,
            bldg,
            iur,
            gml_ns,
            options,
        })
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Converts the selected feature types of `dataset` into `out`, mirroring the
    /// input's directory layout.
    pub fn convert_dataset(&self, dataset: &Dataset, out: &Path) -> Result<Report> {
        let requested = if self.options.feature_types.is_empty() {
            dataset.feature_types()?
        } else {
            self.options.feature_types.clone()
        };

        let mut jobs: Vec<(PathBuf, PathBuf)> = Vec::new();
        for feature_type in &requested {
            let files = dataset.gml_files(feature_type)?;
            if files.is_empty() {
                tracing::warn!(feature_type, "no .gml files found");
            }
            for input in files {
                let relative = input.strip_prefix(dataset.root()).map_err(|_| {
                    Error::Layout(format!("{} is outside the dataset root", input.display()))
                })?;
                jobs.push((input.clone(), out.join(relative)));
            }
        }

        let results: Vec<Result<FileReport>> = if self.options.parallel {
            jobs.par_iter()
                .map(|(i, o)| self.convert_file(i, o))
                .collect()
        } else {
            jobs.iter().map(|(i, o)| self.convert_file(i, o)).collect()
        };

        let mut report = Report::default();
        for result in results {
            report.absorb(&result?);
        }

        // The documents reference their non-GML companions — texture images
        // above all — by relative path, so a converted tree without them would
        // render untextured. They are copied verbatim to the mirrored path.
        for feature_type in &requested {
            for input in dataset.companion_files(feature_type)? {
                let relative = input.strip_prefix(dataset.root()).map_err(|_| {
                    Error::Layout(format!("{} is outside the dataset root", input.display()))
                })?;
                let target = out.join(relative);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
                }
                fs::copy(&input, &target).map_err(|e| Error::io(&target, e))?;
                report.copied += 1;
            }
        }

        // Untouched feature types would leave a tree that is half 2.0 and half
        // 3.0, so say so rather than copying them in silently.
        for feature_type in dataset.feature_types()? {
            if !requested.contains(&feature_type) {
                report.warnings.add(format!(
                    "udx/{feature_type} was not converted and is absent from the output"
                ));
            }
        }

        if self.options.copy_support_files {
            for part in COPIED_PARTS {
                let source = dataset.root().join(part);
                if source.is_dir() {
                    report.copied += copy_tree(&source, &out.join(part))?;
                }
            }
            report.copied += write_iur_schemas(out)?;
            report.copied +=
                write_codelists(dataset.root(), out, &self.rules, &mut report.warnings)?;
            if dataset.root().join("schemas").is_dir() {
                report.warnings.add(
                    "the input's schemas/ describes i-UR 3.x and was replaced by the \
                     i-UR 4.0 schemas; CityGML is referenced at schemas.opengis.net, \
                     as a PLATEAU package does",
                );
            }
        }

        Ok(report)
    }

    pub fn convert_file(&self, input: &Path, output: &Path) -> Result<FileReport> {
        let source = xml::read_to_string(input)?;
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let file = File::create(output).map_err(|e| Error::io(output, e))?;
        let mut sink = BufWriter::new(file);
        let label = input.file_name().map(|n| n.to_string_lossy().into_owned());
        let report = self.convert(label.as_deref().unwrap_or("input"), &source, &mut sink)?;
        sink.flush().map_err(|e| Error::io(output, e))?;
        tracing::debug!(input = %input.display(), features = report.features, "converted");
        Ok(report)
    }

    /// Converts one CityGML document. `label` names the input in diagnostics.
    pub fn convert(&self, label: &str, source: &str, sink: impl Write) -> Result<FileReport> {
        let mut reader = Reader::new(label, source);
        let mut writer = Writer::new(sink, self.rules.prefixes().clone(), self.options.indent);
        let mut report = FileReport::default();
        let mut root: Option<Element> = None;

        // The output is always UTF-8, whatever the input declared.
        writer.write_declaration(None)?;

        while let Some(chunk) = reader.next_chunk()? {
            match chunk {
                Chunk::Decl => {}
                Chunk::Prologue(Node::Comment(text)) => writer.write_comment(&text)?,
                Chunk::Prologue(_) => {}
                Chunk::RootStart(element) => {
                    let element = transform::rename(&self.rules, element).ok_or_else(|| {
                        Error::Profile(format!("the profile drops the root element of {label}"))
                    })?;
                    writer.write_root_start(&element, self.rules.schema_location())?;
                    root = Some(element);
                }
                Chunk::Member(element) => {
                    if root.is_none() {
                        return Err(Error::malformed(label, "content before the root element"));
                    }
                    if let Some(element) = self.convert_member(element, &mut report) {
                        writer.write_element(&element, 1)?;
                    }
                }
                Chunk::RootContent(Node::Comment(text)) => {
                    writer.write_comment_at(&text, 1)?;
                }
                Chunk::RootContent(_) => {}
                Chunk::RootEnd => {
                    if let Some(root) = &root {
                        writer.write_root_end(root)?;
                    }
                }
            }
        }

        let root = root.ok_or_else(|| Error::malformed(label, "no root element"))?;
        if root.children.is_empty() && report.features == 0 {
            // An empty root is legal but almost always a sign of a bad input.
            report.warnings.add("the document held no city objects");
        }
        for namespace in writer.missing_namespaces() {
            report.warnings.add(format!(
                "no prefix is declared for `{namespace}`; elements in it were written \
                 unqualified. Add it to [output.namespaces] in the profile"
            ));
        }
        writer.flush()?;
        Ok(report)
    }

    /// Renames, restructures and reorders one top-level member.
    fn convert_member(&self, element: Element, report: &mut FileReport) -> Option<Element> {
        let mut element = transform::rename(&self.rules, element)?;

        // Seeding ids from the feature's own gml:id keeps generated ids unique
        // across a dataset and stable across runs.
        let seed = first_gml_id(&element, &self.gml_ns)
            .unwrap_or_else(|| format!("{}_{}", element.name.local, report.features + 1));
        let mut ids = IdGen::new(&seed);

        // Module-independent rewrites run first: they introduce elements the
        // building pass must not mistake for building properties (a
        // `core:genericAttribute` wrapper is not a bldg property), and never
        // the other way round.
        self.common.apply(&mut element, &mut report.warnings);
        // The appearance rewrite touches only appearance-namespace content,
        // which no later pass reads, so its slot is free.
        self.app.apply(&mut element, &mut ids, &mut report.warnings);
        // LOD4 is folded before the building pass so that pass only ever sees
        // LOD0-3 names and can never emit an LOD4 slot 3.0 does not have.
        self.lod4.apply(&mut element, &mut report.warnings);
        self.bldg
            .apply(&mut element, &mut ids, &mut report.warnings);
        // Last of the three: the CityGML hooks it introduces are in thematic
        // namespaces (`bldg:adeOfAbstractBuilding`), so running it earlier would
        // offer the building pass a property that is not a building property.
        self.iur.apply(&mut element, &mut report.warnings);
        if self.options.generate_gml_ids {
            transform::assign_gml_ids(&mut element, &self.gml_ns, &mut ids);
        }
        if self.options.reorder {
            transform::reorder(&self.rules, &mut element);
        }

        // `gml:boundedBy` and friends are properties of the CityModel, not city
        // objects, so they must not inflate the feature count.
        if !element.name.in_ns(&self.gml_ns) {
            report.features += 1;
        }
        Some(element)
    }
}

/// The `gml:id` of `element` or of its first descendant that has one.
fn first_gml_id(element: &Element, gml_ns: &str) -> Option<String> {
    if let Some(id) = element.attr(Some(gml_ns), "id") {
        return Some(id.to_owned());
    }
    element
        .elements()
        .find_map(|child| first_gml_id(child, gml_ns))
}

/// Writes the output's `codelists/`.
///
/// The published i-UR 4.0 lists are what a converted package's codes are
/// checked against, so they replace the input's copies of the same files.
/// Input lists with no published counterpart, and the municipality-authored
/// ones the profile's `[codelists] local` patterns name, are copied verbatim
/// — a published file never overwrites those.
fn write_codelists(
    root: &Path,
    out: &Path,
    rules: &Rules,
    warnings: &mut Warnings,
) -> Result<usize> {
    let published: std::collections::HashMap<&str, &str> =
        crate::CODELISTS_4_0.iter().copied().collect();
    let target_dir = out.join("codelists");
    fs::create_dir_all(&target_dir).map_err(|e| Error::io(&target_dir, e))?;

    let mut written = 0;
    let mut kept = 0;
    let source = root.join("codelists");
    if source.is_dir() {
        for entry in fs::read_dir(&source).map_err(|e| Error::io(&source, e))? {
            let entry = entry.map_err(|e| Error::io(&source, e))?;
            if !entry.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if rules.codelists().is_local(name) || !published.contains_key(name) {
                let target = target_dir.join(name);
                fs::copy(entry.path(), &target).map_err(|e| Error::io(&target, e))?;
                kept += 1;
                written += 1;
            }
        }
    }

    for (name, text) in crate::CODELISTS_4_0 {
        let target = target_dir.join(name);
        if target.exists() {
            continue; // a kept input list takes precedence over the published name
        }
        fs::write(&target, text).map_err(|e| Error::io(&target, e))?;
        written += 1;
    }

    if kept > 0 {
        warnings.add(format!(
            "codelists/ was rebuilt from the published i-UR 4.0 lists; {kept} \
             municipality-authored or unpublished input lists were kept as shipped"
        ));
    } else {
        warnings.add(
            "codelists/ was rebuilt from the published i-UR 4.0 lists, replacing \
             the input's copies of the same files",
        );
    }
    Ok(written)
}

/// Writes the vendored i-UR 4.0 schemas into `out/schemas/`.
///
/// The output's `xsi:schemaLocation` points at these by relative path, so they
/// have to be there for the package to resolve on its own.
fn write_iur_schemas(out: &Path) -> Result<usize> {
    let mut written = 0;
    for (relative, text) in crate::IUR_4_0_SCHEMAS {
        let target = out.join("schemas").join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        fs::write(&target, text).map_err(|e| Error::io(&target, e))?;
        written += 1;
    }
    Ok(written)
}

fn copy_tree(from: &Path, to: &Path) -> Result<usize> {
    let mut copied = 0;
    for entry in walkdir::WalkDir::new(from)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let Ok(relative) = entry.path().strip_prefix(from) else {
            continue;
        };
        let target = to.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|e| Error::io(&target, e))?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            fs::copy(entry.path(), &target).map_err(|e| Error::io(&target, e))?;
            copied += 1;
        }
    }
    Ok(copied)
}

/// Convenience for callers that only want the converted text.
pub fn convert_to_string(
    converter: &Converter,
    label: &str,
    source: &str,
) -> Result<(String, FileReport)> {
    let mut out: Vec<u8> = Vec::new();
    let report = converter.convert(label, source, &mut out)?;
    let text = String::from_utf8(out)
        .map_err(|e| Error::RawIo(io::Error::new(io::ErrorKind::InvalidData, e)))?;
    Ok((text, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_PROFILE;
    use crate::profile::Rules;

    /// A self-closing root is legal XML; the output must still close the root
    /// tag it wrote, and the empty document is reported.
    #[test]
    fn a_self_closing_root_produces_a_closed_root() {
        let converter = Converter::new(
            Rules::from_toml(DEFAULT_PROFILE).unwrap(),
            Options::default(),
        )
        .unwrap();
        let src = r#"<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"/>"#;
        let (out, report) = convert_to_string(&converter, "t", src).unwrap();
        assert!(out.contains("</core:CityModel>"), "{out}");
        assert!(
            report
                .warnings
                .iter()
                .any(|(m, _)| m.contains("no city objects")),
            "an empty document is still reported"
        );
    }
}
