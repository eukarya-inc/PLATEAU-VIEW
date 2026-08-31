//! Resolving whatever the user handed us into a single PLATEAU directory tree.
//!
//! PLATEAU data arrives in two shapes:
//!
//! 1. one package holding `udx/`, `codelists/`, `schemas/` and friends — as a
//!    directory or as a zip of one;
//! 2. a loose set of zips, one per part, as the portal serves them.
//!
//! The second shape has to be reassembled before anything can resolve a
//! `codeSpace` or an `xsi:schemaLocation`, so those inputs are extracted into a
//! staging directory laid out like case 1. A directory that is already a package
//! is used where it lies — nothing is copied.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use walkdir::WalkDir;

use crate::error::{Error, Result};

/// The top-level directories of a PLATEAU 3D city model package.
pub const PARTS: &[&str] = &["udx", "codelists", "schemas", "metadata", "specification"];

/// The part every dataset must have for there to be anything to convert.
const REQUIRED_PART: &str = "udx";

/// Where a reassembled dataset is put together.
#[derive(Debug, Clone, Default)]
pub enum Staging {
    /// A temporary directory, removed when the [`Dataset`] is dropped.
    #[default]
    Temporary,
    /// A caller-chosen directory. Created if missing and left in place.
    At(PathBuf),
}

/// A PLATEAU package rooted at a single directory.
#[derive(Debug)]
pub struct Dataset {
    root: PathBuf,
    /// Held only to keep a temporary staging directory alive.
    temp: Option<TempDir>,
}

impl Dataset {
    /// Resolves `inputs` into one tree, staging into a temporary directory if
    /// they need reassembling.
    pub fn open(inputs: &[PathBuf]) -> Result<Self> {
        Self::open_with(inputs, &Staging::Temporary)
    }

    pub fn open_with(inputs: &[PathBuf], staging: &Staging) -> Result<Self> {
        if inputs.is_empty() {
            return Err(Error::Layout("no input given".into()));
        }

        let sources: Vec<Source> = inputs
            .iter()
            .map(|p| Source::classify(p))
            .collect::<Result<_>>()?;

        // The common case: a single directory that is already a package. Use it
        // in place rather than copying a few gigabytes for nothing.
        if let [
            Source::Directory {
                path,
                layout: Layout::Bundle { prefix, .. },
            },
        ] = &sources[..]
        {
            let root = join_prefix(path, prefix);
            check_root(&root)?;
            return Ok(Dataset { root, temp: None });
        }

        let (root, temp) = match staging {
            Staging::Temporary => {
                let dir = TempDir::with_prefix("plateau-convert-")?;
                (dir.path().to_owned(), Some(dir))
            }
            Staging::At(path) => {
                fs::create_dir_all(path).map_err(|e| Error::io(path, e))?;
                (path.clone(), None)
            }
        };

        for source in &sources {
            source.stage(&root)?;
        }
        check_root(&root)?;

        Ok(Dataset { root, temp })
    }

    /// The package root: the directory holding `udx/`, `codelists/` and so on.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// True when the tree was assembled rather than used where it lay.
    pub fn is_staged(&self) -> bool {
        self.temp.is_some()
    }

    pub fn udx(&self) -> PathBuf {
        self.root.join(REQUIRED_PART)
    }

    /// Which of [`PARTS`] this package actually has.
    pub fn parts(&self) -> Vec<&'static str> {
        present_parts(&self.root)
    }

    /// The feature-type directories under `udx/` (`bldg`, `tran`, ...), sorted.
    pub fn feature_types(&self) -> Result<Vec<String>> {
        let udx = self.udx();
        let mut types: Vec<String> = read_dir(&udx)?
            .filter(|e| e.is_dir())
            .filter_map(|e| e.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        types.sort();
        Ok(types)
    }

    /// The `.gml` files of one feature type, sorted so runs are reproducible.
    pub fn gml_files(&self, feature_type: &str) -> Result<Vec<PathBuf>> {
        self.feature_files(feature_type, true)
    }

    /// The files of one feature type that are *not* CityGML — texture images
    /// above all, which the documents reference by relative path — sorted.
    pub fn companion_files(&self, feature_type: &str) -> Result<Vec<PathBuf>> {
        self.feature_files(feature_type, false)
    }

    fn feature_files(&self, feature_type: &str, gml: bool) -> Result<Vec<PathBuf>> {
        let dir = self.udx().join(feature_type);
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut files: Vec<PathBuf> = WalkDir::new(&dir)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .filter(|p| has_extension(p, "gml") == gml)
            .collect();
        files.sort();
        Ok(files)
    }

    /// Stops the staging directory from being removed, returning its path.
    ///
    /// Useful for inspecting what the reassembly produced.
    pub fn keep(mut self) -> PathBuf {
        if let Some(temp) = self.temp.take() {
            let _ = temp.keep();
        }
        self.root
    }
}

