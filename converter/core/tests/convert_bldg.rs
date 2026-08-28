//! End-to-end conversion of a real PLATEAU 2023 bldg file.
//!
//! The fixture under `tests/fixtures/plateau` is a LOD0 roof edge + LOD1 solid
//! package with `uro:` 3.0 attributes — the shape most of PLATEAU is in.

use std::path::{Path, PathBuf};

use plateau_converter_core::convert::{Converter, Options, convert_to_string};
use plateau_converter_core::dataset::Dataset;
use plateau_converter_core::profile::Rules;
use plateau_converter_core::report::FileReport;
use plateau_converter_core::xml::{self, Chunk, Reader};
use plateau_converter_core::{PROFILES, detect};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plateau")
}

fn fixture_gml() -> PathBuf {
    fixture_root().join("udx/bldg/52382287_bldg_6697_psc_op.gml")
}

/// The profile the fixture's own namespaces select. The fixture is i-UR 3.0, so
/// this also asserts that detection does not quietly fall back to the default.
fn fixture_rules() -> Rules {
    let source = xml::read_to_string(&fixture_gml()).unwrap();
    let declared = xml::root_namespaces(&source);
    let candidates: Vec<Rules> = PROFILES
        .iter()
        .map(|(_, toml)| Rules::from_toml(toml).unwrap())
        .collect();
    let found = detect::select(&candidates, &declared).expect("the fixture must match a profile");
    let rules = candidates[found.index].clone();
    assert_eq!(rules.name(), "iur-3.0-to-4.0", "the fixture is i-UR 3.0");
    rules
}

fn converter() -> Converter {
    Converter::new(fixture_rules(), Options::default()).unwrap()
}

fn convert_fixture() -> (String, FileReport) {
    let source = xml::read_to_string(&fixture_gml()).unwrap();
    convert_to_string(&converter(), "fixture", &source).unwrap()
}

#[test]
fn converts_every_building_in_the_fixture() {
    let (_, report) = convert_fixture();
    // Four buildings; `gml:boundedBy` is a CityModel property, not a feature.
    assert_eq!(report.features, 4);
}

