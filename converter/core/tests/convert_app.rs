//! End-to-end conversion of appearance content: the attachment properties, the
//! surface-data role, and the texture-to-surface binding. The fixture package
//! is untextured and cannot exercise this.

use plateau_converter_core::PROFILES;
use plateau_converter_core::convert::{Converter, Options, convert_to_string};
use plateau_converter_core::profile::Rules;

const TEXTURED: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
  xmlns:bldg="http://www.opengis.net/citygml/building/2.0"
  xmlns:app="http://www.opengis.net/citygml/appearance/2.0"
  xmlns:gml="http://www.opengis.net/gml">
<core:cityObjectMember>
<bldg:Building gml:id="b1">
  <bldg:lod2Solid><gml:Solid gml:id="s1"/></bldg:lod2Solid>
  <app:appearance>
    <app:Appearance gml:id="ap1">
      <app:theme>rgbTexture</app:theme>
      <app:surfaceDataMember>
        <app:ParameterizedTexture gml:id="tx1">
          <app:imageURI>52382287_bldg_6697_appearance/hnap0001.jpg</app:imageURI>
          <app:mimeType>image/jpg</app:mimeType>
          <app:target uri="#poly1">
            <app:TexCoordList>
              <app:textureCoordinates ring="#ring1">0.1 0.1 0.9 0.1 0.9 0.9 0.1 0.1</app:textureCoordinates>
            </app:TexCoordList>
          </app:target>
          <app:target uri="#poly2">
            <app:TexCoordList>
              <app:textureCoordinates ring="#ring2">0 0 1 0 1 1 0 0</app:textureCoordinates>
            </app:TexCoordList>
          </app:target>
        </app:ParameterizedTexture>
      </app:surfaceDataMember>
      <app:surfaceDataMember>
        <app:X3DMaterial gml:id="mt1">
          <app:diffuseColor>0.8 0.8 0.8</app:diffuseColor>
          <app:target>#poly3</app:target>
        </app:X3DMaterial>
      </app:surfaceDataMember>
    </app:Appearance>
  </app:appearance>
</bldg:Building>
</core:cityObjectMember>
<app:appearanceMember>
  <app:Appearance gml:id="ap2">
    <app:theme>rgbTexture</app:theme>
  </app:Appearance>
</app:appearanceMember>
</core:CityModel>
"##;

fn convert() -> (String, Vec<String>) {
    let (_, toml) = PROFILES
        .iter()
        .find(|(name, _)| *name == "iur-3.1-to-4.0")
        .unwrap();
    let converter = Converter::new(Rules::from_toml(toml).unwrap(), Options::default()).unwrap();
    let (output, report) = convert_to_string(&converter, "textured", TEXTURED).unwrap();
    let warnings = report.warnings.iter().map(|(m, _)| m.to_owned()).collect();
    (output, warnings)
}

#[test]
fn the_attachment_properties_move_to_core() {
    let (output, _) = convert();
    assert!(output.contains("<core:appearance>"));
    assert!(!output.contains("<app:appearance>"));
    assert!(output.contains("<core:appearanceMember>"));
    assert!(!output.contains("<app:appearanceMember>"));
    assert!(output.contains(r#"xmlns:app="http://www.opengis.net/citygml/appearance/3.0""#));
}

#[test]
fn surface_data_members_take_the_3_0_role_name() {
    let (output, _) = convert();
    assert!(output.contains("<app:surfaceData>"));
    assert!(!output.contains("surfaceDataMember"));
}

#[test]
fn texture_targets_become_association_objects() {
    let (output, warnings) = convert();
    assert!(
        !output.contains("target uri="),
        "the 2.0 shape must not survive"
    );
    assert!(output.contains(r#"<app:TextureAssociation gml:id="b1_1">"#));
    assert!(output.contains(r#"<app:TextureAssociation gml:id="b1_2">"#));
    assert!(output.contains("<app:target>#poly1</app:target>"));
    assert!(output.contains("<app:target>#poly2</app:target>"));
    assert_eq!(
        output.matches("<app:textureParameterization>").count(),
        4,
        "one property per target, one inside each association"
    );
    assert!(warnings.iter().any(|w| w.contains("TextureAssociation")));
}

#[test]
fn rings_move_from_attribute_to_parallel_elements() {
    let (output, _) = convert();
    assert!(!output.contains("ring="));
    assert!(output.contains("<app:ring>#ring1</app:ring>"));
    assert!(output.contains("<app:ring>#ring2</app:ring>"));
    assert!(
        output.contains(
            "<app:textureCoordinates>0.1 0.1 0.9 0.1 0.9 0.9 0.1 0.1</app:textureCoordinates>"
        ),
        "coordinate values are carried through verbatim"
    );
}

#[test]
fn materials_and_untouched_content_survive() {
    let (output, _) = convert();
    assert!(output.contains("<app:X3DMaterial"));
    assert!(output.contains("<app:target>#poly3</app:target>"));
    assert!(output.contains("<app:theme>rgbTexture</app:theme>"));
    assert!(
        output.contains("<app:imageURI>52382287_bldg_6697_appearance/hnap0001.jpg</app:imageURI>")
    );
}
