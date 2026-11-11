//! The top-level driver, taking a dataset in and writing a converted tree out.

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::app::AppearanceRewrite;
use crate::bldg::BuildingRewrite;
use crate::common::CommonRewrite;
use crate::cons::ConsRewrite;
use crate::construction::ConstructionRewrite;
use crate::dataset::Dataset;
use crate::dropped::Dropped;
use crate::error::{Error, Result};
use crate::iur::IurRewrite;
use crate::lod4::Lod4Rewrite;
use crate::lodmap::LodMapRewrite;
use crate::profile::{Lod4Fallback, Rules, UnfDm};
use crate::report::{FileReport, Report, Warnings};
use crate::schema::ChildOrder;
use crate::storey::{Floors, StoreyRewrite};
use crate::tran::{Clearance, TranRewrite};
use crate::transform::{self, IdGen};
use crate::unf::UnfRewrite;
use crate::urf::UrfRewrite;
use crate::wtr::WtrRewrite;
use crate::xal::XalRewrite;
use crate::xml::{self, Chunk, Element, Indent, Name, Node, Reader, Writer};

/// Directories copied through unchanged when converting a whole dataset.
///
/// `schemas/` and `codelists/` are absent. They are rebuilt by
/// [`write_iur_schemas`] and [`write_codelists`].
const COPIED_PARTS: &[&str] = &["metadata", "specification"];

#[derive(Debug, Clone)]
pub struct Options {
    /// `udx` subdirectories to convert. Empty means every one present.
    pub feature_types: Vec<String>,
    /// Mint `gml:id` for geometries that lack one, which GML 3.2 requires.
    pub generate_gml_ids: bool,
    /// Sort children into the order the target schemas declare.
    pub reorder: bool,
    pub indent: Indent,
    /// Copy `codelists/`, `schemas/` and friends alongside the converted `udx/`.
    pub copy_support_files: bool,
    /// Convert files concurrently.
    pub parallel: bool,
    /// Where LOD4 goes when the data does not say. `None` takes the profile's
    /// `[lod4] fallback`.
    pub lod4_fallback: Option<Lod4Fallback>,
    /// Extrude transportation LOD1 spaces into solids. `None` writes no solid.
    pub clearance: Option<Clearance>,
    /// What happens to an underground utility's Dm attributes. `None` takes
    /// the profile's `[unf] dm`.
    pub unf_dm: Option<UnfDm>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            feature_types: Vec::new(),
            generate_gml_ids: true,
            reorder: true,
            indent: Indent::Tab,
            copy_support_files: true,
            parallel: true,
            lod4_fallback: None,
            clearance: None,
            unf_dm: None,
        }
    }
}

pub struct Converter {
    rules: Rules,
    storey: StoreyRewrite,
    common: CommonRewrite,
    xal: XalRewrite,
    app: AppearanceRewrite,
    unf: UnfRewrite,
    lodmap: LodMapRewrite,
    lod4: Lod4Rewrite,
    construction: ConstructionRewrite,
    bldg: BuildingRewrite,
    cons: ConsRewrite,
    tran: TranRewrite,
    urf: UrfRewrite,
    wtr: WtrRewrite,
    iur: IurRewrite,
    order: ChildOrder,
    gml_ns: String,
    xlink_ns: String,
    core_ns: String,
    options: Options,
}

/// What one document's conversion carries from member to member.
#[derive(Debug, Default)]
struct FileState {
    /// The geometry the passes removed, including that removed from members
    /// after the first appearance member before any is written.
    dropped: Dropped,
    /// Floors read ahead of conversion, waiting for their building.
    floors: Floors,
}

