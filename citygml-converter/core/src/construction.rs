//! Structural rewrites shared by every module deriving from the CityGML 3.0
//! construction module, meaning buildings, bridges, tunnels and other
//! constructions.
//!
//! The properties acted on come from the profile's `[construction]` table, so
//! this module names no thematic module itself. It writes only `con` and
//! `core` names.
//!
//! Everything here speaks CityGML **3.0** names, since
//! [`crate::transform::rename`] has already run.

use crate::error::Result;
use crate::profile::{ConstructionRules, Rules};
use crate::report::Warnings;
use crate::xml::{Element, Name, Node};

/// Solid-valued GML geometries, used to decide what a 2.0 `lodNGeometry`
/// property became in 3.0.
const SOLIDS: &[&str] = &["Solid", "CompositeSolid", "MultiSolid"];

/// The construction rewrites, bound to one profile.
#[derive(Debug, Clone)]
pub struct ConstructionRewrite {
    core: String,
    con: String,
    relation: Name,
    rules: ConstructionRules,
    display: Rules,
}

impl ConstructionRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        Ok(ConstructionRewrite {
            core: rules.output_ns("core")?.to_owned(),
            con: rules.output_ns("con")?.to_owned(),
            relation: Name::qualified(rules.output_ns("con")?, "relationToConstruction"),
            rules: rules.construction().clone(),
            display: rules.clone(),
        })
    }

    /// Rewrites `el` and its descendants in place.
    pub fn apply(&self, el: &mut Element, warnings: &mut Warnings) {
        let host = self.rules.geometry_hosts.contains(&el.name);
        let children = std::mem::take(&mut el.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            match child {
                Node::Element(child) => {
                    out.push(Node::Element(self.rewrite(child, host, warnings)));
                }
                other => out.push(other),
            }
        }
        el.children = out;

        for child in el.elements_mut() {
            self.apply(child, warnings);
        }
    }

    /// Rewrites one child of an element. `host` says whether that element is
    /// one of the profile's geometry hosts.
    fn rewrite(&self, child: Element, host: bool, warnings: &mut Warnings) -> Element {
        if let Some((to, relation)) = self.rules.installations.get(&child.name) {
            return self.installation(child, to, relation, warnings);
        }
        if let Some(to) = self.rules.dates.get(&child.name) {
            return self.date(child, to, warnings);
        }
        match lod_geometry(&child.name.local).filter(|_| host) {
            Some(lod) => self.lod_geometry(child, lod, warnings),
            None => child,
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

    /// Rewrites a `lodNGeometry` property into `core:lodNSolid` or
    /// `core:lodNMultiSurface`, depending on the geometry it holds.
    fn lod_geometry(&self, src: Element, lod: char, warnings: &mut Warnings) -> Element {
        let from = self.display.display_name(&src.name);
        let inner = src.elements().next().map(|g| g.name.local.clone());
        let solid = inner.as_deref().is_some_and(|g| SOLIDS.contains(&g));
        let local = if solid {
            format!("lod{lod}Solid")
        } else {
            format!("lod{lod}MultiSurface")
        };

        if !solid && !matches!(inner.as_deref(), Some("MultiSurface")) {
            warnings.add(format!(
                "{from} held gml:{} and became core:{local}, which expects a \
                 gml:MultiSurface; check the result",
                inner.as_deref().unwrap_or("(nothing)")
            ));
        } else {
            warnings.add(format!(
                "{from} became core:{local} based on the geometry it holds"
            ));
        }

        retag(src, Name::qualified(&self.core, local))
    }
}

/// Replaces an element's name, keeping its attributes and children.
fn retag(mut el: Element, name: Name) -> Element {
    el.name = name;
    el
}

/// The LOD digit of a `lodNGeometry` local name, so `lod2Geometry` gives
/// `Some('2')`. LOD4 is not accepted, since [`crate::lod4`] has already
/// decided where `lod4Geometry` went.
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
        rewrite().apply(&mut el, &mut warnings);
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
