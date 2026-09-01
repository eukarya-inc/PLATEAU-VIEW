//! The i-UR rewrites a rename table cannot express.
//!
//! Like [`crate::bldg`] this runs *after* [`crate::transform::rename`], so every
//! name here is already i-UR 4.0.
//!
//! CityGML 2.0 let an extension declare its own property to hang a class off,
//! so `uro:buildingIDAttribute` held a `uro:BuildingIDAttribute` and there was
//! one such property per host feature type. CityGML 3.0 declares a single
//! general hook on each host class instead, and the extension's class
//! substitutes into it. The wrapper is chosen by the class inside it rather
//! than by its own name, and the profile supplies only the class-to-hook
//! table.
//!
//! ```text
//! 2.0   <uro:buildingIDAttribute>   <uro:BuildingIDAttribute>…</…></…>
//! 3.0   <bldg:adeOfAbstractBuilding><uro:BuildingIDAttribute>…</…></…>
//! ```

use crate::error::Result;
use crate::profile::{QualityRules, Rules};
use crate::report::Warnings;
use crate::xml::{Element, Name, Node};

/// i-UR elements that were a `gYear` or `gYearMonth` in 3.x and are a full
/// date in the 4.0 schemas. `urg` is absent, since its statistical-grid types
/// keep `gYear`.
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
    /// dates in 4.0, meaning everything but `urg`.
    date_modules: Vec<String>,
    /// The child a data quality attribute must carry, and what to put in it.
    quality: QualityRules,
    rules: Rules,
}

/// Why an i-UR element in a property position is not an ADE wrapper.
enum NotAHook {
    /// A plain attribute or a class element, which is reported nowhere.
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
            quality: rules.quality().clone(),
            rules: rules.clone(),
        })
    }

    /// Rewrites `el` and its descendants in place.
    pub fn apply(&self, el: &mut Element, warnings: &mut Warnings) {
        self.required_quality_child(el, warnings);
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
        let hook = match self.hook_for(&property) {
            Ok(hook) => hook,
            Err(NotAHook::NotAWrapper) => return property,
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

    /// The CityGML property that should carry `property`'s single class
    /// child. An element whose own name is upper-case, one holding no class,
    /// or one holding several is a [`NotAHook`].
    fn hook_for(&self, property: &Element) -> std::result::Result<Name, NotAHook> {
        if property.name.local.starts_with(char::is_uppercase) {
            return Err(NotAHook::NotAWrapper);
        }
        let mut elements = property.elements();
        let Some(class) = elements.next() else {
            // Matched on the local name, since `href` is `xlink:href` in
            // practice but the binding is the document's to choose.
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

    /// Supplies the child a data quality attribute must carry when the source
    /// recorded none.
    ///
    /// i-UR records provenance per LOD and makes one of those children
    /// mandatory. A source that records it without an LOD leaves the slot
    /// empty, and the profile's `[quality]` value fills it.
    fn required_quality_child(&self, el: &mut Element, warnings: &mut Warnings) {
        let Some(child) = &self.quality.child else {
            return;
        };
        if !self.quality.classes.contains(&el.name) || el.elements().any(|e| e.name == *child) {
            return;
        }

        let mut supplied = Element::with_text(child.clone(), &self.quality.value);
        if let Some(code_space) = &self.quality.code_space {
            supplied.set_attr(Name::unqualified("codeSpace"), code_space.clone());
        }

        // The type is a sequence and the profile declares no order for i-UR
        // classes, so the child goes after the ones it must follow.
        let at = el
            .children
            .iter()
            .rposition(|n| matches!(n, Node::Element(e) if self.quality.after.contains(&e.name)))
            .map_or(0, |i| i + 1);
        el.children.insert(at, Node::Element(supplied));

        warnings.add(format!(
            "{} carried no {}, which i-UR 4.0 requires, so it was written as {} \
             (未作成); the source records provenance without an LOD, and no LOD \
             can be read off one value",
            self.rules.display_name(&el.name),
            self.rules.display_name(child),
            self.quality.value,
        ));
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

    /// A class the profile has no hook for stays where it is, and is
    /// reported.
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

    /// An ADE property written as an `xlink:href` reference carries no class
    /// to place, so it passes through and is reported.
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
}
