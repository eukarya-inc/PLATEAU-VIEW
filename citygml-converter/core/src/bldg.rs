//! Structural rewrites for the Building module that a rename table cannot
//! express, meaning the ones that change a value or the shape of a subtree.
//!
//! Everything here speaks CityGML **3.0** names, since
//! [`crate::transform::rename`] has already run. `bldg:measuredHeight` arrives
//! in the 3.0 building namespace and leaves as `con:height`.
//!
//! Rewrites every construction shares, such as installations, construction
//! dates and `lodNGeometry`, are [`crate::construction`]'s.

use crate::error::Result;
use crate::profile::{HeightDefaults, Rules, locate};
use crate::report::Warnings;
use crate::transform::IdGen;
use crate::xml::{Element, Name, Node};

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

    /// Rewrites `el` and its descendants in place. `to_root` is the path from
    /// the document back to the package root, such as `../../`.
    pub fn apply(&self, el: &mut Element, to_root: &str, ids: &mut IdGen, warnings: &mut Warnings) {
        let children = std::mem::take(&mut el.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            match child {
                Node::Element(child) => self.rewrite(child, &mut out, to_root, ids, warnings),
                other => out.push(other),
            }
        }
        el.children = out;

        for child in el.elements_mut() {
            self.apply(child, to_root, ids, warnings);
        }
    }

    fn rewrite(
        &self,
        child: Element,
        out: &mut Vec<Node>,
        to_root: &str,
        ids: &mut IdGen,
        warnings: &mut Warnings,
    ) {
        if !child.name.in_ns(&self.bldg) {
            out.push(Node::Element(child));
            return;
        }

        let rewritten = match child.name.local.as_str() {
            "measuredHeight" => self.height(child, to_root, warnings),
            "lod0RoofEdge" => self.lod0_boundary(child, "RoofSurface", ids, warnings),
            "lod0FootPrint" => self.lod0_boundary(child, "GroundSurface", ids, warnings),
            _ => child,
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
    fn height(&self, src: Element, to_root: &str, warnings: &mut Warnings) -> Element {
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
        height.push(self.height_reference("lowReference", &self.height.low_reference, to_root));
        height.push(self.height_reference("highReference", &self.height.high_reference, to_root));

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
    fn height_reference(&self, local: &str, code: &str, to_root: &str) -> Element {
        let mut el = Element::with_text(Name::qualified(&self.con, local), code);
        if let Some(code_space) = &self.height.reference_code_space {
            el.set_attr(Name::unqualified("codeSpace"), locate(code_space, to_root));
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
}

/// Replaces an element's name, keeping its attributes and children.
fn retag(mut el: Element, name: Name) -> Element {
    el.name = name;
    el
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_PROFILE;
    use crate::convert::FEATURE_TYPE_TO_ROOT;
    use crate::xml::ns;

    fn rewrite() -> BuildingRewrite {
        BuildingRewrite::new(&Rules::from_toml(DEFAULT_PROFILE).unwrap()).unwrap()
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
        rewrite().apply(&mut building, FEATURE_TYPE_TO_ROOT, &mut ids, &mut warnings);

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
}