fn check_root(root: &Path) -> Result<()> {
    if root.join(REQUIRED_PART).is_dir() {
        return Ok(());
    }
    Err(Error::Layout(format!(
        "{}: no `{REQUIRED_PART}` directory — inputs must add up to a PLATEAU package \
         (udx, codelists, schemas)",
        root.display()
    )))
}

/// What an input turned out to contain.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Layout {
    /// PLATEAU part directories live under `prefix` (empty for the top level).
    Bundle {
        prefix: String,
        parts: Vec<&'static str>,
    },
    /// The input *is* one part's contents, with no directory of its own.
    LoosePart { part: &'static str },
}

#[derive(Debug)]
enum Source {
    Directory { path: PathBuf, layout: Layout },
    Archive { path: PathBuf, layout: Layout },
}

impl Source {
    fn classify(path: &Path) -> Result<Source> {
        let meta = fs::metadata(path).map_err(|e| Error::io(path, e))?;
        if meta.is_dir() {
            let layout = dir_layout(path)?;
            Ok(Source::Directory {
                path: path.to_owned(),
                layout,
            })
        } else if has_extension(path, "zip") {
            let layout = zip_layout(path)?;
            Ok(Source::Archive {
                path: path.to_owned(),
                layout,
            })
        } else {
            Err(Error::Layout(format!(
                "{}: expected a directory or a .zip",
                path.display()
            )))
        }
    }

    /// Copies or extracts this input into the staging tree.
    fn stage(&self, root: &Path) -> Result<()> {
        match self {
            Source::Directory { path, layout } => match layout {
                Layout::Bundle { prefix, .. } => copy_tree(&join_prefix(path, prefix), root),
                Layout::LoosePart { part } => copy_tree(path, &root.join(part)),
            },
            Source::Archive { path, layout } => match layout {
                Layout::Bundle { prefix, .. } => extract(path, prefix, root),
                Layout::LoosePart { part } => extract(path, "", &root.join(part)),
            },
        }
    }
}

// --- classification ---------------------------------------------------------

fn dir_layout(path: &Path) -> Result<Layout> {
    // A package the user pointed straight at.
    let parts = present_parts(path);
    if !parts.is_empty() {
        return Ok(Layout::Bundle {
            prefix: String::new(),
            parts,
        });
    }

    // A package one level down — what unzipping a portal download leaves behind.
    for entry in read_dir(path)?.filter(|e| e.is_dir()) {
        let parts = present_parts(&entry);
        if !parts.is_empty() {
            let prefix = entry
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            return Ok(Layout::Bundle { prefix, parts });
        }
    }

    // Otherwise the directory must itself be one part.
    let part = part_by_name(path)
        .or_else(|| infer_part(dir_entries(path).iter().map(String::as_str)))
        .ok_or_else(|| {
            Error::Layout(format!(
                "{}: not a PLATEAU package and its contents do not look like \
                 udx, codelists or schemas",
                path.display()
            ))
        })?;
    Ok(Layout::LoosePart { part })
}

fn zip_layout(path: &Path) -> Result<Layout> {
    let entries = zip_entries(path)?;

    let found = find_parts(entries.iter().map(String::as_str));
    if !found.is_empty() {
        // Several prefixes can match when a zip nests packages; the outermost
        // one is the package root.
        let prefix = found
            .values()
            .min_by_key(|p| (depth(p), p.len()))
            .cloned()
            .unwrap_or_default();
        let parts = found
            .iter()
            .filter(|(_, p)| **p == prefix)
            .map(|(part, _)| *part)
            .collect();
        return Ok(Layout::Bundle { prefix, parts });
    }

    let part = part_by_name(path)
        .or_else(|| infer_part(entries.iter().map(String::as_str)))
        .ok_or_else(|| {
            Error::Layout(format!(
                "{}: holds no udx/codelists/schemas directory and its contents do \
                 not identify one",
                path.display()
            ))
        })?;
    Ok(Layout::LoosePart { part })
}

