//! Structural rewrites that are not specific to one thematic module.
//!
//! Like [`crate::bldg`], this runs *after* [`crate::transform::rename`] and so
//! speaks CityGML 3.0 names throughout.

use crate::error::Result;
use crate::profile::Rules;
use crate::report::Warnings;
use crate::xml::{Element, Name, Node};

/// CityGML 2.0 generic-attribute properties and the 3.0 data type each one
/// becomes.
///
/// 2.0 wrote the attribute as a property with a `name` XML attribute
/// (`<gen:stringAttribute name="x">`). 3.0 wraps a data type instead, and the
/// name is an element: `StringAttribute` has `name [1..1]` and `value [1..1]`.
const GENERIC_ATTRIBUTES: &[(&str, &str)] = &[
    ("stringAttribute", "StringAttribute"),
    ("intAttribute", "IntAttribute"),
    ("doubleAttribute", "DoubleAttribute"),
    ("dateAttribute", "DateAttribute"),
    ("uriAttribute", "UriAttribute"),
    ("measureAttribute", "MeasureAttribute"),
    ("genericAttributeSet", "GenericAttributeSet"),
];

/// Lifespan properties that were `xs:date` in CityGML 2.0 and are `DateTime` in
/// 3.0.
const LIFESPAN_DATES: &[&str] = &["creationDate", "terminationDate"];

#[derive(Debug, Clone)]
pub struct CommonRewrite {
    core: String,
    generics: String,
    rules: Rules,
}

impl CommonRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        Ok(CommonRewrite {
            core: rules.output_ns("core")?.to_owned(),
            generics: rules.output_ns("gen")?.to_owned(),
            rules: rules.clone(),
        })
    }

    /// Rewrites `el` and its descendants in place.
    pub fn apply(&self, el: &mut Element, warnings: &mut Warnings) {
        // A generic attribute nested in a set hangs off the set's own
        // `gen:genericAttribute` role, not the city object's `core:` one.
        let wrapper_ns = if el.is(&self.generics, "GenericAttributeSet") {
            &self.generics
        } else {
            &self.core
        };
        let wrapper = Name::qualified(wrapper_ns.clone(), "genericAttribute");

        let children = std::mem::take(&mut el.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            match child {
                Node::Element(child) => self.rewrite(child, &wrapper, &mut out, warnings),
                other => out.push(other),
            }
        }
        el.children = out;

        // Elements the profile flags as needing a human decision survive
        // untouched; say so rather than let them look converted.
        if let Some(note) = self.rules.review_note(&el.name) {
            warnings.add(format!(
                "{} was left unchanged: {note}",
                self.rules.display_name(&el.name)
            ));
        }

        for child in el.elements_mut() {
            self.apply(child, warnings);
        }
    }

    fn rewrite(
        &self,
        child: Element,
        wrapper: &Name,
        out: &mut Vec<Node>,
        warnings: &mut Warnings,
    ) {
        if child.name.in_ns(&self.generics) {
            if let Some((_, data_type)) = GENERIC_ATTRIBUTES
                .iter()
                .find(|(property, _)| *property == child.name.local)
            {
                out.push(Node::Element(
                    self.generic_attribute(child, data_type, wrapper),
                ));
                return;
            }
        }
        if child.name.in_ns(&self.core) && LIFESPAN_DATES.contains(&child.name.local.as_str()) {
            out.push(Node::Element(self.lifespan_date(child, warnings)));
            return;
        }
        out.push(Node::Element(child));
    }

    /// `<gen:stringAttribute name="x">v</gen:stringAttribute>` ->
    /// `<core:genericAttribute><gen:StringAttribute><gen:name>x</gen:name>...`
    fn generic_attribute(&self, mut src: Element, data_type: &str, wrapper: &Name) -> Element {
        let name = src.take_attr(None, "name");
        src.name = Name::qualified(self.generics.clone(), data_type);

        if let Some(name) = name {
            src.children.insert(
                0,
                Node::Element(Element::with_text(
                    Name::qualified(self.generics.clone(), "name"),
                    name,
                )),
            );
        }

        let mut property = Element::new(wrapper.clone());
        property.push(src);
        property
    }

    /// Widens an `xs:date` to an `xs:dateTime`, which is what 3.0 asks for.
    fn lifespan_date(&self, src: Element, warnings: &mut Warnings) -> Element {
        let raw = src.text().trim().to_owned();
        if !is_date_only(&raw) {
            return src;
        }
        warnings.add(format!(
            "core:{} is an xs:date in CityGML 2.0 and a DateTime in 3.0, \
             so {raw} became {raw}T00:00:00",
            src.name.local
        ));
        Element::with_text(src.name.clone(), format!("{raw}T00:00:00"))
    }
}

