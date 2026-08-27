//! End-to-end conversion of a real PLATEAU 2023 bldg file.
//!
//! The fixture under `tests/fixtures/plateau` is a LOD0 roof edge + LOD1 solid
//! package with `uro:` 3.0 attributes — the shape most of PLATEAU is in.

use std::path::{Path, PathBuf};

use plateau_converter_core::DEFAULT_PROFILE;
use plateau_converter_core::convert::{Converter, Options, convert_to_string};
use plateau_converter_core::dataset::Dataset;
use plateau_converter_core::profile::Rules;
use plateau_converter_core::report::FileReport;
use plateau_converter_core::xml::{self, Chunk, Reader};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plateau")
}

fn fixture_gml() -> PathBuf {
    fixture_root().join("udx/bldg/52382287_bldg_6697_psc_op.gml")
}

fn converter() -> Converter {
    Converter::new(
        Rules::from_toml(DEFAULT_PROFILE).unwrap(),
        Options::default(),
    )
    .unwrap()
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
    // AbstractConstruction, and [3.0] 1.2.3.1.1 requires the subtype's.
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
fn uro_attributes_move_to_i_ur_4_0_unchanged() {
    let (output, _) = convert_fixture();
    assert!(output.contains(r#"xmlns:uro="https://www.geospatial.jp/iur/uro/4.0""#));
    assert!(!output.contains("iur/uro/3.0"));
    assert!(output.contains("<uro:buildingID>22102-bldg-354359</uro:buildingID>"));
    assert!(output.contains("<uro:BuildingDataQualityAttribute>"));
    assert!(output.contains(r#"codeSpace="../../codelists/Common_urbanPlanType.xml""#));
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
    let converter = Converter::new(Rules::from_toml(DEFAULT_PROFILE).unwrap(), options).unwrap();
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
fn i_ur_consolidations_are_flagged_not_guessed() {
    let (output, report) = convert_fixture();

    let flagged: Vec<&str> = report
        .warnings
        .iter()
        .map(|(m, _)| m)
        .filter(|m| m.contains("was left unchanged"))
        .collect();
    assert_eq!(flagged.len(), 2, "{flagged:#?}");
    assert!(
        flagged
            .iter()
            .any(|m| m.starts_with("uro:BuildingDataQualityAttribute"))
    );
    assert!(
        flagged
            .iter()
            .any(|m| m.starts_with("uro:BuildingLandSlideRiskAttribute"))
    );

    // Flagged means reported, not dropped or half-converted.
    assert!(output.contains("<uro:BuildingDataQualityAttribute>"));
    assert!(!output.contains("urc:"));
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
    assert_eq!(report.copied, 6);
}