/// Locates part directories in a list of relative entry paths, keeping the
/// outermost occurrence of each.
fn find_parts<'a>(entries: impl Iterator<Item = &'a str>) -> BTreeMap<&'static str, String> {
    let mut found: BTreeMap<&'static str, String> = BTreeMap::new();
    for entry in entries {
        let is_dir_entry = entry.ends_with('/');
        let components: Vec<&str> = entry.split('/').filter(|c| !c.is_empty()).collect();
        for (i, component) in components.iter().enumerate() {
            let last = i + 1 == components.len();
            if last && !is_dir_entry {
                break; // a file, not a directory
            }
            let Some(part) = PARTS.iter().find(|p| *p == component) else {
                continue;
            };
            let prefix = components[..i].join("/");
            match found.get(part) {
                Some(existing) if depth(existing) <= depth(&prefix) => {}
                _ => {
                    found.insert(part, prefix);
                }
            }
        }
    }
    found
}

/// A part name taken from the input's own file or directory name, so that
/// `.../udx` and `..._udx.zip` are recognised without looking inside.
fn part_by_name(path: &Path) -> Option<&'static str> {
    let name = path.file_stem()?.to_str()?.to_ascii_lowercase();
    PARTS
        .iter()
        .copied()
        .find(|part| name == *part || name.ends_with(&format!("_{part}")))
}

/// Guesses which part a set of entries belongs to from the file types present.
fn infer_part<'a>(entries: impl Iterator<Item = &'a str>) -> Option<&'static str> {
    let mut gml = false;
    let mut xsd = false;
    let mut xml = false;
    for entry in entries {
        let lower = entry.to_ascii_lowercase();
        gml |= lower.ends_with(".gml");
        xsd |= lower.ends_with(".xsd");
        xml |= lower.ends_with(".xml");
    }
    // Order matters: schemas ship .xsd alongside other files, and codelists are
    // the only part that is nothing but .xml.
    match (gml, xsd, xml) {
        (true, _, _) => Some("udx"),
        (_, true, _) => Some("schemas"),
        (_, _, true) => Some("codelists"),
        _ => None,
    }
}

fn present_parts(dir: &Path) -> Vec<&'static str> {
    PARTS
        .iter()
        .copied()
        .filter(|part| dir.join(part).is_dir())
        .collect()
}

// --- filesystem helpers -----------------------------------------------------

fn read_dir(path: &Path) -> Result<impl Iterator<Item = PathBuf>> {
    let entries = fs::read_dir(path).map_err(|e| Error::io(path, e))?;
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    Ok(paths.into_iter())
}

/// Relative paths under `dir`, shallow enough to identify it but not to walk a
/// whole city.
fn dir_entries(dir: &Path) -> Vec<String> {
    WalkDir::new(dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let rel = e.path().strip_prefix(dir).ok()?;
            let mut s = rel.to_string_lossy().replace('\\', "/");
            if e.file_type().is_dir() {
                s.push('/');
            }
            Some(s)
        })
        .collect()
}

fn zip_entries(path: &Path) -> Result<Vec<String>> {
    let file = File::open(path).map_err(|e| Error::io(path, e))?;
    let archive = zip::ZipArchive::new(file).map_err(|e| Error::zip(path, e))?;
    // Windows-made archives separate with backslashes; real PLATEAU part zips
    // do. Normalise here so layout detection sees the same paths `extract`
    // (which normalises on its own) will write.
    Ok(archive
        .file_names()
        .map(|name| name.replace('\\', "/"))
        .collect())
}

