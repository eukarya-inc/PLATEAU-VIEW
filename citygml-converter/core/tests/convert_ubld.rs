//! End-to-end conversion of real PLATEAU underground buildings (地下街).
//!
//! The fixture under `tests/fixtures/plateau-ubld` holds the following:
//!
//! * `53394611_ubld_6697_op.gml`: Tokyo Station B1 (i-UR 3.1), one
//!   underground building cut down to two rooms, with its floor group placed
//!   *before* the building and carrying no `grp:function`.
//! * `53394651_ubld_6697_op.gml`: Taito (i-UR 3.2), two underground
//!   buildings of two rooms each, with their floor groups placed *after* the
//!   buildings and carrying `grp:class` 2000 and `grp:function` 1030.
//!
//! Both record LOD4 interiors classified by `lodType` 4.1 and cite the
//! building `lodType` list, as 2.0 data does for every building type.

use std::collections::HashSet;
use std::path::Path;

use plateau_converter_core::convert::{Converter, Options, convert_to_string};
use plateau_converter_core::profile::Rules;
use plateau_converter_core::report::FileReport;
use plateau_converter_core::xml::{self, Chunk, Element, Reader, ns};
use plateau_converter_core::{PROFILES, detect};

const TOKYO: &str = "53394611_ubld_6697_op.gml";
const TAITO: &str = "53394651_ubld_6697_op.gml";

const URO4: &str = "https://www.geospatial.jp/iur/uro/4.0";
const GRP3: &str = "http://www.opengis.net/citygml/cityobjectgroup/3.0";

fn source(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/plateau-ubld/udx/ubld")
        .join(file);
    xml::read_to_string(&path).unwrap()
}

fn convert(file: &str) -> (String, FileReport) {
    let source = source(file);
    let declared = xml::root_namespaces(&source);
    let candidates: Vec<Rules> = PROFILES
        .iter()
        .map(|(_, toml)| Rules::from_toml(toml).unwrap())
        .collect();
    let found = detect::select(&candidates, &declared).expect("the fixture must match a profile");
    let converter = Converter::new(candidates[found.index].clone(), Options::default()).unwrap();
    convert_to_string(&converter, file, &source).unwrap()
}

/// The features of a converted document, unwrapped from their members.
fn features(output: &str) -> Vec<Element> {
    let mut reader = Reader::new("output", output);
    let mut out = Vec::new();
    while let Some(chunk) = reader.next_chunk().unwrap() {
        if let Chunk::Member(member) = chunk {
            out.extend(member.elements().next().cloned());
        }
    }
    out
}

fn descendants<'e>(el: &'e Element, out: &mut Vec<&'e Element>) {
    for child in el.elements() {
        out.push(child);
        descendants(child, out);
    }
}

fn all(el: &Element, local: &str) -> Vec<Element> {
    let mut found = Vec::new();
    descendants(el, &mut found);
    found
        .into_iter()
        .filter(|e| e.name.local == local)
        .cloned()
        .collect()
}

fn id(el: &Element) -> &str {
    el.attr(Some(ns::GML_32), "id").unwrap()
}

fn warned(report: &FileReport, text: &str) -> usize {
    report
        .warnings
        .iter()
        .filter(|(m, _)| m.contains(text))
        .map(|(_, n)| n)
        .sum()
}

/// The `(gml:id, parent href, member hrefs)` of every group in a source file.
fn source_groups(file: &str) -> Vec<(String, String, Vec<String>)> {
    let source = source(file);
    let mut reader = Reader::new(file, &source);
    let mut out = Vec::new();
    while let Some(chunk) = reader.next_chunk().unwrap() {
        let Chunk::Member(member) = chunk else {
            continue;
        };
        let Some(group) = member
            .elements()
            .next()
            .filter(|f| f.name.local == "CityObjectGroup")
        else {
            continue;
        };
        let href = |e: &Element| e.attr(Some(ns::XLINK), "href").unwrap().to_owned();
        let parent = group
            .elements()
            .find(|e| e.name.local == "parent")
            .map(href)
            .unwrap();
        let members = group
            .elements()
            .filter(|e| e.name.local == "groupMember")
            .map(href)
            .collect();
        let group_id = group.attr(Some(ns::GML_31), "id").unwrap().to_owned();
        out.push((group_id, parent, members));
    }
    out
}

