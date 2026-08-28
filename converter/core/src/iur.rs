//! The i-UR rewrites a rename table cannot express.
//!
//! Like [`crate::bldg`] this runs *after* [`crate::transform::rename`], so every
//! name here is already i-UR 4.0.
//!
//! CityGML 2.0 let an extension declare its own property to hang a class off:
//! `uro:buildingIDAttribute` held a `uro:BuildingIDAttribute`, and there was one
//! such property per host feature type. CityGML 3.0 declares a single general
//! hook on each host class instead, and the extension's class substitutes into
//! it. The wrapper is therefore chosen by the class inside it rather than by its
//! own name, which is why this is a rewrite and the profile only supplies the
//! table.
//!
//! ```text
//! 2.0   <uro:buildingIDAttribute>   <uro:BuildingIDAttribute>…</…></…>
//! 3.0   <bldg:adeOfAbstractBuilding><uro:BuildingIDAttribute>…</…></…>
//! ```

use crate::error::Result;
use crate::profile::Rules;
use crate::report::Warnings;
use crate::xml::{Element, Name, Node};

#[derive(Debug, Clone)]
pub struct IurRewrite {
    /// The i-UR namespaces whose properties are candidates for rehoming.
    modules: Vec<String>,
    rules: Rules,
}

impl IurRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        let mut modules = Vec::new();
        for prefix in ["uro", "urc", "urf", "urg", "urt"] {
            if let Ok(uri) = rules.output_ns(prefix) {
                modules.push(uri.to_owned());
            }
        }
        Ok(IurRewrite {
            modules,
            rules: rules.clone(),
        })
    }

    /// Rewrites `el` and its descendants in place.
    pub fn apply(&self, el: &mut Element, warnings: &mut Warnings) {
        let children = std::mem::take(&mut el.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            match child {
                Node::Element(child) => out.push(Node::Element(self.rewrite(child, warnings))),
                other => out.push(other),
            }
        }
        el.children = out;

        for child in el.elements_mut() {
            self.apply(child, warnings);
        }
    }

    fn rewrite(&self, property: Element, warnings: &mut Warnings) -> Element {
        if !self.is_iur(&property.name) {
            return property;
        }
        // An ADE property holds exactly one class and nothing else. Anything
        // else is a plain attribute that happens to live in the same namespace.
        let Some(hook) = self.hook_for(&property) else {
            return property;
        };

        warnings.add(format!(
            "{} became {}: CityGML 3.0 replaces the per-feature-type extension \
             properties with one hook per host class, chosen by the class it carries",
            self.rules.display_name(&property.name),
            self.rules.display_name(&hook),
        ));

        let mut wrapper = Element::new(hook);
        wrapper.children = property.children;
        wrapper
    }

    /// The CityGML property that should carry `property`'s single class child.
    fn hook_for(&self, property: &Element) -> Option<Name> {
        let mut elements = property.elements();
        let class = elements.next()?;
        if elements.next().is_some() {
            return None;
        }
        // A property is lower-case, its class upper-case. Guards against a
        // single-valued attribute being mistaken for a wrapper.
        if property.name.local.starts_with(char::is_uppercase) {
            return None;
        }
        self.rules.ade_hook(&class.name).cloned()
    }

    fn is_iur(&self, name: &Name) -> bool {
        self.modules.iter().any(|uri| name.in_ns(uri))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PROFILES;
    use crate::xml::ns;

    const URO: &str = "https://www.geospatial.jp/iur/uro/4.0";
    const URC: &str = "https://www.geospatial.jp/iur/urc/4.0";

    fn rewrite() -> IurRewrite {
        IurRewrite::new(&Rules::from_toml(PROFILES[1].1).unwrap()).unwrap()
    }

    /// Wraps `child` in a 3.0 Building and applies the rewrite.
    fn in_building(child: Element) -> (Element, Warnings) {
        let mut building = Element::new(Name::qualified(ns::BUILDING_3, "Building"));
        building.push(child);
        let mut warnings = Warnings::new();
        rewrite().apply(&mut building, &mut warnings);
        (building, warnings)
    }

    fn wrapped(property: &str, class: &str, ns: &str) -> Element {
        let mut inner = Element::new(Name::qualified(ns, class));
        inner.push(Element::with_text(
            Name::qualified(ns, "buildingID"),
            "08220-bldg-1",
        ));
        let mut outer = Element::new(Name::qualified(URO, property));
        outer.push(inner);
        outer
    }

    #[test]
    fn a_building_property_becomes_the_building_ade_hook() {
        let src = wrapped("buildingIDAttribute", "BuildingIDAttribute", URO);
        let (building, warnings) = in_building(src);

        let hook = building
            .child(ns::BUILDING_3, "adeOfAbstractBuilding")
            .expect("bldg:adeOfAbstractBuilding");
        let class = hook
            .child(URO, "BuildingIDAttribute")
            .expect("the class is carried through unchanged");
        assert_eq!(
            class.child(URO, "buildingID").unwrap().text(),
            "08220-bldg-1"
        );
        assert!(!warnings.is_empty(), "a rehomed property must be reported");
    }

    /// A class that reaches the hook through an i-UR parent resolves to the
    /// general city-object hook, not a building one.
    #[test]
    fn a_quality_attribute_hangs_off_the_city_object_hook() {
        let mut inner = Element::new(Name::qualified(URC, "ExteriorDataQualityAttribute"));
        inner.push(Element::with_text(
            Name::qualified(URC, "lod1HeightType"),
            "2",
        ));
        let mut outer = Element::new(Name::qualified(URO, "bldgDataQualityAttribute"));
        outer.push(inner);

        let (building, _) = in_building(outer);
        let hook = building
            .child(ns::CITYGML_3, "adeOfAbstractCityObject")
            .expect("core:adeOfAbstractCityObject");
        assert!(hook.child(URC, "ExteriorDataQualityAttribute").is_some());
    }

    /// A plain attribute is not a wrapper, even though it is in the same
    /// namespace and holds one child.
    #[test]
    fn a_leaf_attribute_is_left_alone() {
        let src = Element::with_text(Name::qualified(URO, "buildingID"), "08220-bldg-1");
        let (building, warnings) = in_building(src);
        assert!(building.child(URO, "buildingID").is_some());
        assert!(warnings.is_empty());
    }

    /// A class the profile has no hook for stays where it is rather than being
    /// rehomed to a guess.
    #[test]
    fn an_unknown_class_is_left_alone() {
        let src = wrapped("mysteryAttribute", "MysteryAttribute", URO);
        let (building, warnings) = in_building(src);
        assert!(building.child(URO, "mysteryAttribute").is_some());
        assert!(warnings.is_empty());
    }
}
