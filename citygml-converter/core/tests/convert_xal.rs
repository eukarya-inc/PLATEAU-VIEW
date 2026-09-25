//! End-to-end conversion of xAL 2.0 addresses into xAL 3.0 through a real
//! profile.

use plateau_converter_core::convert::{Converter, Options, convert_to_string};
use plateau_converter_core::profile::Rules;
use plateau_converter_core::{DEFAULT_PROFILE, PROFILES};

/// The structured-address shape 2.0-era packages carry, namely: nested
/// Country > Locality > DependentLocality, typed name elements, and a
/// number.
const ADDRESSED: &str = r##"<core:CityModel
    xmlns:core="http://www.opengis.net/citygml/2.0"
    xmlns:bldg="http://www.opengis.net/citygml/building/2.0"
    xmlns:gml="http://www.opengis.net/gml"
    xmlns:xAL="urn:oasis:names:tc:ciq:xsdschema:xAL:2.0">
    <core:cityObjectMember>
        <bldg:Building gml:id="b1">
            <bldg:address>
                <core:Address gml:id="adr1">
                    <core:xalAddress>
                        <xAL:AddressDetails>
                            <xAL:Country>
                                <xAL:CountryName>日本</xAL:CountryName>
                                <xAL:Locality>
                                    <xAL:LocalityName Type="prefecture">東京都</xAL:LocalityName>
                                    <xAL:LocalityName Type="city">渋谷区</xAL:LocalityName>
                                    <xAL:DependentLocality Type="district">
                                        <xAL:DependentLocalityName>神宮前</xAL:DependentLocalityName>
                                        <xAL:DependentLocalityNumber>2-2-3</xAL:DependentLocalityNumber>
                                    </xAL:DependentLocality>
                                </xAL:Locality>
                            </xAL:Country>
                        </xAL:AddressDetails>
                    </core:xalAddress>
                </core:Address>
            </bldg:address>
        </bldg:Building>
    </core:cityObjectMember>
</core:CityModel>"##;

fn convert(source: &str) -> (String, plateau_converter_core::report::FileReport) {
    let rules = Rules::from_toml(PROFILES[1].1).unwrap();
    let converter = Converter::new(rules, Options::default()).unwrap();
    convert_to_string(&converter, "t", source).unwrap()
}

#[test]
fn an_address_becomes_an_xal_3_0_address() {
    let (out, _) = convert(ADDRESSED);
    assert!(out.contains("urn:oasis:names:tc:ciq:xal:3"), "{out}");
    assert!(out.contains("<xAL:Address>"), "{out}");
    assert!(!out.contains("AddressDetails"), "{out}");
    assert!(!out.contains("LocalityName"), "{out}");
}

#[test]
fn nesting_flattens_and_names_become_name_elements() {
    let (out, _) = convert(ADDRESSED);
    let country = out.find("<xAL:Country>").unwrap();
    let locality = out.find("<xAL:Locality>").unwrap();
    let country_end = out.find("</xAL:Country>").unwrap();
    assert!(
        country < country_end && country_end < locality,
        "Locality is a sibling after Country, not nested: {out}"
    );
    assert!(
        out.contains(r#"<xAL:NameElement xAL:NameType="Name">日本</xAL:NameElement>"#),
        "{out}"
    );
    assert!(
        out.contains(r#"<xAL:NameElement xAL:NameType="Name">東京都</xAL:NameElement>"#),
        "{out}"
    );
    assert!(
        out.contains(r#"<xAL:NameElement xAL:NameType="Number">2-2-3</xAL:NameElement>"#),
        "the dependent locality number keeps its number-ness: {out}"
    );
    assert!(out.contains("<xAL:SubLocality>"), "{out}");
}

#[test]
fn untranslatable_type_attributes_are_dropped_and_reported() {
    let (out, report) = convert(ADDRESSED);
    assert!(!out.contains("prefecture"), "{out}");
    assert!(
        report
            .warnings
            .iter()
            .any(|(m, _)| m.contains(r#"Type="prefecture""#) && m.contains("dropped")),
        "{}",
        report.warnings
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|(m, _)| m.contains("became an xAL 3.0 Address")),
        "{}",
        report.warnings
    );
}

#[test]
fn an_element_without_a_counterpart_survives_as_free_text() {
    let src = ADDRESSED.replace(
        "<xAL:CountryName>日本</xAL:CountryName>",
        "<xAL:CountryName>日本</xAL:CountryName>\
         <xAL:Thoroughfare><xAL:ThoroughfareName>青山通り</xAL:ThoroughfareName></xAL:Thoroughfare>\
         <xAL:Firm><xAL:FirmName>某社</xAL:FirmName></xAL:Firm>",
    );
    let (out, report) = convert(&src);
    assert!(
        out.contains(r#"<xAL:NameElement xAL:NameType="NameOnly">青山通り</xAL:NameElement>"#),
        "a thoroughfare has a real counterpart: {out}"
    );
    assert!(
        out.contains("<xAL:FreeTextAddress>")
            && out.contains("<xAL:AddressLine>某社</xAL:AddressLine>"),
        "firm text survives as free text: {out}"
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|(m, _)| m.contains("Firm has no xAL 3.0 counterpart")),
        "{}",
        report.warnings
    );
}

#[test]
fn converting_twice_is_a_no_op() {
    let rules = Rules::from_toml(DEFAULT_PROFILE).unwrap();
    let converter = Converter::new(rules, Options::default()).unwrap();
    let (once, _) = convert_to_string(&converter, "t", ADDRESSED).unwrap();
    let (twice, report) = convert_to_string(&converter, "t", &once).unwrap();
    assert_eq!(once, twice);
    assert!(
        !report
            .warnings
            .iter()
            .any(|(m, _)| m.contains("xAL 3.0 Address")),
        "already-3.0 content converts silently: {}",
        report.warnings
    );
}