impl Converter {
    pub fn new(rules: Rules, options: Options) -> Result<Self> {
        let storey = StoreyRewrite::new(&rules)?;
        let common = CommonRewrite::new(&rules)?;
        let xal = XalRewrite::new(&rules)?;
        let app = AppearanceRewrite::new(&rules)?;
        let schema = ChildOrder::target_with(rules.schema_overrides())?;
        let urf = UrfRewrite::new(&rules)?;
        let unf = UnfRewrite::new(&rules, &schema, options.unf_dm)?;
        let lodmap = LodMapRewrite::new(&rules)?;
        let lod4 = Lod4Rewrite::new(&rules, options.lod4_fallback)?;
        let construction = ConstructionRewrite::new(&rules)?;
        let bldg = BuildingRewrite::new(&rules)?;
        let cons = ConsRewrite::new(&rules, &schema)?;
        let tran = TranRewrite::new(&rules, options.clearance.clone())?;
        let wtr = WtrRewrite::new(&rules)?;
        let iur = IurRewrite::new(&rules, &schema)?;
        let order = if options.reorder {
            schema
        } else {
            ChildOrder::default()
        };
        let gml_ns = rules.output_ns("gml")?.to_owned();
        let core_ns = rules.output_ns("core")?.to_owned();
        let xlink_ns = rules.output_ns("xlink")?.to_owned();
        Ok(Converter {
            rules,
            storey,
            common,
            xal,
            app,
            unf,
            lodmap,
            lod4,
            construction,
            bldg,
            cons,
            tran,
            urf,
            wtr,
            iur,
            order,
            gml_ns,
            xlink_ns,
            core_ns,
            options,
        })
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Converts the selected feature types of `dataset` into `out`, mirroring
    /// the input's directory layout. Non-GML companion files are copied to the
    /// mirrored path, and a feature type present but not requested is
    /// reported.
    pub fn convert_dataset(&self, dataset: &Dataset, out: &Path) -> Result<Report> {
        let requested = if self.options.feature_types.is_empty() {
            dataset.feature_types()?
        } else {
            self.options.feature_types.clone()
        };

        let mut jobs: Vec<(PathBuf, PathBuf, String)> = Vec::new();
        for feature_type in &requested {
            let files = dataset.gml_files(feature_type)?;
            if files.is_empty() {
                tracing::warn!(feature_type, "no .gml files found");
            }
            for input in files {
                let relative = input.strip_prefix(dataset.root()).map_err(|_| {
                    Error::Layout(format!("{} is outside the dataset root", input.display()))
                })?;
                jobs.push((input.clone(), out.join(relative), to_root(relative)));
            }
        }

        let results: Vec<Result<FileReport>> = if self.options.parallel {
            jobs.par_iter()
                .map(|(i, o, r)| self.convert_file(i, o, r))
                .collect()
        } else {
            jobs.iter()
                .map(|(i, o, r)| self.convert_file(i, o, r))
                .collect()
        };

        let mut report = Report::default();
        for result in results {
            report.absorb(&result?);
        }
        self.tran.report_unused_clearance(&mut report.warnings);

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

            let target_dir = out.join("codelists");
            for name in &report.code_spaces {
                if !target_dir.join(name).is_file() {
                    report.warnings.add(format!(
                        "codeSpace references codelists/{name}, which is not in the \
                         output package: the input did not ship it and the published \
                         i-UR 4.0 set does not carry it"
                    ));
                }
            }
        }

        Ok(report)
    }

    /// Converts the document at `input` into `output`. `to_root` is the path
    /// from the document back to its package root, such as `../../`.
    pub fn convert_file(&self, input: &Path, output: &Path, to_root: &str) -> Result<FileReport> {
        let source = xml::read_to_string(input)?;
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let file = File::create(output).map_err(|e| Error::io(output, e))?;
        let mut sink = BufWriter::new(file);
        let label = input.file_name().map(|n| n.to_string_lossy().into_owned());
        let label = label.as_deref().unwrap_or("input");
        let report = self.convert(label, to_root, &source, &mut sink)?;
        sink.flush().map_err(|e| Error::io(output, e))?;
        tracing::debug!(input = %input.display(), features = report.features, "converted");
        Ok(report)
    }