#[test]
fn every_floor_group_becomes_a_storey_in_its_underground_building() {
    for file in [TOKYO, TAITO] {
        let groups = source_groups(file);
        assert!(!groups.is_empty(), "{file} has no floor group to test");
        let (output, report) = convert(file);
        let features = features(&output);

        assert!(
            features.iter().all(|f| !f.name.in_ns(GRP3)),
            "{file} still writes a group"
        );
        assert!(!output.contains("cityobjectgroup/2.0"), "{file}");

        for (group_id, parent, members) in &groups {
            let building = features
                .iter()
                .find(|f| f.is(URO4, "UndergroundBuilding") && format!("#{}", id(f)) == *parent)
                .unwrap_or_else(|| panic!("{file}: no building {parent}"));
            let storey = building
                .elements()
                .filter(|e| e.is(ns::BUILDING_3, "buildingSubdivision"))
                .filter_map(|s| s.child(ns::BUILDING_3, "Storey"))
                .find(|s| id(s) == group_id)
                .unwrap_or_else(|| panic!("{file}: {group_id} is not a storey of {parent}"));

            let rooms: Vec<_> = storey
                .elements()
                .filter(|e| e.is(ns::BUILDING_3, "buildingRoom"))
                .map(|e| e.attr(Some(ns::XLINK), "href").unwrap().to_owned())
                .collect();
            assert_eq!(&rooms, members, "{file}: {group_id}'s rooms");
            assert!(storey.child(ns::GML_32, "name").is_some(), "{file}");
            assert!(storey.elements().all(|e| !e.name.in_ns(GRP3)), "{file}");

            let room_ids: HashSet<_> = all(building, "BuildingRoom")
                .iter()
                .map(|r| format!("#{}", id(r)))
                .collect();
            for room in &rooms {
                assert!(room_ids.contains(room), "{file}: {room} does not resolve");
            }
        }
        assert_eq!(
            warned(
                &report,
                "became a bldg:Storey inside its uro:UndergroundBuilding"
            ),
            groups.len(),
            "{file}"
        );
        assert_eq!(warned(&report, "has no bldg:Storey"), 0, "{file}");
    }
}

#[test]
fn the_group_classification_is_dropped_and_reported() {
    let (output, report) = convert(TAITO);
    assert!(!output.contains("CityObjectGroup_"));
    assert_eq!(warned(&report, "grp:class 2000 was dropped"), 2);
    assert_eq!(warned(&report, "grp:function 1030 was dropped"), 2);

    let (_, report) = convert(TOKYO);
    assert_eq!(warned(&report, "was dropped from a floor"), 0);
}

#[test]
fn lod_type_cites_the_underground_building_list() {
    for file in [TOKYO, TAITO] {
        let (output, report) = convert(file);
        let lod_types: Vec<_> = features(&output)
            .iter()
            .flat_map(|f| all(f, "lodType"))
            .collect();
        assert!(!lod_types.is_empty(), "{file}");
        for lod_type in lod_types {
            assert_eq!(lod_type.text(), "2.1_interior", "{file}");
            assert_eq!(
                lod_type.attr(None, "codeSpace"),
                Some("../../codelists/UndergroundBuilding_lodType.xml"),
                "{file}"
            );
        }
        assert!(!output.contains("/Building_lodType.xml"), "{file}");
        assert!(warned(&report, "became UndergroundBuilding_lodType.xml") > 0);
    }
}

#[test]
fn no_lod4_survives_and_ids_stay_unique() {
    for file in [TOKYO, TAITO] {
        let (output, _) = convert(file);
        for stale in ["<bldg:lod4", "Lod4", "<bldg:interiorRoom", "<bldg:Room "] {
            assert!(!output.contains(stale), "{file} still holds {stale}");
        }
        let mut seen = HashSet::new();
        for feature in features(&output) {
            let mut found = vec![&feature];
            descendants(&feature, &mut found);
            for el in found {
                if let Some(id) = el.attr(Some(ns::GML_32), "id") {
                    assert!(seen.insert(id.to_owned()), "{file}: gml:id {id} twice");
                }
            }
        }
    }
}

#[test]
fn coordinates_pass_through_unchanged() {
    for file in [TOKYO, TAITO] {
        let source = source(file);
        let (output, _) = convert(file);
        let positions = |text: &str| -> Vec<String> {
            text.split("<gml:posList")
                .skip(1)
                .map(|s| {
                    let s = &s[s.find('>').unwrap() + 1..];
                    s[..s.find('<').unwrap()].to_owned()
                })
                .collect()
        };
        let before = positions(&source);
        assert!(!before.is_empty());
        let after: HashSet<_> = positions(&output).into_iter().collect();
        for list in &before {
            assert!(
                after.contains(list),
                "{file}: a posList changed or vanished"
            );
        }
    }
}
