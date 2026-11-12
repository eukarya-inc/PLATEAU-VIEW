//! End-to-end conversion of real PLATEAU city furniture and vegetation.
//!
//! The fixture under `tests/fixtures/plateau-frn-veg` holds the following,
//! each file cut down to the members listed and to the appearance of the
//! surfaces they keep:
//!
//! * `udx/frn/53393681_frn_6697_30_op.gml`: Three Takeshiba city furniture
//!   (i-UR 3.2), one with a Dm geometry only, one with an LOD1 multi-surface
//!   and LOD2, one with an LOD1 solid and LOD2.
//! * `udx/frn/54400098_frn_6697_op.gml`: One Tsukuba city furniture
//!   (i-UR 3.1) with an LOD1 multi-surface, LOD2, LOD3 and lodType 3.1.
//! * `udx/veg/53393681_veg_6697_op.gml`: Four Takeshiba vegetation objects,
//!   a solitary one with a Dm geometry only, a solitary one with an LOD1
//!   solid, a plant cover with an LOD1 multi-solid and one with LOD1 and
//!   LOD2 multi-solids.
//! * `udx/veg/53393680_veg_6697_op.gml`: One Takeshiba plant cover with LOD1,
//!   LOD2 and LOD3 multi-solids.
//! * `udx/veg/54401008_veg_6697_op.gml`: One Tsukuba plant cover with an LOD1
//!   multi-solid and LOD2 and LOD3 multi-surfaces.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use plateau_converter_core::convert::{Converter, Options, convert_to_string};
use plateau_converter_core::profile::Rules;
use plateau_converter_core::xml;
use plateau_converter_core::{PROFILES, detect};

const FRN: &[&str] = &[
    "frn/53393681_frn_6697_30_op.gml",
    "frn/54400098_frn_6697_op.gml",
];
const VEG: &[&str] = &[
    "veg/53393681_veg_6697_op.gml",
    "veg/53393680_veg_6697_op.gml",
    "veg/54401008_veg_6697_op.gml",
];

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plateau-frn-veg")
}

fn source(file: &str) -> String {
    xml::read_to_string(&fixture_root().join("udx").join(file)).unwrap()
}

/// The converter for the profile `source`'s namespaces select.
fn converter(source: &str) -> Converter {
    let declared = xml::root_namespaces(source);
    let candidates: Vec<Rules> = PROFILES
        .iter()
        .map(|(_, toml)| Rules::from_toml(toml).unwrap())
        .collect();
    let found = detect::select(&candidates, &declared).expect("the fixture must match a profile");
    Converter::new(candidates[found.index].clone(), Options::default()).unwrap()
}

fn convert(file: &str) -> String {
    let source = source(file);
    convert_to_string(&converter(&source), file, &source)
        .unwrap()
        .0
}

fn all() -> impl Iterator<Item = &'static str> {
    FRN.iter().chain(VEG).copied()
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

/// The text of every `<tag>…</tag>` in `text`.
fn element_texts<'t>(text: &'t str, tag: &str) -> Vec<&'t str> {
    let close = format!("</{tag}>");
    text.match_indices(&format!("<{tag}"))
        .map(|(i, _)| {
            let rest = &text[i..];
            let start = rest.find('>').unwrap() + 1;
            rest[start..rest.find(&close).unwrap()].trim()
        })
        .collect()
}

/// The element that opens right before position `i` in `text`.
fn parent_at(text: &str, i: usize) -> &str {
    let before = text[..i].trim_end();
    &before[before.rfind('<').unwrap()..]
}

/// No 2.0 geometry property, multi-solid or LOD3 property survives, and no
/// `gml:id` appears twice.
#[test]
fn every_geometry_reaches_a_3_0_property() {
    for file in all() {
        let output = convert(file);
        for stale in [
            "<frn:lod",
            "<veg:lod",
            "gml:MultiSolid",
            "<core:lod3",
            "cityfurniture/2.0",
            "vegetation/2.0",
        ] {
            assert!(!output.contains(stale), "{file} still holds {stale}");
        }
        let mut seen = HashSet::new();
        for id in attribute_values(&output, "gml:id") {
            assert!(seen.insert(id), "{file}: gml:id {id} appears twice");
        }
    }
}

