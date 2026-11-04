//! Moves a feature's 2.0 geometry to the LOD the profile's `[lod_map]` table
//! gives its type, such as a bridge's old LOD3 to LOD2.
//!
//! Runs after [`crate::transform::rename`] and before [`crate::lod4`], so the
//! geometry properties are `core:lodN*` or a module's `lodNGeometry`, not yet
//! routed by [`crate::construction`].
//!
//! Each feature of a listed type is decided on its own, covering everything
//! under it except a nested feature of a listed type, which is decided in
//! turn. For each move from LOD `a` to LOD `b`, a feature with LOD `a`
//! geometry has the following done:
//!
//! * its LOD `b` geometry is removed, with every surface, opening,
//!   installation or other feature left without geometry by it
//! * its LOD `b` data quality descriptors and the `lodType` codes of LOD `b`
//!   are removed. When the feature carries no code of LOD `a` and its
//!   `[lod_types]` table names a `moved` code for LOD `a`, the first such
//!   code is rewritten to that code instead
//! * its LOD `a` geometry properties and descriptors are renamed to LOD `b`
//!
//! Every removal and move is reported, and the `gml:id` of every removed
//! element is added to the caller's set.

use std::collections::{HashMap, HashSet};

use crate::error::Result;
use crate::geometry::collect_ids;
use crate::iur::IurModules;
use crate::profile::Rules;
use crate::report::Warnings;
use crate::xml::{Element, Name, Node};

/// The geometry property families a `lodN` prefix is followed by, in `core`
/// or in a module's namespace, the 2.0 names a module's `lod4*` still
/// carries included.
const FAMILIES: &[&str] = &[
    "Solid",
    "MultiSurface",
    "MultiCurve",
    "Point",
    "Geometry",
    "TerrainIntersection",
    "TerrainIntersectionCurve",
    "ImplicitRepresentation",
];

/// One move's removal of LOD `lod`, for [`LodMapRewrite::strip`].
///
/// `from` is the LOD moving in and `label` the feature being decided, both
/// for the report. `moved` is the code the first removed `lodType` is
/// rewritten to instead, taken once used. `dropped` gains the `gml:id` of
/// every removed element.
struct Strip<'a> {
    lod: char,
    from: char,
    label: &'a str,
    moved: &'a mut Option<String>,
    dropped: &'a mut HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct LodMapRewrite {
    gml: String,
    iur: IurModules,
    moves: HashMap<Name, Vec<(char, char)>>,
    display: Rules,
}

