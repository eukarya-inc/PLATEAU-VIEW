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
//!   `[lod_types]` table, or that of the feature it sits in, names a `moved`
//!   code for LOD `a`, the first such code is rewritten to that code
//!   instead, and the code is added as [`lod_type::add`] places it when there
//!   was none to rewrite
//! * its LOD `a` geometry properties and descriptors are renamed to LOD `b`
//!
//! Once its moves are done, a feature with geometry at a LOD its
//! `[lod_types]` table, or that of the feature it sits in, names a `kept`
//! code for, and with no code of that LOD, is given that code as
//! [`lod_type::add`] places it.
//!
//! Every removal and move is reported, and the `gml:id` of every removed
//! element is added to the caller's set.

use std::collections::HashMap;

use crate::dropped::{Dropped, Prune};
use crate::error::Result;
use crate::geometry::lod_of;
use crate::iur::IurModules;
use crate::lod_type::{self, Added};
use crate::profile::{LodTypeTable, Rules};
use crate::report::Warnings;
use crate::xml::{Element, Name, Node};

#[derive(Debug, Clone)]
pub struct LodMapRewrite {
    iur: IurModules,
    moves: HashMap<Name, Vec<(char, char)>>,
    display: Rules,
}

impl LodMapRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        Ok(LodMapRewrite {
            iur: IurModules::new(rules),
            moves: rules.lod_map().clone(),
            display: rules.clone(),
        })
    }

    /// Rewrites every feature of a listed type under `member`, `member`
    /// included.
    pub fn apply(&self, member: &mut Element, dropped: &mut Dropped, warnings: &mut Warnings) {
        if !self.moves.is_empty() {
            self.walk(member, None, dropped, warnings);
        }
    }

    /// `host` is the `[lod_types]` table of the outermost feature `el` sits
    /// in, if any.
    fn walk<'a>(
        &'a self,
        el: &mut Element,
        host: Option<&'a LodTypeTable>,
        dropped: &mut Dropped,
        warnings: &mut Warnings,
    ) {
        let host = host.or_else(|| self.display.lod_types(&el.name).map(|(_, t)| t));
        if let Some(moves) = self.moves.get(&el.name) {
            for &(from, to) in moves {
                self.relod(el, (from, to), host, dropped, warnings);
            }
            if let Some(table) = host {
                self.keep(el, table, warnings);
            }
        }
        for child in el.elements_mut() {
            if self.lod_of(child).is_none() {
                self.walk(child, host, dropped, warnings);
            }
        }
    }

    /// Moves `feature`'s LOD `from` to LOD `to`. `table` gives the `moved`
    /// code.
    fn relod(
        &self,
        feature: &mut Element,
        (from, to): (char, char),
        table: Option<&LodTypeTable>,
        dropped: &mut Dropped,
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
        let mut moved = table
            .and_then(|table| table.moved.get(&from.to_string()))
            .filter(|_| !self.has_lod_type(feature, from, true))
            .cloned();
        self.strip(feature, (from, to), &label, &mut moved, dropped, warnings);
        self.rename(feature, from, to, &label, true, warnings);
        if let (Some(code), Some(table)) = (moved, table) {
            match lod_type::add(&self.iur, feature, &code, &table.list) {
                Added::Added => warnings.add(format!(
                    "lodType {code} was added to the {label}: its LOD{from} takes LOD{to} and \
                     carries no code of its own"
                )),
                Added::Present => {}
                Added::NoHolder => warnings.add(format!(
                    "the {label} was given no lodType {code} for its LOD{from} that took \
                     LOD{to}: it carries no data quality attribute to hold one"
                )),
            }
        }
    }

    /// Gives `feature` the `kept` code of `table` for each LOD it has
    /// geometry at and no code of.
    fn keep(&self, feature: &mut Element, table: &LodTypeTable, warnings: &mut Warnings) {
        for (lod, code) in &table.kept {
            let Some(lod) = lod.chars().next() else {
                continue;
            };
            if !self.has_lod(feature, lod, true) || self.has_lod_type(feature, lod, true) {
                continue;
            }
            let label = self.display.display_name(&feature.name);
            match lod_type::add(&self.iur, feature, code, &table.list) {
                Added::Added => warnings.add(format!(
                    "lodType {code} was added to the {label}: its LOD{lod} carries no code \
                     of its own"
                )),
                Added::Present => {}
                Added::NoHolder => warnings.add(format!(
                    "the {label} was given no lodType {code} for its LOD{lod}: it carries \
                     no data quality attribute to hold one"
                )),
            }
        }
    }

    /// The LOD digit of a geometry property, or `None` for anything else.
    fn lod_of(&self, el: &Element) -> Option<char> {
        lod_of(&el.name.local)
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

    /// Removes LOD `lod` from `feature` for a move from LOD `from`, outside
    /// any nested feature of a listed type, as [`Dropped::prune`] removes it.
    /// The LOD `lod` geometry properties, quality descriptors and `lodType`
    /// codes go, except that the first such code is rewritten to `moved`,
    /// which is then taken. `label` names the feature in the report.
    fn strip(
        &self,
        feature: &mut Element,
        (from, lod): (char, char),
        label: &str,
        moved: &mut Option<String>,
        dropped: &mut Dropped,
        warnings: &mut Warnings,
    ) {
        let mut visit = |child: &mut Element, warnings: &mut Warnings| {
            if self.is_nested(child, false) {
                return Prune::Keep;
            }
            if let Some(l) = self.lod_of(child) {
                return if l == lod { Prune::Remove } else { Prune::Keep };
            }
            if self.is_quality(child, lod) {
                warnings.add(format!(
                    "{} on the {label} was dropped: its LOD{from} takes LOD{lod}, and the \
                     descriptor described what LOD{lod} held before",
                    self.display.display_name(&child.name)
                ));
                return Prune::Discard;
            }
            if !self.is_lod_type(child, lod) {
                return Prune::Descend;
            }
            if let Some(code) = moved.take() {
                warnings.add(format!(
                    "lodType {} on the {label} became {code}: its LOD{from} takes \
                     LOD{lod} and carries no code of its own",
                    child.text().trim()
                ));
                child.children = vec![Node::Text(code)];
                return Prune::Keep;
            }
            warnings.add(format!(
                "lodType {} on the {label} was dropped: its LOD{from} takes LOD{lod}, \
                 and the code classified what LOD{lod} held before",
                child.text().trim()
            ));
            Prune::Discard
        };
        dropped.prune(feature, warnings, &mut visit, &mut |_, _| {});
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
    /// it leaves empty and the descriptor that described it, and the bridge,
    /// which carries no lodType, gains the `moved` code beside its
    /// descriptors.
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

        let mut dropped = Dropped::default();
        rewrite().apply(&mut bridge, &mut dropped, &mut Warnings::new());

        let walls = walls(&bridge);
        assert_eq!(walls.len(), 1, "the emptied wall and its boundary went");
        assert!(walls[0].child(ns::CITYGML_3, "lod2MultiSurface").is_some());
        assert!(dropped.ids().contains("old2") && !dropped.ids().contains("old3"));
        let quality = bridge.child(URC, "ExteriorDataQualityAttribute").unwrap();
        let descriptors: Vec<(String, String)> = quality
            .elements()
            .map(|e| (e.name.local.clone(), e.text()))
            .collect();
        assert_eq!(
            descriptors,
            [
                ("geometrySrcDescLod2".into(), "b".into()),
                ("lodType".into(), "2.1_exterior".into())
            ]
        );
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

        let mut dropped = Dropped::default();
        rewrite().apply(&mut bridge, &mut dropped, &mut Warnings::new());

        assert_eq!(walls(&bridge).len(), 1);
        assert!(!dropped.ids().contains("w4"));
        assert!(bridge.child(ns::CITYGML_3, "lod2Solid").is_some());
    }
}
