//! End-to-end conversion of an LOD4 building, covering interior rooms,
//! installations, furniture and per-LOD quality descriptors. The fixture
//! package is LOD1 and cannot exercise this.

use plateau_converter_core::PROFILES;
use plateau_converter_core::convert::{Converter, Options, convert_to_string};
use plateau_converter_core::profile::{Lod4Fallback, Profile, Rules};

const LOD4_BUILDING: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
  xmlns:bldg="http://www.opengis.net/citygml/building/2.0"
  xmlns:gml="http://www.opengis.net/gml"
  xmlns:uro="https://www.geospatial.jp/iur/uro/3.1">
<core:cityObjectMember>
<bldg:Building gml:id="b1">
  <bldg:lod3Solid><gml:Solid gml:id="ext"/></bldg:lod3Solid>
  <bldg:lod4Solid><gml:Solid/></bldg:lod4Solid>
  <bldg:boundedBy><bldg:WallSurface gml:id="w1">
    <bldg:lod4MultiSurface><gml:MultiSurface/></bldg:lod4MultiSurface>
    <bldg:opening><bldg:Door gml:id="d1"><bldg:lod4MultiSurface><gml:MultiSurface/></bldg:lod4MultiSurface></bldg:Door></bldg:opening>
  </bldg:WallSurface></bldg:boundedBy>
  <bldg:interiorBuildingInstallation><bldg:IntBuildingInstallation gml:id="i1">
    <bldg:lod4Geometry><gml:MultiSurface/></bldg:lod4Geometry>
  </bldg:IntBuildingInstallation></bldg:interiorBuildingInstallation>
  <bldg:interiorRoom><bldg:Room gml:id="r1">
    <bldg:lod4Solid><gml:Solid/></bldg:lod4Solid>
    <bldg:boundedBy><bldg:FloorSurface gml:id="f1"><bldg:lod4MultiSurface><gml:MultiSurface/></bldg:lod4MultiSurface></bldg:FloorSurface></bldg:boundedBy>
    <bldg:interiorFurniture><bldg:BuildingFurniture gml:id="fu1"><bldg:lod4Geometry><gml:Solid/></bldg:lod4Geometry></bldg:BuildingFurniture></bldg:interiorFurniture>
  </bldg:Room></bldg:interiorRoom>
  <uro:bldgDataQualityAttribute><uro:DataQualityAttribute>
    <uro:geometrySrcDescLod3>3</uro:geometrySrcDescLod3>
    <uro:geometrySrcDescLod4>CODE</uro:geometrySrcDescLod4>
    <uro:srcScaleLod4>1</uro:srcScaleLod4>
    <uro:lodType>4.1</uro:lodType>
  </uro:DataQualityAttribute></uro:bldgDataQualityAttribute>
</bldg:Building>
</core:cityObjectMember>
</core:CityModel>
"#;

/// The 3.1 profile with `CODE` placed under the given LOD list.
fn rules(code_under: Option<&str>) -> Rules {
    let (_, toml) = PROFILES
        .iter()
        .find(|(name, _)| *name == "iur-3.1-to-4.0")
        .unwrap();
    let mut profile = Profile::load(toml).unwrap();
    // The shipped tables are replaced, so the test says where CODE goes.
    let policy = &mut profile.lod4;
    policy.lod2.clear();
    policy.lod3.clear();
    match code_under {
        Some("lod2") => policy.lod2 = vec!["CODE".into()],
        Some("lod3") => policy.lod3 = vec!["CODE".into()],
        _ => {}
    }

    Rules::compile(&profile).unwrap()
}

fn convert(code_under: Option<&str>, fallback: Option<Lod4Fallback>) -> (String, Vec<String>) {
    let options = Options {
        lod4_fallback: fallback,
        ..Options::default()
    };
    let converter = Converter::new(rules(code_under), options).unwrap();
    let (output, report) = convert_to_string(&converter, "lod4", LOD4_BUILDING).unwrap();
    let warnings = report.warnings.iter().map(|(m, _)| m.to_owned()).collect();
    (output, warnings)
}

#[test]
fn no_lod4_slot_survives_whatever_the_decision() {
    for (code_under, fallback) in [
        (Some("lod2"), None),
        (Some("lod3"), None),
        (None, None),
        (None, Some(Lod4Fallback::Lod2)),
        (None, Some(Lod4Fallback::Drop)),
    ] {
        let (output, _) = convert(code_under, fallback);
        assert!(
            !output.contains("lod4") && !output.contains("Lod4") && !output.contains(">4.1<"),
            "{code_under:?}/{fallback:?} left LOD4 in the output:\n{output}"
        );
    }
}

/// The shipped profile sends 500 (BIM/CAD/drawings) to interior LOD3 and the
/// survey codes to interior LOD2. The 2.0 installation properties come out as
/// bldg:buildingInstallation carrying con:relationToConstruction.
#[test]
fn the_shipped_profile_decides_by_measurement_method() {
    let (_, toml) = PROFILES
        .iter()
        .find(|(name, _)| *name == "iur-3.1-to-4.0")
        .unwrap();
    let converter = Converter::new(Rules::from_toml(toml).unwrap(), Options::default()).unwrap();

    for (code, room_solid) in [("500", "<core:lod3Solid>"), ("000", "<core:lod2Solid>")] {
        let source = LOD4_BUILDING.replace(">CODE<", &format!(">{code}<"));
        let (output, report) = convert_to_string(&converter, "lod4", &source).unwrap();
        let warnings: Vec<String> = report.warnings.iter().map(|(m, _)| m.to_owned()).collect();
        assert!(output.contains(room_solid), "{code}: {warnings:?}");
        assert!(
            !warnings.iter().any(|w| w.contains("neither lod2 nor lod3")),
            "{code} is listed: {warnings:?}"
        );
        assert!(
            output.contains("<con:relationToConstruction>inside</con:relationToConstruction>"),
            "the interior installation records its placement"
        );
        assert!(output.contains("<bldg:buildingInstallation>"));
        assert!(!output.contains("interiorBuildingInstallation"));
    }
}

#[test]
fn the_building_pass_never_invents_lod4() {
    let (output, warnings) = convert(None, None);
    assert!(!output.contains("core:lod4"));
    assert!(warnings.iter().all(|w| !w.contains("core:lod4")));
}
