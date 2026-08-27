//! Structural rewrites for the Building module that a rename table cannot
//! express — the ones that change a value or the shape of a subtree.
//!
//! Everything here speaks CityGML **3.0** names: [`crate::transform::rename`]
//! has already run, so `bldg:measuredHeight` arrives in the 3.0 building
//! namespace and leaves as `con:height`.

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

        // Recurse afterwards: nothing we produce is itself in the building
        // namespace, so a rewritten child is never rewritten twice.
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
            // [3.0] 1.2.1.1: in LOD0.1 the building outline and the ground
            // contact outline are told apart by which boundary surface carries
            // the geometry -- the building's orthographic outline goes on a
            // RoofSurface, the ground contact outline on a GroundSurface.
            "lod0RoofEdge" => self.lod0_boundary(child, "RoofSurface", ids, warnings),
            "lod0FootPrint" => self.lod0_boundary(child, "GroundSurface", ids, warnings),
            "yearOfConstruction" => self.construction_date(child, "dateOfConstruction", warnings),
            "yearOfDemolition" => self.construction_date(child, "dateOfDemolition", warnings),
            local => match lod_geometry(local) {
                Some(lod) => self.lod_geometry(child, lod, warnings),
                None => child,
            },
        };
        out.push(Node::Element(rewritten));
    }

    /// `bldg:measuredHeight` -> `con:height` wrapping a `con:Height` object.
    ///
    /// [3.0] §1.13.3.1.15 makes `value`, `status`, `lowReference` and
    /// `highReference` all mandatory, and the two references are code-list
    /// values. CityGML 2.0 records none of them, so they come from the profile's
    /// `[height]` section and every substitution is reported.
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

    /// A `con:lowReference` / `con:highReference` carrying a code and, when the
    /// profile gives one, the code list it came from.
    fn height_reference(&self, local: &str, code: &str) -> Element {
        let mut el = Element::with_text(Name::qualified(&self.con, local), code);
        if let Some(code_space) = &self.height.reference_code_space {
            el.set_attr(Name::unqualified("codeSpace"), code_space.clone());
        }
        el
    }

    /// `bldg:lod0RoofEdge` / `bldg:lod0FootPrint` -> a `con:boundary` holding the
    /// named construction surface, whose geometry is `core:lod0MultiSurface`.
    ///
    /// CityGML 3.0 has a single LOD0 slot per space, so the 2.0 pair cannot both
    /// stay on the building. [3.0] §1.2.1.1 resolves this for LOD0.1 by putting
    /// the roof outline on a `RoofSurface` and the ground outline on a
    /// `GroundSurface`, which is lossless for either or both.
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
             tells the outlines apart by surface type ([3.0] 1.2.1.1, LOD0.1)"
        ));

        let mut boundary = Element::new(Name::qualified(&self.con, "boundary"));
        boundary.push(surface);
        boundary
    }

    /// `bldg:yearOfConstruction` (a `gYear`) -> `con:dateOfConstruction` (a `date`).
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

    /// `bldg:lodNGeometry` -> `core:lodNSolid` or `core:lodNMultiSurface`,
    /// depending on what geometry it actually holds.
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

/// `lod2Geometry` -> `Some('2')`.
fn lod_geometry(local: &str) -> Option<char> {
    let rest = local.strip_prefix("lod")?;
    let mut chars = rest.chars();
    let digit = chars.next().filter(|c| c.is_ascii_digit())?;
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

    #[test]
    fn measured_height_becomes_a_construction_height() {
        let mut src = Element::with_text(Name::qualified(ns::BUILDING_3, "measuredHeight"), "14.3");
        src.set_attr(Name::unqualified("uom"), "m");

        let (building, warnings) = in_building(src);

        let height = building
            .child(ns::CONSTRUCTION_3, "height")
            .expect("con:height");
        let obj = height
            .child(ns::CONSTRUCTION_3, "Height")
            .expect("con:Height");
        let value = obj.child(ns::CONSTRUCTION_3, "value").unwrap();
        assert_eq!(value.text(), "14.3");
        assert_eq!(
            value.attr(None, "uom"),
            Some("m"),
            "uom moves onto con:value"
        );
        assert_eq!(
            obj.child(ns::CONSTRUCTION_3, "status").unwrap().text(),
            "measured"
        );
        // The references are code-list values, not free text ([3.0] 1.13.3.1.15).
        let high = obj.child(ns::CONSTRUCTION_3, "highReference").unwrap();
        assert_eq!(high.text(), "2", "2 = the construction's highest point");
        assert_eq!(
            high.attr(None, "codeSpace"),
            Some("../../codelists/Elevation_elevationReference.xml")
        );
        assert_eq!(
            obj.child(ns::CONSTRUCTION_3, "lowReference")
                .unwrap()
                .text(),
            "6",
            "6 = the lowest ground point"
        );
        assert!(
            !warnings.is_empty(),
            "the assumed references must be reported"
        );
    }

    #[test]
    fn roof_edge_becomes_a_roof_surface_boundary() {
        let mut src = Element::new(Name::qualified(ns::BUILDING_3, "lod0RoofEdge"));
        src.push(Element::new(Name::qualified(ns::GML_32, "MultiSurface")));

        let (building, _) = in_building(src);

        let boundary = building
            .child(ns::CONSTRUCTION_3, "boundary")
            .expect("con:boundary");
        let surface = boundary
            .child(ns::CONSTRUCTION_3, "RoofSurface")
            .expect("con:RoofSurface");
        assert_eq!(surface.attr(Some(ns::GML_32), "id"), Some("bldg_1_1"));
        let geom = surface
            .child(ns::CITYGML_3, "lod0MultiSurface")
            .expect("lod0MultiSurface");
        assert!(
            geom.child(ns::GML_32, "MultiSurface").is_some(),
            "geometry is carried through"
        );
    }

    #[test]
    fn foot_print_becomes_a_ground_surface_boundary() {
        let mut src = Element::new(Name::qualified(ns::BUILDING_3, "lod0FootPrint"));
        src.push(Element::new(Name::qualified(ns::GML_32, "MultiSurface")));

        let (building, _) = in_building(src);

        let boundary = building
            .child(ns::CONSTRUCTION_3, "boundary")
            .expect("con:boundary");
        let surface = boundary
            .child(ns::CONSTRUCTION_3, "GroundSurface")
            .expect("con:GroundSurface");
        assert!(surface.child(ns::CITYGML_3, "lod0MultiSurface").is_some());
    }

    /// Both 2.0 outlines can be present; each keeps its own surface, and the
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
            .filter(|e| e.is(ns::CONSTRUCTION_3, "boundary"))
            .filter_map(|b| b.elements().next())
            .map(|s| s.name.local.as_str())
            .collect();
        assert_eq!(surfaces, ["RoofSurface", "GroundSurface"]);

        let ids: Vec<&str> = building
            .elements()
            .filter(|e| e.is(ns::CONSTRUCTION_3, "boundary"))
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
    fn recognises_lod_geometry_names() {
        assert_eq!(lod_geometry("lod2Geometry"), Some('2'));
        assert_eq!(lod_geometry("lod2Solid"), None);
        assert_eq!(lod_geometry("lodXGeometry"), None);
        assert_eq!(lod_geometry("Geometry"), None);
    }

    #[test]
    fn leaves_untouched_properties_alone() {
        let src = Element::with_text(Name::qualified(ns::BUILDING_3, "roofType"), "2100");
        let (building, warnings) = in_building(src);
        assert_eq!(
            building.child(ns::BUILDING_3, "roofType").unwrap().text(),
            "2100"
        );
        assert!(warnings.is_empty());
    }
}