fn extract(archive_path: &Path, strip: &str, dest: &Path) -> Result<()> {
    let file = File::open(archive_path).map_err(|e| Error::io(archive_path, e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| Error::zip(archive_path, e))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| Error::zip(archive_path, e))?;
        // `enclosed_name` rejects absolute paths and `..`, so a malicious archive
        // cannot write outside `dest`.
        let Some(name) = entry.enclosed_name() else {
            tracing::warn!(entry = entry.name(), "skipping unsafe zip entry");
            continue;
        };
        let relative = name.to_string_lossy().replace('\\', "/");
        let Some(relative) = strip_prefix(&relative, strip) else {
            continue;
        };
        if relative.is_empty() {
            continue;
        }

        let target = dest.join(&relative);
        if entry.is_dir() {
            fs::create_dir_all(&target).map_err(|e| Error::io(&target, e))?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let mut out = File::create(&target).map_err(|e| Error::io(&target, e))?;
        io::copy(&mut entry, &mut out).map_err(|e| Error::io(&target, e))?;
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    for entry in WalkDir::new(from).into_iter().filter_map(|e| e.ok()) {
        let relative = match entry.path().strip_prefix(from) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let target = to.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|e| Error::io(&target, e))?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            fs::copy(entry.path(), &target).map_err(|e| Error::io(&target, e))?;
        }
    }
    Ok(())
}

fn join_prefix(base: &Path, prefix: &str) -> PathBuf {
    let mut path = base.to_owned();
    for component in prefix.split('/').filter(|c| !c.is_empty()) {
        path.push(component);
    }
    path
}

/// Removes `prefix` from a `/`-separated relative path, or returns `None` when
/// the path is not under it.
fn strip_prefix(path: &str, prefix: &str) -> Option<String> {
    if prefix.is_empty() {
        return Some(path.to_owned());
    }
    path.strip_prefix(prefix)
        // The remainder must start at a component boundary, so `rootx/udx` does
        // not match the prefix `root`.
        .and_then(|rest| {
            rest.strip_prefix('/')
                .or_else(|| rest.is_empty().then_some(""))
        })
        .map(str::to_owned)
}

fn depth(prefix: &str) -> usize {
    prefix.split('/').filter(|c| !c.is_empty()).count()
}

fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(entries: &[&str]) -> Vec<String> {
        entries.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn finds_the_package_root_inside_a_zip() {
        let entries = names(&[
            "22100_shizuoka-shi_city_2023_citygml_1_op/",
            "22100_shizuoka-shi_city_2023_citygml_1_op/udx/",
            "22100_shizuoka-shi_city_2023_citygml_1_op/udx/bldg/x.gml",
            "22100_shizuoka-shi_city_2023_citygml_1_op/codelists/a.xml",
        ]);
        let found = find_parts(entries.iter().map(String::as_str));
        assert_eq!(
            found.get("udx").map(String::as_str),
            Some("22100_shizuoka-shi_city_2023_citygml_1_op")
        );
        assert_eq!(
            found.len(),
            2,
            "codelists is found from a file entry too: {found:?}"
        );
    }

    #[test]
    fn a_part_zip_has_no_package_root() {
        let entries = names(&["bldg/", "bldg/52382287_bldg_6697_op.gml"]);
        assert!(find_parts(entries.iter().map(String::as_str)).is_empty());
        assert_eq!(infer_part(entries.iter().map(String::as_str)), Some("udx"));
    }

    #[test]
    fn infers_parts_from_file_types() {
        assert_eq!(infer_part(["a/b.gml"].into_iter()), Some("udx"));
        assert_eq!(
            infer_part(["iur/uro/3.0/urbanObject.xsd"].into_iter()),
            Some("schemas")
        );
        assert_eq!(
            infer_part(["Common_urbanPlanType.xml"].into_iter()),
            Some("codelists")
        );
        assert_eq!(infer_part(["readme.txt"].into_iter()), None);
    }

    #[test]
    fn recognises_a_part_from_the_input_name() {
        assert_eq!(part_by_name(Path::new("/data/udx")), Some("udx"));
        assert_eq!(
            part_by_name(Path::new(
                "22100_shizuoka-shi_city_2023_citygml_1_op_codelists.zip"
            )),
            Some("codelists")
        );
        assert_eq!(part_by_name(Path::new("/data/something")), None);
    }

    #[test]
    fn strips_zip_prefixes() {
        assert_eq!(
            strip_prefix("root/udx/x.gml", "root"),
            Some("udx/x.gml".into())
        );
        assert_eq!(strip_prefix("root", "root"), Some(String::new()));
        assert_eq!(strip_prefix("other/udx", "root"), None);
        assert_eq!(strip_prefix("udx/x.gml", ""), Some("udx/x.gml".into()));
        // A sibling with a name that merely starts the same must not match.
        assert_eq!(strip_prefix("rootx/udx", "root"), None);
    }
}
