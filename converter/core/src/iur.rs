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

/// i-UR elements that were a `gYear` or `gYearMonth` in 3.x and are a full
/// date in the 4.0 schemas. `urg` is deliberately absent: its statistical-grid
/// types keep `gYear`, and it is the only module that does.
const YEAR_TO_DATE: &[&str] = &[
    "acquisitionYear",
    "assessmentFiscalYear",
    "completionYear",
    "constructionStartYear",
    "creationDate",
    "enactmentFiscalYear",
    "expectedRenewalYearWithMeasures",
    "expirationFiscalYear",
    "fiscalYear",
    "fiscalYearForCountermeasures",
    "installationYear",
    "maintenanceFiscalYear",
    "maintenanceYear",
    "measurementYearMonth",
    "repairFiscalYear",
    "startFiscalYear",
    "surveyYear",
    "terminationDate",
    "updateDate",
    "year",
    "yearClosed",
    "yearOfConstruction",
    "yearOfDiversion",
    "yearOpened",
];

#[derive(Debug, Clone)]
pub struct IurRewrite {
    /// The i-UR namespaces whose properties are candidates for rehoming.
    modules: Vec<String>,
    /// The subset of [`IurRewrite::modules`] whose year-valued elements became
    /// dates in 4.0 — everything but `urg`.
    date_modules: Vec<String>,
    rules: Rules,
}

/// Why an i-UR element in a property position is not an ADE wrapper.
enum NotAHook {
    /// A plain attribute or a class element. Expected, and silent.
    NotAWrapper,
    /// Shaped like an ADE property, but the hook could not be worked out.
    Unresolved(String),
}

