//! Structural rewrites for the Building module that a rename table cannot
//! express, meaning the ones that change a value or the shape of a subtree.
//!
//! Everything here speaks CityGML **3.0** names, since
//! [`crate::transform::rename`] has already run. `bldg:measuredHeight` arrives
//! in the 3.0 building namespace and leaves as `con:height`.

use crate::error::Result;
use crate::profile::{HeightDefaults, Rules};
use crate::report::Warnings;
use crate::transform::IdGen;
use crate::xml::{Element, Name, Node};

/// Solid-valued GML geometries, used to decide what a 2.0 `lodNGeometry`
/// property became in 3.0.
const SOLIDS: &[&str] = &["Solid", "CompositeSolid", "MultiSolid"];

/// The building rewrites, bound to one profile's output namespaces.
#[derive(Debug, Clone)]
pub struct BuildingRewrite {
    core: String,
    bldg: String,
    con: String,
    gml: String,
    height: HeightDefaults,
}

impl BuildingRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        Ok(BuildingRewrite {
            core: rules.output_ns("core")?.to_owned(),
            bldg: rules.output_ns("bldg")?.to_owned(),
            con: rules.output_ns("con")?.to_owned(),
            gml: rules.output_ns("gml")?.to_owned(),
            height: rules.height().clone(),
        })
    }

    /// Rewrites `el` and its descendants in place.
    pub fn apply(&self, el: &mut Element, ids: &mut IdGen, warnings: &mut Warnings) {
        let children = std::mem::take(&mut el.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            match child {
                Node::Element(child) => self.rewrite(child, &mut out, ids, warnings),
                other => out.push(other),
            }
        }
        el.children = out;

        for child in el.elements_mut() {
            self.apply(child, ids, warnings);
        }
    }

    fn rewrite(
        &self,
        child: Element,
        out: &mut Vec<Node>,
        ids: &mut IdGen,
        warnings: &mut Warnings,
    ) {
        if !child.name.in_ns(&self.bldg) {
            out.push(Node::Element(child));
            return;
        }

        let rewritten = match child.name.local.as_str() {
            "measuredHeight" => self.height(child, warnings),
            "lod0RoofEdge" => self.lod0_boundary(child, "RoofSurface", ids, warnings),
            "lod0FootPrint" => self.lod0_boundary(child, "GroundSurface", ids, warnings),
            "yearOfConstruction" => self.construction_date(child, "dateOfConstruction", warnings),
            "yearOfDemolition" => self.construction_date(child, "dateOfDemolition", warnings),
            "outerBuildingInstallation" => self.installation(child, "outside", warnings),
            "interiorBuildingInstallation" | "roomInstallation" => {
                self.installation(child, "inside", warnings)
            }
            local => match lod_geometry(local) {
                Some(lod) => self.lod_geometry(child, lod, warnings),
                None => child,
            },
        };
        out.push(Node::Element(rewritten));
    }

    /// Rewrites `bldg:measuredHeight` into a `con:height` wrapping a
    /// `con:Height` object.
    ///
    /// CityGML 3.0 makes `value`, `status`, `lowReference` and
    /// `highReference` all mandatory. CityGML 2.0 records none of them, so
    /// they come from the profile's `[height]` section and every substitution
    /// is reported.
    fn height(&self, src: Element, warnings: &mut Warnings) -> Element {
        let uom = src.attr(None, "uom").map(str::to_owned);
        let mut value = Element::with_text(
            Name::qualified(&self.con, "value"),
            src.text().trim().to_owned(),
        );
        if let Some(uom) = uom {
            value.set_attr(Name::unqualified("uom"), uom);
        }

        let mut height = Element::new(Name::qualified(&self.con, "Height"));
        height.push(value);
        height.push(Element::with_text(
            Name::qualified(&self.con, "status"),
            self.height.status.clone(),
        ));
        height.push(self.height_reference("lowReference", &self.height.low_reference));
        height.push(self.height_reference("highReference", &self.height.high_reference));

        warnings.add(format!(
            "bldg:measuredHeight became con:height with status={}, lowReference={}, \
             highReference={}; CityGML 2.0 records none of these, so the profile's \
             [height] defaults were used",
            self.height.status, self.height.low_reference, self.height.high_reference
        ));

        let mut prop = Element::new(Name::qualified(&self.con, "height"));
        prop.push(height);
        prop
    }

    /// A `con:lowReference` or `con:highReference` carrying a code and, when
    /// the profile gives one, the code list it came from.
    fn height_reference(&self, local: &str, code: &str) -> Element {
        let mut el = Element::with_text(Name::qualified(&self.con, local), code);
        if let Some(code_space) = &self.height.reference_code_space {
            el.set_attr(Name::unqualified("codeSpace"), code_space.clone());
        }
        el
    }

    /// Rewrites `bldg:lod0RoofEdge` or `bldg:lod0FootPrint` into a
    /// `core:boundary` holding the construction surface named by `surface`,
    /// whose geometry is `core:lod0MultiSurface`.
    ///
    /// The roof outline goes on a `RoofSurface` and the ground outline on a
    /// `GroundSurface`, so an input carrying both keeps both.
    fn lod0_boundary(
        &self,
        src: Element,
        surface_type: &str,
        ids: &mut IdGen,
        warnings: &mut Warnings,
    ) -> Element {
        let property = src.name.local.clone();
        let geometry = retag(src, Name::qualified(&self.core, "lod0MultiSurface"));

        let mut surface = Element::new(Name::qualified(&self.con, surface_type));
        surface.set_attr(Name::qualified(&self.gml, "id"), ids.mint());
        surface.push(geometry);

        warnings.add(format!(
            "bldg:{property} became a con:{surface_type} boundary carrying \
             core:lod0MultiSurface; CityGML 3.0 has one LOD0 geometry per space and \
             tells the outlines apart by surface type (LOD0.1)"
        ));

        let mut boundary = Element::new(Name::qualified(&self.core, "boundary"));
        boundary.push(surface);
        boundary
    }

    /// Rewrites `bldg:outerBuildingInstallation`,
    /// `bldg:interiorBuildingInstallation` and `bldg:roomInstallation` into
    /// `bldg:buildingInstallation`, moving the placement the 2.0 property name
    /// expressed onto the installation as `con:relationToConstruction`.
    fn installation(&self, src: Element, relation: &str, warnings: &mut Warnings) -> Element {
        let property = src.name.local.clone();
        let mut prop = retag(src, Name::qualified(&self.bldg, "buildingInstallation"));
        let relation_name = Name::qualified(&self.con, "relationToConstruction");
        for installation in prop.elements_mut() {
            if installation
                .child(&self.con, "relationToConstruction")
                .is_none()
            {
                installation.children.insert(
                    0,
                    Node::Element(Element::with_text(relation_name.clone(), relation)),
                );
            }
        }
        warnings.add(format!(
            "bldg:{property} became bldg:buildingInstallation with \
             con:relationToConstruction={relation}: CityGML 3.0 has one \
             installation property and records the placement on the installation"
        ));
        prop
    }

    /// Rewrites `bldg:yearOfConstruction`, a `gYear`, into
    /// `con:dateOfConstruction`, a `date`.
    fn construction_date(&self, src: Element, local: &str, warnings: &mut Warnings) -> Element {
        let raw = src.text().trim().to_owned();
        let value = if raw.len() == 4 && raw.bytes().all(|b| b.is_ascii_digit()) {
            warnings.add(format!(
                "bldg:{} held the year {raw}; con:{local} is a date, so it became {raw}-01-01",
                src.name.local
            ));
            format!("{raw}-01-01")
        } else {
            raw
        };
        Element::with_text(Name::qualified(&self.con, local), value)
    }

    /// Rewrites `bldg:lodNGeometry` into `core:lodNSolid` or
    /// `core:lodNMultiSurface`, depending on the geometry it holds.
    fn lod_geometry(&self, src: Element, lod: char, warnings: &mut Warnings) -> Element {
        let inner = src.elements().next().map(|g| g.name.local.clone());
        let solid = inner.as_deref().is_some_and(|g| SOLIDS.contains(&g));
        let local = if solid {
            format!("lod{lod}Solid")
        } else {
            format!("lod{lod}MultiSurface")
        };

        if !solid && !matches!(inner.as_deref(), Some("MultiSurface")) {
            warnings.add(format!(
                "bldg:lod{lod}Geometry held gml:{} and became core:{local}, which \
                 expects a gml:MultiSurface; check the result",
                inner.as_deref().unwrap_or("(nothing)")
            ));
        } else {
            warnings.add(format!(
                "bldg:lod{lod}Geometry became core:{local} based on the geometry it holds"
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

    fn rewrite() -> BuildingRewrite {
        BuildingRewrite::new(&Rules::from_toml(DEFAULT_PROFILE).unwrap()).unwrap()
    }

    /// Wraps `child` in a 3.0 Building and applies the rewrites.
    fn in_building(child: Element) -> (Element, Warnings) {
        let mut building = Element::new(Name::qualified(ns::BUILDING_3, "Building"));
        building.push(child);
        let mut warnings = Warnings::new();
        let mut ids = IdGen::new("bldg_1");
        rewrite().apply(&mut building, &mut ids, &mut warnings);
        (building, warnings)
    }

    /// Both 2.0 outlines can be present. Each keeps its own surface, and the
    /// generated ids stay distinct.
    #[test]
    fn both_lod0_outlines_survive_together() {
        let mut building = Element::new(Name::qualified(ns::BUILDING_3, "Building"));
        building.push(Element::new(Name::qualified(
            ns::BUILDING_3,
            "lod0RoofEdge",
        )));
        building.push(Element::new(Name::qualified(
            ns::BUILDING_3,
            "lod0FootPrint",
        )));
        let mut warnings = Warnings::new();
        let mut ids = IdGen::new("bldg_1");
        rewrite().apply(&mut building, &mut ids, &mut warnings);

        let surfaces: Vec<&str> = building
            .elements()
            .filter(|e| e.is(ns::CITYGML_3, "boundary"))
            .filter_map(|b| b.elements().next())
            .map(|s| s.name.local.as_str())
            .collect();
        assert_eq!(surfaces, ["RoofSurface", "GroundSurface"]);

        let ids: Vec<&str> = building
            .elements()
            .filter(|e| e.is(ns::CITYGML_3, "boundary"))
            .filter_map(|b| b.elements().next())
            .filter_map(|s| s.attr(Some(ns::GML_32), "id"))
            .collect();
        assert_eq!(ids, ["bldg_1_1", "bldg_1_2"]);
    }

    #[test]
    fn year_of_construction_becomes_a_date() {
        let src = Element::with_text(
            Name::qualified(ns::BUILDING_3, "yearOfConstruction"),
            "1998",
        );
        let (building, warnings) = in_building(src);
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
        let (building, _) = in_building(solid);
        assert!(building.child(ns::CITYGML_3, "lod2Solid").is_some());

        let mut surfaces = Element::new(Name::qualified(ns::BUILDING_3, "lod3Geometry"));
        surfaces.push(Element::new(Name::qualified(ns::GML_32, "MultiSurface")));
        let (building, _) = in_building(surfaces);
        assert!(building.child(ns::CITYGML_3, "lod3MultiSurface").is_some());
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

            let (building, warnings) = in_building(src);

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
        let (building, _) = in_building(src);
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
