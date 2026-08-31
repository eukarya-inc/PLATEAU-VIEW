//! Resolving the shapes PLATEAU data actually arrives in.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use plateau_converter_core::dataset::{Dataset, Staging};
use tempfile::TempDir;
use zip::write::SimpleFileOptions;

/// Writes `files` (relative path -> contents) into a new zip.
fn make_zip(path: &Path, files: &[(&str, &str)]) {
    let file = fs::File::create(path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    for (name, contents) in files {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

const GML: &str = r#"<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"/>"#;

#[test]
fn a_package_directory_is_used_in_place() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("udx/bldg/a.gml"), GML);
    write(&dir.path().join("codelists/x.xml"), "<x/>");

    let dataset = Dataset::open(&[dir.path().to_owned()]).unwrap();

    assert!(!dataset.is_staged(), "nothing should be copied");
    assert_eq!(dataset.root(), dir.path());
    assert_eq!(dataset.feature_types().unwrap(), ["bldg"]);
    assert_eq!(dataset.gml_files("bldg").unwrap().len(), 1);
}

#[test]
fn a_package_one_level_down_is_found() {
    // What unzipping a portal download by hand leaves behind.
    let dir = TempDir::new().unwrap();
    let package = dir.path().join("22100_shizuoka-shi_city_2023_citygml_1_op");
    write(&package.join("udx/bldg/a.gml"), GML);
    write(
        &package.join("schemas/iur/uro/3.0/urbanObject.xsd"),
        "<xs:schema/>",
    );

    let dataset = Dataset::open(&[dir.path().to_owned()]).unwrap();

    assert_eq!(dataset.root(), package);
    assert_eq!(dataset.parts(), ["udx", "schemas"]);
}

#[test]
fn a_zipped_package_is_extracted_and_unwrapped() {
    let dir = TempDir::new().unwrap();
    let archive = dir
        .path()
        .join("22100_shizuoka-shi_city_2023_citygml_1_op.zip");
    make_zip(
        &archive,
        &[
            (
                "22100_shizuoka-shi_city_2023_citygml_1_op/udx/bldg/a.gml",
                GML,
            ),
            (
                "22100_shizuoka-shi_city_2023_citygml_1_op/codelists/x.xml",
                "<x/>",
            ),
        ],
    );

    let dataset = Dataset::open(&[archive]).unwrap();

    assert!(dataset.is_staged());
    // The wrapping directory is stripped: udx sits at the root.
    assert!(dataset.root().join("udx/bldg/a.gml").is_file());
    assert!(dataset.root().join("codelists/x.xml").is_file());
    assert_eq!(dataset.gml_files("bldg").unwrap().len(), 1);
}

#[test]
fn part_zips_that_contain_their_own_directory_are_merged() {
    let dir = TempDir::new().unwrap();
    let udx = dir.path().join("udx.zip");
    let codelists = dir.path().join("codelists.zip");
    let schemas = dir.path().join("schemas.zip");
    make_zip(&udx, &[("udx/bldg/a.gml", GML), ("udx/tran/b.gml", GML)]);
    make_zip(
        &codelists,
        &[("codelists/Common_urbanPlanType.xml", "<x/>")],
    );
    make_zip(
        &schemas,
        &[("schemas/iur/uro/3.0/urbanObject.xsd", "<xs:schema/>")],
    );

    let dataset = Dataset::open(&[udx, codelists, schemas]).unwrap();

    assert!(dataset.is_staged());
    assert_eq!(dataset.parts(), ["udx", "codelists", "schemas"]);
    assert_eq!(dataset.feature_types().unwrap(), ["bldg", "tran"]);
    assert!(
        dataset
            .root()
            .join("codelists/Common_urbanPlanType.xml")
            .is_file()
    );
    assert!(
        dataset
            .root()
            .join("schemas/iur/uro/3.0/urbanObject.xsd")
            .is_file()
    );
}

#[test]
fn part_zips_holding_only_their_contents_are_placed_by_name() {
    // The portal also serves archives whose entries start *inside* the part.
    let dir = TempDir::new().unwrap();
    let udx = dir.path().join("22100_shizuoka_2023_citygml_1_op_udx.zip");
    let codelists = dir
        .path()
        .join("22100_shizuoka_2023_citygml_1_op_codelists.zip");
    make_zip(&udx, &[("bldg/a.gml", GML)]);
    make_zip(&codelists, &[("Common_urbanPlanType.xml", "<x/>")]);

    let dataset = Dataset::open(&[udx, codelists]).unwrap();

    assert!(dataset.root().join("udx/bldg/a.gml").is_file());
    assert!(
        dataset
            .root()
            .join("codelists/Common_urbanPlanType.xml")
            .is_file()
    );
}

#[test]
fn a_part_zip_is_identified_from_its_contents_when_the_name_says_nothing() {
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("download-1.zip");
    make_zip(&archive, &[("bldg/a.gml", GML)]);

    let dataset = Dataset::open(&[archive]).unwrap();

    assert!(
        dataset.root().join("udx/bldg/a.gml").is_file(),
        "a .gml means udx"
    );
}

#[test]
fn a_loose_part_directory_is_placed_by_name() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("udx/bldg/a.gml"), GML);
    write(&dir.path().join("codelists/x.xml"), "<x/>");

    let dataset = Dataset::open(&[dir.path().join("udx"), dir.path().join("codelists")]).unwrap();

    assert!(dataset.is_staged());
    assert!(dataset.root().join("udx/bldg/a.gml").is_file());
    assert!(dataset.root().join("codelists/x.xml").is_file());
}