/// True for exactly `YYYY-MM-DD`.
fn is_date_only(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && [0, 1, 2, 3, 5, 6, 8, 9]
            .iter()
            .all(|&i| bytes[i].is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_PROFILE;
    use crate::xml::ns;

    const GEN3: &str = "http://www.opengis.net/citygml/generics/3.0";

    fn rewrite() -> CommonRewrite {
        CommonRewrite::new(&Rules::from_toml(DEFAULT_PROFILE).unwrap()).unwrap()
    }

    fn in_building(child: Element) -> (Element, Warnings) {
        let mut building = Element::new(Name::qualified(ns::BUILDING_3, "Building"));
        building.push(child);
        let mut warnings = Warnings::new();
        rewrite().apply(&mut building, &mut warnings);
        (building, warnings)
    }

    #[test]
    fn string_attribute_becomes_a_wrapped_data_type() {
        let mut src = Element::with_text(Name::qualified(GEN3, "stringAttribute"), "");
        src.set_attr(Name::unqualified("name"), "風致地区");
        src.children.clear();
        src.push(Element::with_text(Name::qualified(GEN3, "value"), "第1種"));

        let (building, _) = in_building(src);

        let property = building
            .child(ns::CITYGML_3, "genericAttribute")
            .expect("wrapper");
        let object = property
            .child(GEN3, "StringAttribute")
            .expect("StringAttribute");
        assert_eq!(object.child(GEN3, "name").unwrap().text(), "風致地区");
        assert_eq!(object.child(GEN3, "value").unwrap().text(), "第1種");
        assert!(
            object.attr(None, "name").is_none(),
            "the XML attribute is consumed"
        );
    }

    #[test]
    fn each_generic_attribute_flavour_maps_to_its_data_type() {
        for (property, data_type) in GENERIC_ATTRIBUTES {
            let src = Element::new(Name::qualified(GEN3, *property));
            let (building, _) = in_building(src);
            let wrapper = building
                .child(ns::CITYGML_3, "genericAttribute")
                .expect(property);
            assert!(
                wrapper.child(GEN3, data_type).is_some(),
                "{property} -> {data_type}"
            );
        }
    }

    #[test]
    fn a_nested_attribute_hangs_off_the_sets_own_role() {
        let mut inner = Element::new(Name::qualified(GEN3, "stringAttribute"));
        inner.set_attr(Name::unqualified("name"), "inner");
        let mut set = Element::new(Name::qualified(GEN3, "genericAttributeSet"));
        set.set_attr(Name::unqualified("name"), "outer");
        set.push(inner);

        let (building, _) = in_building(set);

        let outer = building.child(ns::CITYGML_3, "genericAttribute").unwrap();
        let set = outer.child(GEN3, "GenericAttributeSet").unwrap();
        assert_eq!(set.child(GEN3, "name").unwrap().text(), "outer");
        // Inside a set the wrapper is gen:genericAttribute, not core:.
        let nested = set
            .child(GEN3, "genericAttribute")
            .expect("gen:genericAttribute");
        assert!(nested.child(GEN3, "StringAttribute").is_some());
        assert!(set.child(ns::CITYGML_3, "genericAttribute").is_none());
    }

    #[test]
    fn measure_attributes_keep_the_uom_on_their_value() {
        let mut value = Element::with_text(Name::qualified(GEN3, "value"), "3.5");
        value.set_attr(Name::unqualified("uom"), "m");
        let mut src = Element::new(Name::qualified(GEN3, "measureAttribute"));
        src.set_attr(Name::unqualified("name"), "eaveHeight");
        src.push(value);

        let (building, _) = in_building(src);

        let object = building
            .child(ns::CITYGML_3, "genericAttribute")
            .unwrap()
            .child(GEN3, "MeasureAttribute")
            .unwrap();
        assert_eq!(
            object.child(GEN3, "value").unwrap().attr(None, "uom"),
            Some("m")
        );
    }

    #[test]
    fn creation_date_widens_to_a_date_time() {
        let src = Element::with_text(Name::qualified(ns::CITYGML_3, "creationDate"), "2023-03-01");
        let (building, warnings) = in_building(src);
        assert_eq!(
            building
                .child(ns::CITYGML_3, "creationDate")
                .unwrap()
                .text(),
            "2023-03-01T00:00:00"
        );
        assert!(
            warnings
                .iter()
                .any(|(m, _)| m.contains("2023-03-01T00:00:00"))
        );
    }

    #[test]
    fn a_value_that_is_already_a_date_time_is_left_alone() {
        let src = Element::with_text(
            Name::qualified(ns::CITYGML_3, "creationDate"),
            "2023-03-01T09:00:00",
        );
        let (building, warnings) = in_building(src);
        assert_eq!(
            building
                .child(ns::CITYGML_3, "creationDate")
                .unwrap()
                .text(),
            "2023-03-01T09:00:00"
        );
        assert!(warnings.is_empty());
    }

    #[test]
    fn recognises_date_only_values() {
        assert!(is_date_only("2023-03-01"));
        assert!(!is_date_only("2023-03-01T00:00:00"));
        assert!(!is_date_only("2023-3-1"));
        assert!(!is_date_only(""));
    }
}
