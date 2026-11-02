//! End-to-end conversion of real PLATEAU underground utilities.
//!
//! The fixture under `tests/fixtures/plateau-unf` holds the following:
//!
//! * `09LD283_unf_10170_op.gml`: One Takeshiba sewer pipe (i-UR 3.2) with
//!   LOD1 to LOD4 and a Dm attribute, and its appearance cut down to a
//!   material on LOD4 faces and a texture and a material on LOD3 faces.
//! * `08EE751_unf_10169_sewer_op.gml`: Three Nagaoka features (i-UR 3.1),
//!   namely a manhole with a Dm attribute only, a manhole with LOD2 only and a
//!   sewer pipe with LOD3 only.

use std::collections::HashSet;
use std::path::Path;

use plateau_converter_core::convert::{Converter, Options, convert_to_string};
use plateau_converter_core::profile::{Rules, UnfDm};
use plateau_converter_core::report::FileReport;
use plateau_converter_core::xml;
use plateau_converter_core::{PROFILES, detect};

const TAKESHIBA: &str = "09LD283_unf_10170_op.gml";
const NAGAOKA: &str = "08EE751_unf_10169_sewer_op.gml";

fn convert(file: &str, options: Options) -> (String, FileReport) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/plateau-unf/udx/unf")
        .join(file);
    let source = xml::read_to_string(&path).unwrap();
    let declared = xml::root_namespaces(&source);
    let candidates: Vec<Rules> = PROFILES
        .iter()
        .map(|(_, toml)| Rules::from_toml(toml).unwrap())
        .collect();
    let found = detect::select(&candidates, &declared).expect("the fixture must match a profile");
    let converter = Converter::new(candidates[found.index].clone(), options).unwrap();
    convert_to_string(&converter, file, &source).unwrap()
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

/// The values of every `attr="…"` in `text`.
fn attribute_values<'t>(text: &'t str, attr: &str) -> Vec<&'t str> {
    let open = format!("{attr}=\"");
    text.match_indices(&open)
        .map(|(i, _)| {
            let rest = &text[i + open.len()..];
            &rest[..rest.find('"').unwrap()]
        })
        .collect()
}

fn assert_unique_ids(output: &str) {
    let mut seen = HashSet::new();
    for id in attribute_values(output, "gml:id") {
        assert!(seen.insert(id), "gml:id {id} appears twice");
    }
}

#[test]
fn no_city_furniture_geometry_lod4_or_dm_survives() {
    for file in [TAKESHIBA, NAGAOKA] {
        let (output, _) = convert(file, Options::default());
        for stale in [
            "<frn:lod",
            "Lod4",
            "Lod3",
            "DmGeometricAttribute",
            "DmAttribute",
            "<uro:frnDataQualityAttribute",
            "citygml/cityfurniture/2.0",
        ] {
            assert!(!output.contains(stale), "{file} still holds {stale}");
        }
        assert_unique_ids(&output);
    }
}

#[test]
fn the_dm_geometry_becomes_the_lod0_and_a_feature_without_one_is_reported() {
    let (takeshiba, _) = convert(TAKESHIBA, Options::default());
    assert_eq!(count(&takeshiba, "<core:lod0MultiCurve>"), 1);

    let (nagaoka, report) = convert(NAGAOKA, Options::default());
    assert_eq!(count(&nagaoka, "<core:lod0Point>"), 1);
    let unreported: usize = report
        .warnings
        .iter()
        .filter(|(m, _)| m.contains("has no LOD0"))
        .map(|(_, n)| n)
        .sum();
    assert_eq!(unreported, 2, "{}", report.warnings);
}

/// Old LOD3 is exterior LOD2.1, written beside the LOD2.0 solid, and its
/// quality descriptors move with it.
#[test]
fn old_lod3_becomes_lod2() {
    let (output, _) = convert(TAKESHIBA, Options::default());
    assert_eq!(count(&output, "<core:lod1Solid>"), 1);
    assert_eq!(count(&output, "<core:lod2Solid>"), 1);
    assert_eq!(count(&output, "<core:lod2MultiSurface>"), 1);
    assert!(!output.contains("<core:lod3"));
    assert!(output.contains("<urc:appearanceSrcDescLod2"));

    let (output, _) = convert(NAGAOKA, Options::default());
    assert_eq!(count(&output, "<core:lod2Solid>"), 1);
    assert_eq!(count(&output, "<core:lod2MultiSurface>"), 1);
}

/// Every texture and material link names a surface the output still holds,
/// and a material whose every surface was LOD4 is gone.
#[test]
fn every_texture_link_resolves() {
    let (output, _) = convert(TAKESHIBA, Options::default());
    let ids: HashSet<&str> = attribute_values(&output, "gml:id").into_iter().collect();
    let targets: Vec<&str> = output
        .match_indices("<app:target>")
        .map(|(i, _)| {
            let rest = &output[i + "<app:target>".len()..];
            rest[..rest.find('<').unwrap()].trim_start_matches('#')
        })
        .collect();
    assert_eq!(
        targets.len(),
        4,
        "the LOD3 texture and material keep their links"
    );
    for target in targets {
        assert!(ids.contains(target), "app:target {target} names no surface");
    }
    assert_eq!(count(&output, "<app:X3DMaterial"), 1);
    assert_eq!(count(&output, "<app:ParameterizedTexture"), 1);
}

/// Kept Dm attributes sit on the city object's hook with every child in the
/// module the class declares it in, and the copied LOD0 duplicates no id.
#[test]
fn a_kept_dm_attribute_is_rehomed_whole() {
    let options = Options {
        unf_dm: Some(UnfDm::Keep),
        ..Options::default()
    };
    for file in [TAKESHIBA, NAGAOKA] {
        let (output, report) = convert(file, options.clone());
        let kept = count(&output, "<urc:DmGeometricAttribute");
        assert!(kept > 0, "{file} keeps its Dm attributes");
        assert_eq!(count(&output, "<urc:shapeType"), kept);
        assert_eq!(count(&output, "<urc:lod0Geometry>"), kept);
        assert!(!output.contains("<uro:frnDmAttribute"));
        assert!(!output.contains("<uro:shapeType"));
        assert!(
            report
                .warnings
                .iter()
                .any(|(m, _)| m.contains("dmCode 0001")),
            "{}",
            report.warnings
        );
        assert_unique_ids(&output);
    }
}