/// A furniture's LOD1 surface sits in the furniture's i-UR extension, and
/// its LOD1 solid in the core property.
#[test]
fn lod1_surface_moves_into_the_extension() {
    for (file, surfaces, solids) in [(FRN[0], 1, 1), (FRN[1], 1, 0)] {
        let output = convert(file);
        assert_eq!(count(&output, "<uro:lod1MultiSurface>"), surfaces, "{file}");
        for (i, _) in output.match_indices("<uro:lod1MultiSurface>") {
            assert!(
                parent_at(&output, i).starts_with("<uro:CityFurnitureExtendedGeometry"),
                "{file}"
            );
            let holder = output[..i]
                .rfind("<uro:CityFurnitureExtendedGeometry")
                .unwrap();
            assert!(parent_at(&output, holder).starts_with("<frn:adeOfCityFurniture"));
        }
        assert_eq!(count(&output, "<core:lod1Solid>"), solids, "{file}");
    }
}

/// Each multi-solid becomes the core solid property of its LOD, holding the
/// solid itself, with the old LOD3 at LOD2.
#[test]
fn multi_solids_become_solids() {
    for (file, lod1, lod2) in [(VEG[0], 3, 1), (VEG[1], 1, 1), (VEG[2], 1, 0)] {
        let output = convert(file);
        assert_eq!(count(&output, "<core:lod1Solid>"), lod1, "{file}");
        assert_eq!(count(&output, "<core:lod2Solid>"), lod2, "{file}");
        for property in ["<core:lod1Solid>", "<core:lod2Solid>"] {
            for (i, _) in output.match_indices(property) {
                let inner = output[i + property.len()..].trim_start();
                assert!(inner.starts_with("<gml:Solid"), "{file}: {inner:.60}");
            }
        }
    }
    assert_eq!(count(&convert(VEG[2]), "<core:lod2MultiSurface>"), 1);
}

/// Every lodType written is a code of the published list its codeSpace
/// names, and a feature with LOD2 geometry carries one.
#[test]
fn lod_types_are_published_codes() {
    let lists = plateau_converter_core::CODELISTS_4_0;
    for file in all() {
        let output = convert(file);
        let spaces = attribute_values(&output, "codeSpace")
            .into_iter()
            .filter(|s| s.ends_with("_lodType.xml"))
            .collect::<Vec<_>>();
        let codes = element_texts(&output, "urc:lodType");
        assert_eq!(spaces.len(), codes.len(), "{file}");
        let lod2 = count(&output, "<core:lod2");
        assert!(
            codes.len() >= lod2,
            "{file}: {lod2} LOD2 features, {codes:?}"
        );
        for (space, code) in spaces.iter().zip(&codes) {
            let list = space.rsplit('/').next().unwrap();
            let (_, text) = lists
                .iter()
                .find(|(name, _)| *name == list)
                .unwrap_or_else(|| panic!("{file}: {list} is not published"));
            assert!(
                text.contains(&format!("<gml:name>{code}</gml:name>")),
                "{file}: {code} is not a code of {list}"
            );
        }
    }
}

/// LOD0 stays in the Dm attribute, and the converter writes none of its own.
#[test]
fn lod0_stays_in_the_dm_attribute() {
    for file in all() {
        let output = convert(file);
        assert!(!output.contains("<core:lod0"), "{file}");
        assert_eq!(
            count(&output, "<urc:lod0Geometry>"),
            count(&source(file), "<uro:lod0Geometry>"),
            "{file}"
        );
    }
}

/// Every texture and material link names a surface the output still holds.
#[test]
fn every_texture_link_resolves() {
    for file in all() {
        let output = convert(file);
        let ids: HashSet<&str> = attribute_values(&output, "gml:id").into_iter().collect();
        let targets = element_texts(&output, "app:target");
        assert!(!targets.is_empty(), "{file}");
        for target in targets {
            let target = target.trim_start_matches('#');
            assert!(
                ids.contains(target),
                "{file}: app:target {target} names no surface"
            );
        }
    }
}
