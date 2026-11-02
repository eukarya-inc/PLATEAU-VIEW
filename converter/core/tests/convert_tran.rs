//! End-to-end conversion of real PLATEAU 2024 roads.
//!
//! The fixture under `tests/fixtures/plateau-tran` holds four Sumida roads
//! with `uro:` 3.1 attributes, namely: one divided at LOD2 only, one divided
//! at LOD2 and LOD3 with auxiliary areas and lodType 3.0, one with LOD3 areas
//! only including a lane and lodType 3.2, and one with the full-width surface
//! alone.

use std::path::{Path, PathBuf};

use plateau_converter_core::convert::{Converter, Options, convert_to_string};
use plateau_converter_core::profile::Rules;
use plateau_converter_core::report::FileReport;
use plateau_converter_core::tran::Clearance;
use plateau_converter_core::xml;
use plateau_converter_core::{PROFILES, detect};

fn fixture_gml() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/plateau-tran/udx/tran/53394653_tran_6697_op.gml")
}

fn convert_fixture() -> (String, FileReport) {
    convert_fixture_with(Options::default())
}

fn convert_fixture_with(options: Options) -> (String, FileReport) {
    let source = xml::read_to_string(&fixture_gml()).unwrap();
    let declared = xml::root_namespaces(&source);
    let candidates: Vec<Rules> = PROFILES
        .iter()
        .map(|(_, toml)| Rules::from_toml(toml).unwrap())
        .collect();
    let found = detect::select(&candidates, &declared).expect("the fixture must match a profile");
    let rules = candidates[found.index].clone();
    assert_eq!(rules.name(), "iur-3.1-to-4.0", "the fixture is i-UR 3.1");
    let converter = Converter::new(rules, options).unwrap();
    convert_to_string(&converter, "fixture", &source).unwrap()
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

#[test]
fn every_road_converts_and_no_2_0_shape_survives() {
    let (output, report) = convert_fixture();
    assert_eq!(report.features, 4);
    for stale in [
        "<tran:trafficArea",
        "<tran:auxiliaryTrafficArea",
        "<tran:lod1MultiSurface",
        "<tran:lod2MultiSurface",
        "<tran:lod3MultiSurface",
        "citygml/transportation/2.0",
    ] {
        assert!(!output.contains(stale), "output still holds {stale}");
    }
}

/// Every surface sits under a space, every space says its granularity, and
/// the spaces wrapping real areas are named after them.
#[test]
fn every_area_is_reached_through_a_space() {
    let (output, _) = convert_fixture();
    let areas = count(&output, "<tran:TrafficArea") + count(&output, "<tran:AuxiliaryTrafficArea");
    let spaces =
        count(&output, "<tran:TrafficSpace") + count(&output, "<tran:AuxiliaryTrafficSpace");
    assert_eq!(areas, spaces);
    assert_eq!(count(&output, "<tran:granularity>"), spaces);
    assert_eq!(count(&output, "<core:boundary>"), spaces);
    // 12 areas in the input, plus 2 minted full-width areas.
    assert_eq!(areas, 14);
    assert!(output.contains(
        r#"<tran:TrafficSpace gml:id="tra_04806b92-8819-4fad-b0fa-24239d9f3a07_space">"#
    ));
    assert_eq!(
        count(&output, "<tran:granularity>lane</tran:granularity>"),
        3
    );
}

/// The geometry slots move down one LOD, and the full-width surface survives
/// only where no area divides it at LOD2.
#[test]
fn geometry_moves_down_one_lod() {
    let (output, _) = convert_fixture();
    // 5 areas held LOD2 and 7 held LOD3 in the input, and 2 full widths survive.
    assert_eq!(count(&output, "<core:lod1MultiSurface"), 5 + 2);
    assert_eq!(count(&output, "<core:lod2MultiSurface"), 7);
    assert_eq!(count(&output, "<core:lod3MultiSurface"), 0);
    assert_eq!(
        count(
            &output,
            r#"<tran:function codeSpace="../../codelists/TrafficArea_function.xml">1140</tran:function>"#
        ),
        2
    );
}

/// The quality descriptors follow the geometry and lodType takes the 4.0
/// code.
#[test]
fn quality_descriptors_follow_the_geometry() {
    let (output, report) = convert_fixture();
    assert!(
        !output.contains("SrcDescLod3"),
        "nothing describes an LOD3 band"
    );
    assert_eq!(count(&output, "<urc:geometrySrcDescLod2"), 4);
    assert_eq!(count(&output, "<urc:geometrySrcDescLod1"), 4);
    assert!(output.contains(r#"codeSpace="../../codelists/Road_lodType.xml">2.0</urc:lodType>"#));
    assert!(output.contains(r#"codeSpace="../../codelists/Road_lodType.xml">2.1</urc:lodType>"#));
    assert!(!output.contains(">3.0</urc:lodType>"));
    assert!(
        !report
            .warnings
            .iter()
            .any(|(m, _)| m.contains("could not be determined")),
        "{}",
        report.warnings
    );
}

/// tran:class moves to the per-type list, and the hooks land where the
/// generated rules put them.
#[test]
fn class_and_hooks_take_their_3_0_homes() {
    let (output, _) = convert_fixture();
    assert_eq!(
        count(
            &output,
            r#"<tran:class codeSpace="../../codelists/Road_class.xml">1040</tran:class>"#
        ),
        4
    );
    assert!(!output.contains("TransportationComplex_class.xml"));
    assert_eq!(count(&output, "<core:adeOfAbstractCityObject>"), 4);
    assert_eq!(count(&output, "<tran:adeOfRoad>"), 4);
}

/// With the extrusion enabled, every space whose area has an LOD1 surface
/// gets a solid at the profile's road height, a CSV row for one area wins
/// over it, and the spaces whose areas carry only LOD2 stay without one.
#[test]
fn clearance_extrudes_every_lod1_space() {
    let (plain, _) = convert_fixture();
    assert_eq!(count(&plain, "<core:lod1Solid>"), 0);

    let csv = "gml_id,height\ntra_04806b92-8819-4fad-b0fa-24239d9f3a07,9\n";
    let options = Options {
        clearance: Some(Clearance {
            height: None,
            overrides: Clearance::parse_csv(csv).unwrap(),
        }),
        ..Options::default()
    };
    let (output, report) = convert_fixture_with(options);

    // 5 divided-area spaces and 2 minted full-width spaces carry LOD1.
    assert_eq!(count(&output, "<core:lod1Solid>"), 7);
    assert_eq!(count(&output, "<gml:Shell>"), 7);
    assert_eq!(count(&output, "<gml:CompositeSolid"), 0);
    assert!(
        output.contains(
            r#"<gml:Solid gml:id="tra_04806b92-8819-4fad-b0fa-24239d9f3a07_space_solid">"#
        )
    );
    assert!(
        output.contains(" 9 "),
        "the CSV height must reach the top face"
    );
    assert!(
        output.contains(" 4.5 "),
        "the profile default must reach the other spaces"
    );
    let extruded_by = |height: &str| -> usize {
        let needle = format!("extruded upward by {height} m");
        report
            .warnings
            .iter()
            .filter(|(m, _)| m.contains(&needle))
            .map(|(_, n)| n)
            .sum()
    };
    assert_eq!(extruded_by("4.5"), 6);
    assert_eq!(extruded_by("9"), 1);

    // The envelope's top rises by the largest height the run can extrude by.
    assert!(
        plain.contains("<gml:upperCorner>35.71697615109303 139.80021956325191 2.7107658593039097<")
    );
    assert!(
        output.contains("<gml:upperCorner>35.71697615109303 139.80021956325191 11.71076585930391<")
    );
}

/// A transportation feature carries no geometry of its own in CityGML 3.0, so
/// every surface has to reach a space. A feature holding areas hands its own
/// surfaces to them; a feature holding none has nowhere else to put them.
const AREALESS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
  xmlns:tran="http://www.opengis.net/citygml/transportation/2.0"
  xmlns:gml="http://www.opengis.net/gml"
  xmlns:xlink="http://www.w3.org/1999/xlink"
  xmlns:uro="https://www.geospatial.jp/iur/uro/3.1">
<core:cityObjectMember>
<tran:Road gml:id="alone">
  <tran:lod2MultiSurface><gml:MultiSurface gml:id="ms_a2"/></tran:lod2MultiSurface>
  <tran:lod3MultiSurface><gml:MultiSurface gml:id="ms_a3"/></tran:lod3MultiSurface>
</tran:Road>
</core:cityObjectMember>
<core:cityObjectMember>
<tran:Road gml:id="full">
  <tran:lod1MultiSurface><gml:MultiSurface gml:id="ms_f1"/></tran:lod1MultiSurface>
  <tran:lod2MultiSurface><gml:MultiSurface gml:id="ms_f2"/></tran:lod2MultiSurface>
</tran:Road>
</core:cityObjectMember>
<core:cityObjectMember>
<tran:Square gml:id="referenced">
  <tran:trafficArea xlink:href="../tran/x.gml#area_1"/>
  <tran:lod2MultiSurface><gml:MultiSurface gml:id="ms_r2"/></tran:lod2MultiSurface>
</tran:Square>
</core:cityObjectMember>
</core:CityModel>
"#;

#[test]
fn a_feature_holding_no_area_keeps_its_own_surfaces() {
    let (_, toml) = PROFILES
        .iter()
        .find(|(name, _)| *name == "iur-3.1-to-4.0")
        .unwrap();
    let rules = Rules::from_toml(toml).unwrap();
    let converter = Converter::new(rules, Options::default()).unwrap();
    let (output, _) = convert_to_string(&converter, "arealess", AREALESS).unwrap();

    // The arealess road's LOD2 and LOD3 aggregate nothing, so they become the
    // full-width area's LOD1 and LOD2. The road that also carries its own LOD1
    // keeps that at LOD1 and drops its aggregate.
    assert!(output.contains(r#"gml:id="ms_a2""#));
    assert!(output.contains(r#"gml:id="ms_a3""#));
    assert!(output.contains(r#"gml:id="ms_f1""#));
    assert!(!output.contains(r#"gml:id="ms_f2""#));
    assert_eq!(count(&output, "<core:lod1MultiSurface"), 2);
    assert_eq!(count(&output, "<core:lod2MultiSurface"), 1);

    // The square's areas are referenced, so they carry the geometry its own
    // aggregate stands for and it mints no full-width space of its own.
    assert!(!output.contains(r#"gml:id="ms_r2""#));
    assert_eq!(count(&output, "<tran:TrafficSpace"), 2);
    assert!(output.contains(r#"<tran:trafficSpace xlink:href="../tran/x.gml#area_1_space"/>"#));
}
