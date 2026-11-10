//! End-to-end conversion of real PLATEAU water bodies.
//!
//! The fixture under `tests/fixtures/plateau-wtr` holds the following:
//!
//! * `udx/wtr/533936_wtr_6697_op.gml`: Two Takeshiba water bodies (i-UR 3.2),
//!   one with LOD0 to LOD3 and boundary surfaces at LOD2 and LOD3, one with
//!   LOD0 and LOD1 only, cut down to two members per geometry, with their
//!   appearance cut down to the textures and materials on the members kept.
//! * `udx/fld/53393681_fld_6697_l2_op.gml`: One Takeshiba flood risk area
//!   (i-UR 3.2) with LOD1 only and a river flooding risk attribute.

use std::collections::HashSet;
use std::path::Path;

use plateau_converter_core::convert::{Converter, Options, convert_to_string};
use plateau_converter_core::profile::Rules;
use plateau_converter_core::report::FileReport;
use plateau_converter_core::xml;
use plateau_converter_core::{PROFILES, detect};

const WTR: &str = "wtr/533936_wtr_6697_op.gml";
const FLD: &str = "fld/53393681_fld_6697_l2_op.gml";

fn convert(file: &str) -> (String, FileReport) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/plateau-wtr/udx")
        .join(file);
    let source = xml::read_to_string(&path).unwrap();
    let declared = xml::root_namespaces(&source);
    let candidates: Vec<Rules> = PROFILES
        .iter()
        .map(|(_, toml)| Rules::from_toml(toml).unwrap())
        .collect();
    let found = detect::select(&candidates, &declared).expect("the fixture must match a profile");
    let converter = Converter::new(candidates[found.index].clone(), Options::default()).unwrap();
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

/// Every LOD1 multi-surface sits in a water surface of its own, which no
/// 2.0 water body property survives, and no `gml:id` appears twice.
#[test]
fn lod1_moves_into_a_water_surface() {
    for (file, bodies) in [(WTR, 2), (FLD, 1)] {
        let (output, _) = convert(file);
        for stale in [
            "<wtr:lod",
            "<wtr:boundedBy",
            "WaterClosureSurface",
            "citygml/waterbody/2.0",
        ] {
            assert!(!output.contains(stale), "{file} still holds {stale}");
        }
        assert_eq!(count(&output, "<core:lod1MultiSurface>"), bodies);
        for (i, _) in output.match_indices("<core:lod1MultiSurface>") {
            let before = output[..i].trim_end();
            let parent = &before[before.rfind('<').unwrap()..];
            assert!(parent.starts_with("<wtr:WaterSurface "), "{file}: {parent}");
        }
        let mut seen = HashSet::new();
        for id in attribute_values(&output, "gml:id") {
            assert!(seen.insert(id), "{file}: gml:id {id} appears twice");
        }
    }
}

/// Old LOD3 takes the LOD2 slots with lodType 2.1, what old LOD2 held goes,
/// and every boundary surface holds its LOD2 as a multi-surface.
#[test]
fn old_lod3_becomes_lod2() {
    let (output, _) = convert(WTR);
    assert!(!output.contains("<core:lod3"));
    assert_eq!(count(&output, "<core:lod2Solid>"), 1);
    assert_eq!(count(&output, "<core:lod2MultiSurface>"), 3);
    for (i, _) in output.match_indices("<core:lod2MultiSurface>") {
        let inner = output[i + "<core:lod2MultiSurface>".len()..].trim_start();
        assert!(inner.starts_with("<gml:MultiSurface"), "{inner:.60}");
    }
    assert_eq!(count(&output, ">2.1</urc:lodType>"), 1);
    assert_eq!(count(&output, "<urc:geometrySrcDescLod2"), 3);
}

/// Every texture and material link names a surface the output still holds,
/// and the textures of the dropped old LOD2 are gone.
#[test]
fn every_texture_link_resolves() {
    let (output, _) = convert(WTR);
    let ids: HashSet<&str> = attribute_values(&output, "gml:id").into_iter().collect();
    let targets: Vec<&str> = output
        .match_indices("<app:target>")
        .map(|(i, _)| {
            let rest = &output[i + "<app:target>".len()..];
            rest[..rest.find('<').unwrap()].trim_start_matches('#')
        })
        .collect();
    assert!(!targets.is_empty());
    for target in targets {
        assert!(ids.contains(target), "app:target {target} names no surface");
    }
    assert_eq!(count(&output, "<app:ParameterizedTexture"), 2);
}

/// The flood risk attribute and the quality attribute sit on the city
/// object's hook.
#[test]
fn the_risk_attribute_is_rehomed() {
    let (output, _) = convert(FLD);
    assert_eq!(count(&output, "<core:adeOfAbstractCityObject>"), 2);
    assert_eq!(count(&output, "<urc:RiverFloodingRiskAttribute>"), 1);
    assert_eq!(count(&output, "<urc:ExteriorDataQualityAttribute>"), 1);
    assert!(!output.contains("<uro:floodingRiskAttribute"));
}
