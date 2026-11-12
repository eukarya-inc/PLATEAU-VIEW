//! Structural rewrites shared by every module deriving from the CityGML 3.0
//! construction module, meaning buildings, bridges, tunnels and other
//! constructions. The `lodNGeometry` rewrite acts on every type the table
//! lists, constructions or not.
//!
//! The properties acted on come from the profile's `[construction]` table, so
//! this module names no thematic module itself. It writes only `con` and
//! `core` names.
//!
//! Everything here speaks CityGML **3.0** names, since
//! [`crate::transform::rename`] has already run.

use crate::dropped::Dropped;
use crate::error::Result;
use crate::geometry::{slot, wrap_single};
use crate::profile::{ConstructionRules, Rules};
use crate::report::Warnings;
use crate::xml::{Element, Name, Node};

/// The construction rewrites, bound to one profile.
#[derive(Debug, Clone)]
pub struct ConstructionRewrite {
    core: String,
    con: String,
    gml: String,
    relation: Name,
    rules: ConstructionRules,
    display: Rules,
}

impl ConstructionRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        Ok(ConstructionRewrite {
            core: rules.output_ns("core")?.to_owned(),
            con: rules.output_ns("con")?.to_owned(),
            gml: rules.output_ns("gml")?.to_owned(),
            relation: Name::qualified(rules.output_ns("con")?, "relationToConstruction"),
            rules: rules.construction().clone(),
            display: rules.clone(),
        })
    }

    /// Rewrites `el` and its descendants in place. The `gml:id` of every
    /// element removed with a geometry 3.0 has no property for is added to
    /// `dropped`.
    pub fn apply(&self, el: &mut Element, dropped: &mut Dropped, warnings: &mut Warnings) {
        self.walk(el, dropped, warnings);
    }

    fn walk(&self, el: &mut Element, dropped: &mut Dropped, warnings: &mut Warnings) {
        let host = self.rules.lod_geometry.get(&el.name).map(String::as_str);
        let label = self.display.display_name(&el.name);
        let children = std::mem::take(&mut el.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            match child {
                Node::Element(child) => {
                    if let Some(child) = self.rewrite(child, host, &label, dropped, warnings) {
                        out.push(Node::Element(child));
                    }
                }
                other => out.push(other),
            }
        }
        el.children = out;

        for child in el.elements_mut() {
            self.walk(child, dropped, warnings);
        }
    }

    /// Rewrites one child of the element named `label`, or returns `None` to
    /// remove it. `host` is the namespace of that element's `lodNGeometry`
    /// children when it is one of the profile's geometry hosts.
    fn rewrite(
        &self,
        child: Element,
        host: Option<&str>,
        label: &str,
        dropped: &mut Dropped,
        warnings: &mut Warnings,
    ) -> Option<Element> {
        if let Some((to, relation)) = self.rules.installations.get(&child.name) {
            return Some(self.installation(child, to, relation, warnings));
        }
        if let Some(to) = self.rules.dates.get(&child.name) {
            return Some(self.date(child, to, warnings));
        }
        match lod_geometry(&child.name.local)
            .filter(|_| host.is_some_and(|ns| child.name.in_ns(ns)))
        {
            Some(lod) => self.lod_geometry(child, lod, label, dropped, warnings),
            None => Some(child),
        }
    }

    /// Retags an installation property to `to`, and gives each installation
    /// under it a `con:relationToConstruction` of `relation` unless it already
    /// has one.
    fn installation(
        &self,
        src: Element,
        to: &Name,
        relation: &str,
        warnings: &mut Warnings,
    ) -> Element {
        let from = self.display.display_name(&src.name);
        let mut prop = retag(src, to.clone());
        for installation in prop.elements_mut() {
            if installation
                .child(&self.con, "relationToConstruction")
                .is_none()
            {
                installation.children.insert(
                    0,
                    Node::Element(Element::with_text(self.relation.clone(), relation)),
                );
            }
        }
        warnings.add(format!(
            "{from} became {} with con:relationToConstruction={relation}: CityGML 3.0 \
             has one installation property and records the placement on the \
             installation",
            self.display.display_name(to)
        ));
        prop
    }

    /// Rewrites a `gYear` property into the `date` property `to`, padding a
    /// bare year to the first of January.
    fn date(&self, src: Element, to: &Name, warnings: &mut Warnings) -> Element {
        let raw = src.text().trim().to_owned();
        let value = if raw.len() == 4 && raw.bytes().all(|b| b.is_ascii_digit()) {
            warnings.add(format!(
                "{} held the year {raw}; {} is a date, so it became {raw}-01-01",
                self.display.display_name(&src.name),
                self.display.display_name(to)
            ));
            format!("{raw}-01-01")
        } else {
            raw
        };
        Element::with_text(to.clone(), value)
    }

    /// Rewrites a `lodNGeometry` property into the `core:lodN*` property for
    /// the geometry it holds, wrapping a single surface or curve in its
    /// multi-geometry. A geometry with no such property at that LOD is removed
    /// and its `gml:id`s are added to `dropped`.
    fn lod_geometry(
        &self,
        mut src: Element,
        lod: char,
        label: &str,
        dropped: &mut Dropped,
        warnings: &mut Warnings,
    ) -> Option<Element> {
        let from = self.display.display_name(&src.name);
        let inner = src
            .elements()
            .next()
            .filter(|g| g.name.in_ns(&self.gml))
            .map(|g| g.name.local.clone());
        let Some(target) = inner.as_deref().and_then(|kind| slot(kind, lod)) else {
            dropped.add(&src);
            warnings.add(format!(
                "{from} on the {label} held {}, for which CityGML 3.0 has no LOD{lod} \
                 property, and was dropped",
                inner.map_or("no geometry".to_owned(), |k| format!("a gml:{k}"))
            ));
            return None;
        };
        if target != "Solid" && target != "Point" {
            wrap_single(&mut src, target, &self.gml);
        }
        let local = format!("lod{lod}{target}");
        warnings.add(format!(
            "{from} became core:{local} based on the geometry it holds"
        ));
        Some(retag(src, Name::qualified(&self.core, local)))
    }
}