    /// Converts one CityGML document into `sink`. `label` names the input in
    /// diagnostics, and `to_root` is the path from the document back to its
    /// package root, such as `../../`, which every path the profile gives from
    /// the package root is written with. A document whose root element never
    /// closes is a [`Error::MalformedXml`].
    pub fn convert(
        &self,
        label: &str,
        to_root: &str,
        source: &str,
        sink: impl Write,
    ) -> Result<FileReport> {
        let mut reader = Reader::new(label, source);
        let mut writer = Writer::new(sink, self.rules.prefixes().clone(), self.options.indent);
        let mut report = FileReport::default();
        let mut root: Option<Element> = None;
        let mut root_input: Option<Name> = None;
        let mut closed = false;
        let floors = self.storey.scan(label, source, &mut report.warnings)?;
        let mut state = FileState {
            dropped: self.dropped_after_appearance(label, source, &floors)?,
            floors,
        };

        writer.write_declaration(None)?;

        while let Some(chunk) = reader.next_chunk()? {
            match chunk {
                Chunk::Decl => {}
                Chunk::Prologue(Node::Comment(text)) => writer.write_comment(&text)?,
                Chunk::Prologue(_) => {}
                Chunk::RootStart(element) => {
                    root_input = Some(element.name.clone());
                    let element = transform::rename(&self.rules, element).ok_or_else(|| {
                        Error::Profile(format!("the profile drops the root element of {label}"))
                    })?;
                    let location = self.rules.schema_location(to_root);
                    writer.write_root_start(&element, location.as_deref())?;
                    root = Some(element);
                }
                Chunk::Member(element) => {
                    if root.is_none() {
                        return Err(Error::malformed(label, "content before the root element"));
                    }
                    if let Some(element) = self.convert_member(
                        element,
                        root_input.as_ref(),
                        to_root,
                        &mut report,
                        &mut state,
                    ) {
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
                    closed = true;
                }
            }
        }

        let root = root.ok_or_else(|| Error::malformed(label, "no root element"))?;
        if !closed {
            return Err(Error::malformed(
                label,
                format!(
                    "the document ends inside <{}> without closing it; it is truncated",
                    self.rules.display_name(&root.name)
                ),
            ));
        }
        if !state.floors.is_settled() {
            report.warnings.add(
                "a floor's building was not written, so the bldg:Storey made from its \
                 grp:CityObjectGroup was dropped with it",
            );
        }
        if root.children.is_empty() && report.features == 0 {
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
    ///
    /// The restructuring passes run in a fixed order, namely: `urf`, `storey`,
    /// `common`, `xal`, `app`, `unf`, `lodmap`, `lod4`, `construction`, `bldg`, `cons`,
    /// `tran`, `wtr`, `iur`. Generated `gml:id` values
    /// are seeded from the member's own `gml:id`, so they are unique across a
    /// dataset and stable across runs. Appearance links to geometry dropped
    /// from any member of the same document are removed, and a member left
    /// with no content is `None`. `root` is the input name of the document's
    /// root element, and `to_root` the path from the document back to its
    /// package root.
    fn convert_member(
        &self,
        element: Element,
        root: Option<&Name>,
        to_root: &str,
        report: &mut FileReport,
        state: &mut FileState,
    ) -> Option<Element> {
        let mut element = transform::rename_under(&self.rules, element, root)?;
        if self.urf.apply(&mut element, &mut report.warnings) {
            return None;
        }
        if self.storey.is_floor(&element, &state.floors) {
            return None;
        }
        self.storey
            .attach(&mut element, &mut state.floors, &mut report.warnings);

        let seed = first_gml_id(&element, &self.gml_ns)
            .unwrap_or_else(|| format!("{}_{}", element.name.local, report.features + 1));
        let mut ids = IdGen::new(&seed);

        self.tran.raise_envelope(&mut element, &mut report.warnings);
        self.common.apply(&mut element, &mut report.warnings);
        self.xal.apply(&mut element, &mut report.warnings);
        self.app.apply(&mut element, &mut ids, &mut report.warnings);
        if self.remove_geometry(&mut element, &mut state.dropped, &mut report.warnings) {
            self.storey.warn_missing(&element, &mut report.warnings);
        }
        if !state.dropped.is_empty()
            && self
                .app
                .prune(&mut element, state.dropped.ids(), &mut report.warnings)
        {
            return None;
        }
        self.bldg
            .apply(&mut element, to_root, &mut ids, &mut report.warnings);
        self.cons.apply(&mut element, &mut report.warnings);
        self.tran
            .apply(&mut element, to_root, &mut ids, &mut report.warnings);
        self.wtr.apply(&mut element, &mut ids);
        self.iur.apply(&mut element, to_root, &mut report.warnings);
        if self.options.generate_gml_ids {
            transform::assign_gml_ids(&mut element, &self.gml_ns, &mut ids);
        }
        if self.options.reorder {
            transform::reorder(&self.order, &mut element);
        }

        if !element.name.in_ns(&self.gml_ns) {
            report.features += 1;
        }
        collect_code_spaces(&element, &mut report.code_spaces);
        Some(element)
    }

    /// The geometry [`Self::remove_geometry`] removes from the members of
    /// `source` that follow its first appearance member, skipping the floor
    /// groups in `floors`. Empty when `source` has no appearance member.
    fn dropped_after_appearance(
        &self,
        label: &str,
        source: &str,
        floors: &Floors,
    ) -> Result<Dropped> {
        let mut dropped = Dropped::new(&self.gml_ns, &self.xlink_ns);
        if !source.contains("appearanceMember") {
            return Ok(dropped);
        }
        let mut reader = Reader::new(label, source);
        let mut root: Option<Name> = None;
        let mut seen = false;
        let mut warnings = Warnings::new();
        while let Some(chunk) = reader.next_chunk()? {
            match chunk {
                Chunk::RootStart(element) => root = Some(element.name),
                Chunk::Member(element) => {
                    let Some(mut element) =
                        transform::rename_under(&self.rules, element, root.as_ref())
                    else {
                        continue;
                    };
                    if element.is(&self.core_ns, "appearanceMember") {
                        seen = true;
                        continue;
                    }
                    if !seen || self.storey.is_floor(&element, floors) {
                        continue;
                    }
                    self.remove_geometry(&mut element, &mut dropped, &mut warnings);
                }
                _ => {}
            }
        }
        Ok(dropped)
    }

    /// Runs the `unf`, `lodmap`, `lod4` and `construction` passes on
    /// `element`, recording what they remove in `dropped`, then writes each
    /// removed object kept geometry still links to back in place of the link.
    /// Returns true when `lod4` folded an interior model to an LOD.
    fn remove_geometry(
        &self,
        element: &mut Element,
        dropped: &mut Dropped,
        warnings: &mut Warnings,
    ) -> bool {
        dropped.begin(element);
        self.unf.apply(element, dropped, warnings);
        self.lodmap.apply(element, dropped, warnings);
        let interior = self.lod4.apply(element, dropped, warnings);
        self.construction.apply(element, dropped, warnings);
        dropped.restore(element, warnings);
        interior
    }
}

/// Records the file names of every code list `element` references through a
/// relative `codelists/` path.
fn collect_code_spaces(element: &Element, out: &mut std::collections::BTreeSet<String>) {
    if let Some(value) = element.attr(None, "codeSpace") {
        if let Some((dir, file)) = value.rsplit_once('/') {
            if dir.ends_with("codelists") && !file.is_empty() {
                out.insert(file.to_owned());
            }
        }
    }
    for child in element.elements() {
        collect_code_spaces(child, out);
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
/// The published i-UR 4.0 lists replace the input's copies of the same files.
/// Two kinds of input list are kept instead, namely: one with no published
/// counterpart, and one the profile's `[codelists] local` patterns name. A
/// published file never overwrites those. Returns how many files were
/// written.
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
            let keep = if rules.codelists().is_local(name) {
                true
            } else if rules.codelists().is_superseded(name) {
                false
            } else {
                match published.get(name) {
                    None => true,
                    Some(text) => {
                        let input = fs::read_to_string(entry.path())
                            .map_err(|e| Error::io(entry.path(), e))?;
                        let extra: Vec<String> = definition_codes(&input)
                            .difference(&definition_codes(text))
                            .cloned()
                            .collect();
                        if extra.is_empty() {
                            false
                        } else {
                            warnings.add(format!(
                                "{name} defines {} code(s) the published i-UR 4.0 \
                                 list does not ({}); the input's copy was kept in \
                                 place of the published one",
                                extra.len(),
                                summarize(&extra),
                            ));
                            true
                        }
                    }
                }
            };
            if keep {
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

/// The codes a code list defines, meaning every `<gml:name>` inside a
/// `<gml:Definition>`. The dictionary's own `<gml:name>`, which is the list
/// title, sits outside any Definition and is excluded.
pub(crate) fn definition_codes(text: &str) -> std::collections::BTreeSet<String> {
    let mut codes = std::collections::BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find("<gml:Definition") {
        let block = &rest[start..];
        let end = block.find("</gml:Definition>").unwrap_or(block.len());
        let block = &block[..end];
        let mut inner = block;
        while let Some(open) = inner.find("<gml:name>") {
            let after = &inner[open + "<gml:name>".len()..];
            let Some(close) = after.find("</gml:name>") else {
                break;
            };
            codes.insert(after[..close].trim().to_owned());
            inner = &after[close..];
        }
        rest = &rest[start + end..];
    }
    codes
}

/// At most five codes, followed by an ellipsis when more were given.
fn summarize(codes: &[String]) -> String {
    let mut shown: Vec<&str> = codes.iter().take(5).map(String::as_str).collect();
    if codes.len() > 5 {
        shown.push("...");
    }
    shown.join(", ")
}

/// Writes the vendored i-UR 4.0 schemas into `out/schemas/`, where the
/// output's `xsi:schemaLocation` points at them by relative path. Returns how
/// many files were written.
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

/// The path back to the package root from a document directly under
/// `udx/<feature type>/`.
pub const FEATURE_TYPE_TO_ROOT: &str = "../../";

/// The path from the document at `relative`, a path from the package root,
/// back to that root.
pub fn to_root(relative: &Path) -> String {
    let depth = relative.components().count().saturating_sub(1);
    "../".repeat(depth)
}

/// Convenience for callers that only want the converted text, for a
/// document directly under `udx/<feature type>/`.
pub fn convert_to_string(
    converter: &Converter,
    label: &str,
    source: &str,
) -> Result<(String, FileReport)> {
    let mut out: Vec<u8> = Vec::new();
    let report = converter.convert(label, FEATURE_TYPE_TO_ROOT, source, &mut out)?;
    let text = String::from_utf8(out)
        .map_err(|e| Error::RawIo(io::Error::new(io::ErrorKind::InvalidData, e)))?;
    Ok((text, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_PROFILE;
    use crate::profile::Rules;

    /// A self-closing root is legal XML, and the output still closes the root
    /// tag it wrote. The empty document is reported.
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

    /// A file cut short mid-document is an error rather than output with an
    /// unclosed root element.
    #[test]
    fn a_truncated_document_is_an_error_not_an_unclosed_root() {
        let converter = Converter::new(
            Rules::from_toml(DEFAULT_PROFILE).unwrap(),
            Options::default(),
        )
        .unwrap();
        for src in [
            r#"<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0">"#,
            concat!(
                r#"<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0""#,
                r#" xmlns:bldg="http://www.opengis.net/citygml/building/2.0">"#,
                "<core:cityObjectMember><bldg:Building/></core:cityObjectMember>",
            ),
        ] {
            let err = convert_to_string(&converter, "cut", src)
                .expect_err("a document with no closing root tag must not convert");
            assert!(err.to_string().contains("truncated"), "{err} for {src:?}");
        }
    }

    use crate::dataset::Dataset;
    use std::path::PathBuf;

    const MINI: &str = r#"<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
        xmlns:bldg="http://www.opengis.net/citygml/building/2.0"
        xmlns:gml="http://www.opengis.net/gml">
        <core:cityObjectMember>
            <bldg:Building gml:id="b1">
                <bldg:usage codeSpace="../../codelists/Building_usage.xml">402</bldg:usage>
                <bldg:function codeSpace="../../codelists/KeyValuePairAttribute_key200.xml">1</bldg:function>
            </bldg:Building>
        </core:cityObjectMember>
    </core:CityModel>"#;

    /// Lays out `root/udx/bldg/x.gml` (+ optional codelists) and converts it.
    fn convert_mini(codelists: &[(&str, &str)]) -> (tempfile::TempDir, Report) {
        let root = tempfile::TempDir::new().unwrap();
        let bldg = root.path().join("udx/bldg");
        std::fs::create_dir_all(&bldg).unwrap();
        std::fs::write(bldg.join("x.gml"), MINI).unwrap();
        for (name, text) in codelists {
            let dir = root.path().join("codelists");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(name), text).unwrap();
        }
        let out = tempfile::TempDir::new().unwrap();
        let converter = Converter::new(
            Rules::from_toml(DEFAULT_PROFILE).unwrap(),
            Options::default(),
        )
        .unwrap();
        let dataset = Dataset::open(&[PathBuf::from(root.path())]).unwrap();
        let report = converter
            .convert_dataset(&dataset, &out.path().join("pkg"))
            .unwrap();
        (out, report)
    }

    /// A `codeSpace` naming a list that is neither shipped by the input nor
    /// part of the published set is reported.
    #[test]
    fn a_dangling_code_space_reference_is_reported() {
        let (_out, report) = convert_mini(&[]);
        assert!(
            report.warnings.iter().any(|(m, _)| {
                m.contains("codeSpace references codelists/KeyValuePairAttribute_key200.xml")
            }),
            "{}",
            report.warnings
        );
        assert!(
            !report
                .warnings
                .iter()
                .any(|(m, _)| m.contains("codelists/Building_usage.xml,")),
            "a reference the published set resolves is not reported: {}",
            report.warnings
        );
    }

    const EDITED_USAGE: &str = r#"<gml:Dictionary xmlns:gml="http://www.opengis.net/gml" gml:id="cl1">
        <gml:name>Building_usage</gml:name>
        <gml:dictionaryEntry>
            <gml:Definition gml:id="d1">
                <gml:description>a locally added class</gml:description>
                <gml:name>9001</gml:name>
            </gml:Definition>
        </gml:dictionaryEntry>
    </gml:Dictionary>"#;

    /// An input list defining codes the published file lacks is kept, and the
    /// published copy does not replace it.
    #[test]
    fn an_edited_input_list_survives_the_published_one() {
        let (out, report) = convert_mini(&[("Building_usage.xml", EDITED_USAGE)]);
        let written =
            std::fs::read_to_string(out.path().join("pkg/codelists/Building_usage.xml")).unwrap();
        assert!(
            written.contains("9001"),
            "the input's copy is the one written"
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|(m, _)| m.contains("Building_usage.xml defines 1 code(s)")),
            "{}",
            report.warnings
        );
    }

    /// A superseded list is replaced however many extra codes it defines.
    #[test]
    fn a_superseded_list_is_replaced_despite_extra_codes() {
        let edited = EDITED_USAGE.replace("Building_usage", "Building_lodType");
        let (out, report) = convert_mini(&[("Building_lodType.xml", &edited)]);
        let written =
            std::fs::read_to_string(out.path().join("pkg/codelists/Building_lodType.xml")).unwrap();
        assert!(
            !written.contains("9001"),
            "the published copy replaces the edited input"
        );
        assert!(
            !report
                .warnings
                .iter()
                .any(|(m, _)| m.contains("Building_lodType.xml defines")),
            "{}",
            report.warnings
        );
    }

    /// An input list whose codes are all in the published file is replaced by
    /// the published copy, with no warning.
    #[test]
    fn an_unedited_input_list_is_replaced_by_the_published_one() {
        let subset: String = EDITED_USAGE
            .replace("9001", "402")
            .replace("a locally added class", "商業施設");
        let (out, report) = convert_mini(&[("Building_usage.xml", &subset)]);
        let written =
            std::fs::read_to_string(out.path().join("pkg/codelists/Building_usage.xml")).unwrap();
        assert!(
            !written.contains("cl1"),
            "the published copy replaces the input's"
        );
        assert!(
            !report
                .warnings
                .iter()
                .any(|(m, _)| m.contains("defines 1 code(s)")),
            "{}",
            report.warnings
        );
    }
}