#[test]
fn no_citygml_2_0_namespace_survives() {
    let (output, _) = convert_fixture();
    for stale in [
        "opengis.net/citygml/2.0",
        "citygml/building/2.0",
        "citygml/generics/2.0",
        r#""http://www.opengis.net/gml""#, // GML 3.1.1
    ] {
        assert!(!output.contains(stale), "output still references {stale}");
    }
    assert!(output.contains(r#"xmlns:core="http://www.opengis.net/citygml/3.0""#));
    assert!(output.contains(r#"xmlns:gml="http://www.opengis.net/gml/3.2""#));
}

/// CityGML 3.0 puts each thematic module under its own namespace path and calls
/// the base module `core.xsd`; 2.0 was flat and called it `cityGMLBase.xsd`.
/// Getting this wrong points every converted file at URLs that do not exist,
/// which nothing else here would catch. i-UR is the other half of the split:
/// remote for CityGML, relative into the package for i-UR.
#[test]
fn schema_location_uses_the_3_0_layout() {
    let (output, _) = convert_fixture();
    for location in [
        "http://schemas.opengis.net/citygml/3.0/core.xsd",
        "http://schemas.opengis.net/citygml/building/3.0/building.xsd",
        "http://schemas.opengis.net/citygml/construction/3.0/construction.xsd",
        "http://schemas.opengis.net/gml/3.2.1/gml.xsd",
        "../../schemas/iur/uro/4.0/urbanObject.xsd",
    ] {
        assert!(output.contains(location), "schemaLocation lacks {location}");
    }
    assert!(
        !output.contains("cityGMLBase.xsd"),
        "cityGMLBase.xsd is the CityGML 2.0 file name"
    );
    assert!(
        !output.contains("citygml/3.0/building.xsd"),
        "the flat 2.0 schema layout does not exist in 3.0"
    );
}

#[test]
fn measured_height_becomes_a_construction_height() {
    let (output, _) = convert_fixture();
    assert!(!output.contains("measuredHeight"));
    assert!(output.contains(r#"<con:value uom="m">14.3</con:value>"#));
    assert!(output.contains("<con:status>measured</con:status>"));
}

#[test]
fn lod0_roof_edge_becomes_a_roof_surface_boundary() {
    let (output, _) = convert_fixture();
    assert!(!output.contains("lod0RoofEdge"));
    // con:boundary, not core:boundary: Building inherits the role from
    // AbstractConstruction, and the subtype's role is the one that wins.
    assert!(output.contains("<con:boundary>"));
    assert!(!output.contains("<core:boundary>"));
    assert!(output.contains("<con:RoofSurface"));
    assert!(output.contains("<core:lod0MultiSurface>"));
}

#[test]
fn lod1_solid_moves_to_the_core_module() {
    let (output, _) = convert_fixture();
    assert!(!output.contains("<bldg:lod1Solid>"));
    assert!(output.contains("<core:lod1Solid>"));
}

/// i-UR moves 3.x -> 4.0 by namespace alone; element names and codeSpace paths
/// are left exactly as they were.
#[test]
fn uro_attributes_reach_i_ur_4_0_with_their_values_intact() {
    let (output, _) = convert_fixture();
    assert!(output.contains(r#"xmlns:uro="https://www.geospatial.jp/iur/uro/4.0""#));
    assert!(!output.contains("iur/uro/3.0"));
    // Values and codeSpace paths are carried through untouched, whatever
    // happens to the element around them.
    assert!(output.contains("<uro:buildingID>22102-bldg-354359</uro:buildingID>"));
    assert!(output.contains(r#"codeSpace="../../codelists/Common_urbanPlanType.xml""#));
}

/// i-UR 4.0 moves the shared attribute groups into the Urban Core module, and
/// i-UR 3.0's per-feature-type classes collapse into the generic ones. Neither
/// name survives, so the fixture is a check that both actually convert rather
/// than landing in a namespace that does not declare them.
#[test]
fn the_quality_class_moves_to_urban_core() {
    let (output, _) = convert_fixture();
    assert!(
        !output.contains("<uro:BuildingDataQualityAttribute>"),
        "i-UR 4.0 declares no per-feature-type quality class"
    );
    assert!(output.contains("<urc:ExteriorDataQualityAttribute>"));
    assert!(output.contains(r#"xmlns:urc="https://www.geospatial.jp/iur/urc/4.0""#));
}

/// The fixture's one generic attribute has to be restructured: 2.0 put the name
/// in an XML attribute, 3.0 makes it an element inside a wrapped data type.
#[test]
fn generic_attributes_are_wrapped_and_renamed() {
    let (output, _) = convert_fixture();
    assert!(!output.contains("<gen:stringAttribute"));
    assert!(output.contains("<core:genericAttribute>"));
    assert!(output.contains("<gen:StringAttribute>"));
    assert!(output.contains("<gen:name>風致地区</gen:name>"));
    assert!(output.contains("<gen:value>第1種風致地区（大崩）</gen:value>"));
}

#[test]
fn geometry_coordinates_are_carried_through_verbatim() {
    let source = xml::read_to_string(&fixture_gml()).unwrap();
    let (output, _) = convert_to_string(&converter(), "fixture", &source).unwrap();

    let pos_lists = |text: &str| -> Vec<String> {
        text.match_indices("<gml:posList>")
            .map(|(i, tag)| {
                let rest = &text[i + tag.len()..];
                rest[..rest.find("</gml:posList>").unwrap()].to_string()
            })
            .collect()
    };
    // Reordering moves the roof edge after lod1Solid, so compare as a multiset:
    // the point is that no coordinate string is rewritten, reformatted or lost.
    let mut before = pos_lists(&source);
    let mut after = pos_lists(&output);
    assert_eq!(before.len(), 30, "the fixture's geometry count");
    before.sort();
    after.sort();
    assert_eq!(after, before, "coordinates must not be rewritten");
}

#[test]
fn every_geometry_gets_a_gml_id() {
    let (output, _) = convert_fixture();
    for geometry in [
        "gml:Solid",
        "gml:CompositeSurface",
        "gml:MultiSurface",
        "gml:Polygon",
    ] {
        let opens = output.matches(&format!("<{geometry} ")).count();
        assert!(opens > 0, "no {geometry} in the output");
    }
    // Every opening geometry tag carries an id; count them against the tags.
    let ids = output.matches("gml:id=").count();
    let geometries = [
        "gml:Solid ",
        "gml:CompositeSurface ",
        "gml:MultiSurface ",
        "gml:Polygon ",
    ]
    .iter()
    .map(|g| output.matches(&format!("<{g}")).count())
    .sum::<usize>();
    // 4 Buildings + 4 generated RoofSurfaces already had or were given ids.
    assert_eq!(ids, geometries + 8);
}

#[test]
fn ids_are_absent_when_generation_is_off() {
    let options = Options {
        generate_gml_ids: false,
        ..Options::default()
    };
    let converter = Converter::new(fixture_rules(), options).unwrap();
    let source = xml::read_to_string(&fixture_gml()).unwrap();
    let (output, _) = convert_to_string(&converter, "fixture", &source).unwrap();
    assert!(!output.contains("<gml:Polygon gml:id="));
}

#[test]
fn output_is_well_formed_and_reparses() {
    let (output, _) = convert_fixture();
    let mut reader = Reader::new("output", &output);
    let mut members = 0;
    let mut saw_root = false;
    while let Some(chunk) = reader.next_chunk().expect("output must re-parse") {
        match chunk {
            Chunk::RootStart(root) => {
                assert!(root.is("http://www.opengis.net/citygml/3.0", "CityModel"));
                saw_root = true;
            }
            Chunk::Member(_) => members += 1,
            _ => {}
        }
    }
    assert!(saw_root);
    assert_eq!(members, 5, "gml:boundedBy plus four cityObjectMembers");
}

/// i-UR 4.0 consolidations the converter must not guess at are named in the
/// report, and the elements themselves are left intact.
#[test]
fn every_i_ur_name_in_the_output_exists_in_i_ur_4_0() {
    let (output, _) = convert_fixture();

    // The failure this guards against is quiet: an element bumped into a
    // namespace that does not declare it still looks converted, and the writer
    // has no prefix for it, so it is emitted with no namespace at all.
    let declared = i_ur_4_0_element_names();
    let mut undeclared: Vec<String> = Vec::new();
    for prefix in ["uro", "urc", "urf", "urg", "urt"] {
        for name in element_names(&output, prefix) {
            if !declared.contains(&name) {
                undeclared.push(format!("{prefix}:{name}"));
            }
        }
    }
    assert!(
        undeclared.is_empty(),
        "not declared by i-UR 4.0: {undeclared:?}"
    );
}

/// The ADE properties CityGML 2.0 let an extension name for itself are replaced
/// by the general hook CityGML 3.0 declares on the host class.
#[test]
fn i_ur_classes_hang_off_the_citygml_ade_hooks() {
    let (output, _) = convert_fixture();
    assert!(!output.contains("<uro:buildingIDAttribute>"));
    assert!(output.contains("<bldg:adeOfAbstractBuilding>"));
    assert!(
        output.contains("<core:adeOfAbstractCityObject>"),
        "classes reaching the hook through an i-UR parent use the general one"
    );
}

#[test]
fn conversion_is_deterministic() {
    let (first, _) = convert_fixture();
    let (second, _) = convert_fixture();
    assert_eq!(first, second, "generated ids must not depend on run order");
}

#[test]
fn reports_the_assumptions_it_had_to_make() {
    let (_, report) = convert_fixture();
    let messages: Vec<&str> = report.warnings.iter().map(|(m, _)| m).collect();
    assert!(
        messages.iter().any(|m| m.contains("lowReference")),
        "the assumed height references must be reported: {messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("lod0RoofEdge")),
        "the roof-edge demotion must be reported: {messages:?}"
    );
}

/// A converted package resolves i-UR through its own `schemas/`, the way a
/// PLATEAU package does -- so the schemas have to be written, and the reference
/// to them has to be the relative path that finds them.
#[test]
fn the_i_ur_4_0_schemas_are_written_and_referenced_relatively() {
    let out = tempfile::tempdir().unwrap();
    let dataset = Dataset::open(&[fixture_root()]).unwrap();
    converter().convert_dataset(&dataset, out.path()).unwrap();

    for relative in [
        "schemas/iur/uro/4.0/urbanObject.xsd",
        "schemas/iur/urc/4.0/urbanCore.xsd",
        "schemas/iur/urf/4.0/urbanFunction.xsd",
        "schemas/iur/urg/4.0/statisticalGrid.xsd",
        "schemas/iur/urt/4.0/publicTransit.xsd",
    ] {
        assert!(out.path().join(relative).is_file(), "missing {relative}");
    }

    let converted =
        std::fs::read_to_string(out.path().join("udx/bldg/52382287_bldg_6697_psc_op.gml")).unwrap();
    assert!(
        converted.contains("../../schemas/iur/uro/4.0/urbanObject.xsd"),
        "i-UR must resolve through the package, not over the network"
    );
    assert!(
        converted.contains("http://schemas.opengis.net/citygml/building/3.0/building.xsd"),
        "CityGML stays remote, as in a PLATEAU package"
    );
    // The relative path has to actually reach the file from udx/bldg/.
    let from_gml = out
        .path()
        .join("udx/bldg/../../schemas/iur/uro/4.0/urbanObject.xsd");
    assert!(
        from_gml.is_file(),
        "the relative schemaLocation must resolve"
    );
}

#[test]
fn converts_a_whole_dataset_into_a_mirrored_tree() {
    let out = tempfile::tempdir().unwrap();
    let dataset = Dataset::open(&[fixture_root()]).unwrap();
    assert!(!dataset.is_staged(), "a package directory is used in place");

    let report = converter().convert_dataset(&dataset, out.path()).unwrap();

    assert_eq!(report.converted, 1);
    assert_eq!(report.features, 4);
    assert!(
        out.path()
            .join("udx/bldg/52382287_bldg_6697_psc_op.gml")
            .is_file()
    );
    // Codelists are referenced by relative path from the gml, so they must come along.
    assert!(
        out.path()
            .join("codelists/Common_urbanPlanType.xml")
            .is_file()
    );
    // 6 codelists plus the five i-UR 4.0 schemas the converter writes.
    assert_eq!(report.copied, 11);
}

/// Every element name i-UR 4.0 declares, read from the schemas the converter
/// ships. Reading them rather than listing them keeps the check honest when the
/// vendored schemas are updated.
fn i_ur_4_0_element_names() -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
    for (_, text) in plateau_converter_core::IUR_4_0_SCHEMAS {
        let mut rest = *text;
        while let Some(at) = rest.find("element name=\"") {
            rest = &rest[at + "element name=\"".len()..];
            if let Some(end) = rest.find('"') {
                names.insert(rest[..end].to_owned());
                rest = &rest[end..];
            }
        }
    }
    names
}

/// The distinct local names appearing as `<prefix:name` in a document.
fn element_names(document: &str, prefix: &str) -> std::collections::BTreeSet<String> {
    let open = format!("<{prefix}:");
    let mut names = std::collections::BTreeSet::new();
    let mut rest = document;
    while let Some(at) = rest.find(&open) {
        rest = &rest[at + open.len()..];
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        if end > 0 {
            names.insert(rest[..end].to_owned());
        }
    }
    names
}