#[test]
fn staging_can_be_pinned_to_a_directory_and_inspected() {
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("udx.zip");
    make_zip(&archive, &[("udx/bldg/a.gml", GML)]);
    let staging = dir.path().join("staging");

    let dataset = Dataset::open_with(&[archive], &Staging::At(staging.clone())).unwrap();

    assert_eq!(dataset.root(), staging);
    assert!(!dataset.is_staged(), "a pinned directory is not temporary");
    drop(dataset);
    assert!(
        staging.join("udx/bldg/a.gml").is_file(),
        "a pinned staging dir survives"
    );
}

#[test]
fn companion_files_are_the_non_gml_ones() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("udx/bldg/a.gml"), GML);
    write(
        &dir.path().join("udx/bldg/a_appearance/t.jpg"),
        "not really a jpeg",
    );

    let dataset = Dataset::open(&[dir.path().to_owned()]).unwrap();

    assert_eq!(dataset.gml_files("bldg").unwrap().len(), 1);
    let companions = dataset.companion_files("bldg").unwrap();
    assert_eq!(companions.len(), 1);
    assert!(companions[0].ends_with("udx/bldg/a_appearance/t.jpg"));
}

#[test]
fn a_dataset_without_udx_is_rejected() {
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("codelists.zip");
    make_zip(&archive, &[("codelists/x.xml", "<x/>")]);

    let error = Dataset::open(&[archive]).unwrap_err().to_string();

    assert!(error.contains("udx"), "{error}");
}

#[test]
fn an_unrecognisable_input_is_rejected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("notes.txt");
    fs::write(&path, "hello").unwrap();

    let error = Dataset::open(&[path]).unwrap_err().to_string();

    assert!(error.contains("expected a directory or a .zip"), "{error}");
}

#[test]
fn no_input_is_rejected() {
    let error = Dataset::open(&[] as &[PathBuf]).unwrap_err().to_string();
    assert!(error.contains("no input"), "{error}");
}

/// Windows-made part zips separate entries with backslashes; the layout must
/// be recognised exactly as if they used forward slashes.
#[test]
fn a_zip_with_backslash_entries_stages_like_a_forward_slash_one() {
    let dir = TempDir::new().unwrap();
    let udx = dir.path().join("x_udx.zip");
    make_zip(&udx, &[("udx/bldg/a.gml", GML)]);
    let lists = dir.path().join("x_codelists.zip");
    make_zip(&lists, &[("codelists\\", ""), ("codelists\\a.xml", "<x/>")]);
    let staging = dir.path().join("staging");

    let dataset = Dataset::open_with(&[udx, lists], &Staging::At(staging.clone())).unwrap();

    assert!(
        staging.join("codelists/a.xml").is_file(),
        "the part directory inside the archive is the part root, not a \
         subdirectory of it"
    );
    assert!(!staging.join("codelists/codelists").exists());
    assert_eq!(dataset.parts(), vec!["udx", "codelists"]);
}

#[test]
fn zip_entries_cannot_escape_the_staging_directory() {
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("evil.zip");
    make_zip(
        &archive,
        &[("udx/bldg/a.gml", GML), ("../escaped.gml", GML)],
    );
    let staging = dir.path().join("staging");

    Dataset::open_with(&[archive], &Staging::At(staging.clone())).unwrap();

    assert!(staging.join("udx/bldg/a.gml").is_file());
    assert!(
        !dir.path().join("escaped.gml").exists(),
        "traversal must be refused"
    );
}