impl LodMapRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        Ok(LodMapRewrite {
            gml: rules.output_ns("gml")?.to_owned(),
            iur: IurModules::new(rules),
            moves: rules.lod_map().clone(),
            display: rules.clone(),
        })
    }

    /// Rewrites every feature of a listed type under `member`, `member`
    /// included. Returns true when anything was added to `dropped`.
    pub fn apply(
        &self,
        member: &mut Element,
        dropped: &mut HashSet<String>,
        warnings: &mut Warnings,
    ) -> bool {
        if self.moves.is_empty() {
            return false;
        }
        let before = dropped.len();
        self.walk(member, dropped, warnings);
        dropped.len() > before
    }

    fn walk(&self, el: &mut Element, dropped: &mut HashSet<String>, warnings: &mut Warnings) {
        if let Some(moves) = self.moves.get(&el.name) {
            for &(from, to) in moves {
                self.relod(el, from, to, dropped, warnings);
            }
        }
        for child in el.elements_mut() {
            if self.lod_of(child).is_none() {
                self.walk(child, dropped, warnings);
            }
        }
    }

    /// Moves `feature`'s LOD `from` to LOD `to`.
    fn relod(
        &self,
        feature: &mut Element,
        from: char,
        to: char,
        dropped: &mut HashSet<String>,
        warnings: &mut Warnings,
    ) {
        if !self.has_lod(feature, from, true) {
            return;
        }
        let label = self.display.display_name(&feature.name);
        if self.has_lod(feature, to, true) {
            warnings.add(format!(
                "the {label} carries both LOD{to} and LOD{from} geometry. Its LOD{from} \
                 is LOD{to} in CityGML 3.0 and takes the LOD{to} slots, so its LOD{to} \
                 geometry was dropped, with every feature left without geometry by it"
            ));
        }
        let mut moved = self
            .display
            .lod_types(&feature.name)
            .and_then(|(_, table)| table.moved.get(&from.to_string()))
            .filter(|_| !self.has_lod_type(feature, from, true))
            .cloned();
        let mut strip = Strip {
            lod: to,
            from,
            label: &label,
            moved: &mut moved,
            dropped,
        };
        self.strip(feature, &mut strip, true, warnings);
        self.rename(feature, from, to, &label, true, warnings);
    }

    /// The LOD digit of a geometry property, or `None` for anything else.
    fn lod_of(&self, el: &Element) -> Option<char> {
        let rest = el.name.local.strip_prefix("lod")?;
        let mut chars = rest.chars();
        let lod = chars.next().filter(char::is_ascii_digit)?;
        FAMILIES.contains(&chars.as_str()).then_some(lod)
    }

    /// True for a feature of a listed type other than the one being decided.
    fn is_nested(&self, el: &Element, top: bool) -> bool {
        !top && self.moves.contains_key(&el.name)
    }

    /// True when a LOD `lod` geometry property sits under `el`, outside any
    /// nested feature of a listed type.
    fn has_lod(&self, el: &Element, lod: char, top: bool) -> bool {
        if self.is_nested(el, top) {
            return false;
        }
        el.elements().any(|child| match self.lod_of(child) {
            Some(l) => l == lod,
            None => self.has_lod(child, lod, false),
        })
    }

    /// True when a `lodType` code of LOD `lod` sits under `el`, outside any
    /// nested feature of a listed type.
    fn has_lod_type(&self, el: &Element, lod: char, top: bool) -> bool {
        if self.is_nested(el, top) {
            return false;
        }
        el.elements()
            .any(|child| self.is_lod_type(child, lod) || self.has_lod_type(child, lod, false))
    }

    /// True when any geometry property sits under `el`.
    fn has_geometry(&self, el: &Element) -> bool {
        el.elements()
            .any(|child| self.lod_of(child).is_some() || self.has_geometry(child))
    }

    /// Removes the LOD `lod` geometry properties under `el`, with each feature
    /// they leave without geometry and each property they leave empty, and
    /// the LOD `lod` quality descriptors and `lodType` codes, as [`Strip`]
    /// describes. Returns true when a geometry property was removed under
    /// `el`.
    fn strip(
        &self,
        el: &mut Element,
        strip: &mut Strip,
        top: bool,
        warnings: &mut Warnings,
    ) -> bool {
        let (lod, from, label) = (strip.lod, strip.from, strip.label);
        if self.is_nested(el, top) {
            return false;
        }
        let mut lost = false;
        let children = std::mem::take(&mut el.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            let Node::Element(mut child) = child else {
                out.push(child);
                continue;
            };
            if let Some(l) = self.lod_of(&child) {
                if l == lod {
                    collect_ids(&child, &self.gml, strip.dropped);
                    lost = true;
                    continue;
                }
                out.push(Node::Element(child));
                continue;
            }
            if self.is_quality(&child, lod) {
                warnings.add(format!(
                    "{} on the {label} was dropped: its LOD{from} takes LOD{lod}, and the \
                     descriptor described what LOD{lod} held before",
                    self.display.display_name(&child.name)
                ));
                continue;
            }
            if self.is_lod_type(&child, lod) {
                if let Some(code) = strip.moved.take() {
                    warnings.add(format!(
                        "lodType {} on the {label} became {code}: its LOD{from} takes \
                         LOD{lod} and carries no code of its own",
                        child.text().trim()
                    ));
                    child.children = vec![Node::Text(code)];
                    out.push(Node::Element(child));
                    continue;
                }
                warnings.add(format!(
                    "lodType {} on the {label} was dropped: its LOD{from} takes LOD{lod}, \
                     and the code classified what LOD{lod} held before",
                    child.text().trim()
                ));
                continue;
            }
            let had_content = child.elements().next().is_some();
            let child_lost = self.strip(&mut child, strip, false, warnings);
            lost |= child_lost;
            let is_feature =
                child.name.local.starts_with(char::is_uppercase) && !child.name.in_ns(&self.gml);
            let emptied_feature = is_feature && child_lost && !self.has_geometry(&child);
            let emptied_property =
                !is_feature && child_lost && had_content && child.elements().next().is_none();
            if emptied_feature || emptied_property {
                collect_ids(&child, &self.gml, strip.dropped);
                continue;
            }
            out.push(Node::Element(child));
        }
        el.children = out;
        lost
    }

    /// Renames the LOD `from` geometry properties and quality descriptors
    /// under `el` to LOD `to`.
    fn rename(
        &self,
        el: &mut Element,
        from: char,
        to: char,
        label: &str,
        top: bool,
        warnings: &mut Warnings,
    ) {
        if self.is_nested(el, top) {
            return;
        }
        for child in el.elements_mut() {
            if self.lod_of(child) == Some(from) {
                let old = self.display.display_name(&child.name);
                child.name.local = format!("lod{to}{}", &child.name.local[4..]);
                warnings.add(format!(
                    "{old} became {} under the {label}: the old LOD{from} of a {label} is \
                     LOD{to} in CityGML 3.0",
                    self.display.display_name(&child.name)
                ));
                continue;
            }
            if let Some((stem, lod)) = self.iur.quality_descriptor(&child.name) {
                if lod == from {
                    let old = self.display.display_name(&child.name);
                    child.name.local = format!("{stem}Lod{to}");
                    warnings.add(format!(
                        "{old} became {} along with the {label}'s LOD{from} geometry",
                        self.display.display_name(&child.name)
                    ));
                }
                continue;
            }
            self.rename(child, from, to, label, false, warnings);
        }
    }

    /// True for a LOD `lod` data quality descriptor.
    fn is_quality(&self, el: &Element, lod: char) -> bool {
        matches!(self.iur.quality_descriptor(&el.name), Some((_, l)) if l == lod)
    }

    /// True for an i-UR `lodType` holding a code of LOD `lod`.
    fn is_lod_type(&self, el: &Element, lod: char) -> bool {
        el.name.local == "lodType"
            && self.iur.contains(&el.name)
            && el.text().trim().starts_with(&format!("{lod}."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_PROFILE;
    use crate::xml::ns;

    const BRID: &str = "http://www.opengis.net/citygml/bridge/3.0";
    const URC: &str = "https://www.geospatial.jp/iur/urc/4.0";

    fn rewrite() -> LodMapRewrite {
        LodMapRewrite::new(&Rules::from_toml(DEFAULT_PROFILE).unwrap()).unwrap()
    }

    fn geometry(ns: &str, local: &str, id: &str) -> Element {
        let mut polygon = Element::new(Name::qualified(ns::GML_32, "Polygon"));
        polygon.set_attr(Name::qualified(ns::GML_32, "id"), id);
        let mut prop = Element::new(Name::qualified(ns, local));
        prop.push(polygon);
        prop
    }

    /// A `core:boundary` holding one wall with the given children.
    fn wall(children: Vec<Element>) -> Element {
        let mut wall = Element::new(Name::qualified(ns::CONSTRUCTION_3, "WallSurface"));
        for c in children {
            wall.push(c);
        }
        let mut boundary = Element::new(Name::qualified(ns::CITYGML_3, "boundary"));
        boundary.push(wall);
        boundary
    }

    fn walls(bridge: &Element) -> Vec<&Element> {
        bridge
            .elements()
            .filter(|e| e.is(ns::CITYGML_3, "boundary"))
            .filter_map(|b| b.elements().next())
            .collect()
    }

    /// The moved LOD takes the slot. What the slot held goes, with the wall
    /// it leaves empty and the descriptor that described it.
    #[test]
    fn the_moved_lod_replaces_what_its_new_slot_held() {
        let mut quality = Element::new(Name::qualified(URC, "ExteriorDataQualityAttribute"));
        quality.push(Element::with_text(
            Name::qualified(URC, "geometrySrcDescLod2"),
            "a",
        ));
        quality.push(Element::with_text(
            Name::qualified(URC, "geometrySrcDescLod3"),
            "b",
        ));
        let mut bridge = Element::new(Name::qualified(BRID, "Bridge"));
        bridge.push(wall(vec![geometry(
            ns::CITYGML_3,
            "lod2MultiSurface",
            "old2",
        )]));
        bridge.push(wall(vec![geometry(
            ns::CITYGML_3,
            "lod3MultiSurface",
            "old3",
        )]));
        bridge.push(quality);

        let mut dropped = HashSet::new();
        assert!(rewrite().apply(&mut bridge, &mut dropped, &mut Warnings::new()));

        let walls = walls(&bridge);
        assert_eq!(walls.len(), 1, "the emptied wall and its boundary went");
        assert!(walls[0].child(ns::CITYGML_3, "lod2MultiSurface").is_some());
        assert!(dropped.contains("old2") && !dropped.contains("old3"));
        let quality = bridge.child(URC, "ExteriorDataQualityAttribute").unwrap();
        let descriptors: Vec<(String, String)> = quality
            .elements()
            .map(|e| (e.name.local.clone(), e.text()))
            .collect();
        assert_eq!(descriptors, [("geometrySrcDescLod2".into(), "b".into())]);
    }

    /// A wall whose remaining geometry is LOD4, still in the module's
    /// namespace at this point, is not left empty and stays.
    #[test]
    fn a_surface_holding_lod4_geometry_is_kept() {
        let mut window = Element::new(Name::qualified(ns::CONSTRUCTION_3, "WindowSurface"));
        window.push(geometry(BRID, "lod4MultiSurface", "w4"));
        let mut opening = Element::new(Name::qualified(ns::CONSTRUCTION_3, "fillingSurface"));
        opening.push(window);
        let mut bridge = Element::new(Name::qualified(BRID, "Bridge"));
        bridge.push(wall(vec![
            geometry(ns::CITYGML_3, "lod2MultiSurface", "old2"),
            opening,
        ]));
        bridge.push(geometry(ns::CITYGML_3, "lod3Solid", "old3"));

        let mut dropped = HashSet::new();
        rewrite().apply(&mut bridge, &mut dropped, &mut Warnings::new());

        assert_eq!(walls(&bridge).len(), 1);
        assert!(!dropped.contains("w4"));
        assert!(bridge.child(ns::CITYGML_3, "lod2Solid").is_some());
    }
}