impl IurRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        let mut modules = Vec::new();
        let mut date_modules = Vec::new();
        for prefix in ["uro", "urc", "urf", "urg", "urt"] {
            if let Ok(uri) = rules.output_ns(prefix) {
                modules.push(uri.to_owned());
                if prefix != "urg" {
                    date_modules.push(uri.to_owned());
                }
            }
        }
        Ok(IurRewrite {
            modules,
            date_modules,
            rules: rules.clone(),
        })
    }

    /// Rewrites `el` and its descendants in place.
    pub fn apply(&self, el: &mut Element, warnings: &mut Warnings) {
        let children = std::mem::take(&mut el.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            match child {
                Node::Element(child) => {
                    let child = self.year_to_date(child, warnings);
                    out.push(Node::Element(self.rewrite(child, warnings)));
                }
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
        let hook = match self.hook_for(&property) {
            Ok(hook) => hook,
            // A plain attribute or a class element: nothing to rehome, and
            // nothing surprising about it.
            Err(NotAHook::NotAWrapper) => return property,
            // Shaped like an ADE property, but the hook could not be worked
            // out. The rename pass has already moved it into an i-UR 4.0
            // namespace, so passing it through quietly would leave an element
            // 4.0 does not declare looking converted.
            Err(NotAHook::Unresolved(why)) => {
                warnings.add(format!(
                    "{} carries an i-UR class but its CityGML 3.0 hook could not be \
                     determined ({why}); it was left as it is, which i-UR 4.0 may not \
                     declare -- check the result",
                    self.rules.display_name(&property.name),
                ));
                return property;
            }
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
    fn hook_for(&self, property: &Element) -> std::result::Result<Name, NotAHook> {
        // A property is lower-case, its class upper-case. Guards against a
        // single-valued attribute being mistaken for a wrapper.
        if property.name.local.starts_with(char::is_uppercase) {
            return Err(NotAHook::NotAWrapper);
        }
        let mut elements = property.elements();
        let Some(class) = elements.next() else {
            // No class to place. An `xlink:href` in place of one is still an
            // ADE property, just one written by reference; anything else is an
            // ordinary attribute holding a value.
            // Matched on the local name: `href` is `xlink:href` in practice,
            // but the binding is the document's to choose.
            return if property.attrs.iter().any(|a| a.name.local == "href") {
                Err(NotAHook::Unresolved(
                    "it carries a reference rather than a class".to_owned(),
                ))
            } else {
                Err(NotAHook::NotAWrapper)
            };
        };
        let class = class.name.clone();
        if elements.next().is_some() {
            let n = property.elements().count();
            return Err(NotAHook::Unresolved(format!(
                "it holds {n} elements, and an ADE property holds one class"
            )));
        }
        self.rules.ade_hook(&class).cloned().ok_or_else(|| {
            NotAHook::Unresolved(format!(
                "no [ade_hooks] entry for {}",
                self.rules.display_name(&class)
            ))
        })
    }

    /// Pads a 3.x year (`2021`) or year-month (`2021-04`) value to the full
    /// date the 4.0 schemas require, on the elements they retyped.
    fn year_to_date(&self, mut el: Element, warnings: &mut Warnings) -> Element {
        if !self.date_modules.iter().any(|uri| el.name.in_ns(uri))
            || !YEAR_TO_DATE.contains(&el.name.local.as_str())
        {
            return el;
        }
        let raw = el.text().trim().to_owned();
        let bytes = raw.as_bytes();
        let year_only = bytes.len() == 4 && bytes.iter().all(u8::is_ascii_digit);
        let year_month = bytes.len() == 7
            && bytes[4] == b'-'
            && [0, 1, 2, 3, 5, 6]
                .iter()
                .all(|&i| bytes[i].is_ascii_digit());
        let value = if year_only {
            format!("{raw}-01-01")
        } else if year_month {
            format!("{raw}-01")
        } else {
            return el;
        };
        warnings.add(format!(
            "{} held {raw} and i-UR 4.0 types it as a date, so it became {value}",
            self.rules.display_name(&el.name),
        ));
        el.children = vec![Node::Text(value)];
        el
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
    /// rehomed to a guess — and says so, because the rename pass has already
    /// moved it into a 4.0 namespace that may not declare it.
    #[test]
    fn an_unknown_class_is_left_alone_and_reported() {
        let src = wrapped("mysteryAttribute", "MysteryAttribute", URO);
        let (building, warnings) = in_building(src);
        assert!(building.child(URO, "mysteryAttribute").is_some());
        assert!(
            warnings
                .iter()
                .any(|(w, _)| w.contains("no [ade_hooks] entry")),
            "{warnings:?}"
        );
    }

    /// An ADE property written as an `xlink:href` reference carries no class to
    /// place, so the hook cannot be chosen. It passes through, reported.
    #[test]
    fn a_property_by_reference_is_reported() {
        let mut src = Element::new(Name::qualified(URO, "buildingIDAttribute"));
        src.set_attr(Name::unqualified("href"), "#shared");
        let (building, warnings) = in_building(src);
        assert!(building.child(URO, "buildingIDAttribute").is_some());
        assert!(
            warnings.iter().any(|(w, _)| w.contains("reference")),
            "{warnings:?}"
        );
    }

    /// Nor can it be chosen when the property holds more than one class.
    #[test]
    fn a_property_holding_two_classes_is_reported() {
        let mut src = Element::new(Name::qualified(URO, "buildingIDAttribute"));
        src.push(Element::new(Name::qualified(URO, "BuildingIDAttribute")));
        src.push(Element::new(Name::qualified(URO, "BuildingIDAttribute")));
        let (building, warnings) = in_building(src);
        assert!(building.child(URO, "buildingIDAttribute").is_some());
        assert!(
            warnings.iter().any(|(w, _)| w.contains("holds 2 elements")),
            "{warnings:?}"
        );
    }

    #[test]
    fn a_year_becomes_a_date_where_4_0_requires_one() {
        let src = Element::with_text(Name::qualified(URO, "surveyYear"), "2021");
        let (building, warnings) = in_building(src);
        let survey = building.child(URO, "surveyYear").unwrap();
        assert_eq!(survey.text(), "2021-01-01");
        assert!(
            warnings
                .iter()
                .any(|(m, _)| m.contains("became 2021-01-01"))
        );
    }

    #[test]
    fn a_year_month_becomes_a_date_where_4_0_requires_one() {
        let src = Element::with_text(Name::qualified(URO, "updateDate"), "2023-04");
        let (building, warnings) = in_building(src);
        assert_eq!(
            building.child(URO, "updateDate").unwrap().text(),
            "2023-04-01"
        );
        assert!(
            warnings
                .iter()
                .any(|(m, _)| m.contains("became 2023-04-01"))
        );
    }

    #[test]
    fn a_statistical_grid_year_stays_a_g_year() {
        const URG: &str = "https://www.geospatial.jp/iur/urg/4.0";
        let src = Element::with_text(Name::qualified(URG, "surveyYear"), "2021");
        let (building, warnings) = in_building(src);
        assert_eq!(building.child(URG, "surveyYear").unwrap().text(), "2021");
        assert!(warnings.is_empty(), "urg keeps gYear in 4.0: {warnings}");
    }

    #[test]
    fn a_full_date_passes_through_silently() {
        let src = Element::with_text(Name::qualified(URO, "surveyYear"), "2021-04-01");
        let (building, warnings) = in_building(src);
        assert_eq!(
            building.child(URO, "surveyYear").unwrap().text(),
            "2021-04-01"
        );
        assert!(warnings.is_empty(), "{warnings}");
    }
}