/// Replaces an element's name, keeping its attributes and children.
fn retag(mut el: Element, name: Name) -> Element {
    el.name = name;
    el
}

/// The LOD digit of a `lodNGeometry` local name, so `lod2Geometry` gives
/// `Some('2')`, for LOD 0 to 3 only.
fn lod_geometry(local: &str) -> Option<char> {
    let rest = local.strip_prefix("lod")?;
    let mut chars = rest.chars();
    let digit = chars.next().filter(|c| ('0'..='3').contains(c))?;
    (chars.as_str() == "Geometry").then_some(digit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_PROFILE;
    use crate::xml::ns;

    fn rewrite() -> ConstructionRewrite {
        ConstructionRewrite::new(&Rules::from_toml(DEFAULT_PROFILE).unwrap()).unwrap()
    }

    /// Wraps `child` in a 3.0 `parent` and applies the rewrites.
    fn under(parent: &str, child: Element) -> (Element, Warnings) {
        let mut el = Element::new(Name::qualified(ns::BUILDING_3, parent));
        el.push(child);
        let mut warnings = Warnings::new();
        rewrite().apply(&mut el, &mut Dropped::default(), &mut warnings);
        (el, warnings)
    }

    #[test]
    fn year_of_construction_becomes_a_date() {
        let src = Element::with_text(
            Name::qualified(ns::BUILDING_3, "yearOfConstruction"),
            "1998",
        );
        let (building, warnings) = under("Building", src);
        let date = building
            .child(ns::CONSTRUCTION_3, "dateOfConstruction")
            .unwrap();
        assert_eq!(date.text(), "1998-01-01");
        assert!(warnings.iter().any(|(m, _)| m.contains("1998-01-01")));
    }

    #[test]
    fn lod_geometry_picks_solid_or_multi_surface() {
        let mut solid = Element::new(Name::qualified(ns::BUILDING_3, "lod2Geometry"));
        solid.push(Element::new(Name::qualified(ns::GML_32, "Solid")));
        let (inst, _) = under("BuildingInstallation", solid);
        assert!(inst.child(ns::CITYGML_3, "lod2Solid").is_some());

        let mut surfaces = Element::new(Name::qualified(ns::BUILDING_3, "lod3Geometry"));
        surfaces.push(Element::new(Name::qualified(ns::GML_32, "MultiSurface")));
        let (inst, _) = under("BuildingInstallation", surfaces);
        assert!(inst.child(ns::CITYGML_3, "lod3MultiSurface").is_some());
    }

    /// Only a geometry host's `lodNGeometry` is routed.
    #[test]
    fn lod_geometry_outside_a_host_is_left_alone() {
        let mut surfaces = Element::new(Name::qualified(ns::BUILDING_3, "lod2Geometry"));
        surfaces.push(Element::new(Name::qualified(ns::GML_32, "MultiSurface")));
        let (building, _) = under("Building", surfaces);
        assert!(building.child(ns::BUILDING_3, "lod2Geometry").is_some());
    }

    #[test]
    fn installations_gain_their_relation_to_construction() {
        for (property, relation) in [
            ("outerBuildingInstallation", "outside"),
            ("interiorBuildingInstallation", "inside"),
            ("roomInstallation", "inside"),
        ] {
            let mut inst = Element::new(Name::qualified(ns::BUILDING_3, "BuildingInstallation"));
            inst.push(Element::with_text(
                Name::qualified(ns::BUILDING_3, "function"),
                "1000",
            ));
            let mut src = Element::new(Name::qualified(ns::BUILDING_3, property));
            src.push(inst);

            let (building, warnings) = under("Building", src);

            let prop = building
                .child(ns::BUILDING_3, "buildingInstallation")
                .expect(property);
            let inst = prop.elements().next().unwrap();
            assert_eq!(
                inst.child(ns::CONSTRUCTION_3, "relationToConstruction")
                    .unwrap()
                    .text(),
                relation,
                "{property}"
            );
            assert!(
                inst.child(ns::BUILDING_3, "function").is_some(),
                "existing children survive"
            );
            assert!(!warnings.is_empty(), "the invented value is reported");
        }
    }

    /// An installation that already records its placement keeps it.
    #[test]
    fn an_existing_relation_to_construction_is_kept() {
        let mut inst = Element::new(Name::qualified(ns::BUILDING_3, "BuildingInstallation"));
        inst.push(Element::with_text(
            Name::qualified(ns::CONSTRUCTION_3, "relationToConstruction"),
            "bothInsideAndOutside",
        ));
        let mut src = Element::new(Name::qualified(ns::BUILDING_3, "outerBuildingInstallation"));
        src.push(inst);
        let (building, _) = under("Building", src);
        let inst = building
            .child(ns::BUILDING_3, "buildingInstallation")
            .unwrap()
            .elements()
            .next()
            .unwrap();
        let relations: Vec<String> = inst
            .elements()
            .filter(|e| e.name.local == "relationToConstruction")
            .map(|e| e.text())
            .collect();
        assert_eq!(relations, ["bothInsideAndOutside"]);
    }
}
