//! Where CityGML 2.0 LOD4 goes, since CityGML 3.0 stops at LOD3.
//!
//! Like [`crate::bldg`] this runs *after* [`crate::transform::rename`] and so
//! speaks 3.0 / i-UR 4.0 names. The profile deliberately has no rename rule for
//! any `lod4*` element, so they reach this pass still spelled `lod4` — a
//! `bldg:lod4Solid` in the 3.0 building namespace — and are the only thing
//! this pass touches.
//!
//! CityGML 3.0 splits a building into an *exterior* model and an *interior*
//! model, each with its own LOD0–3. CityGML 2.0's LOD4 is the interior model,
//! and which LOD it becomes depends on how it was measured: a surveyed
//! interior is an interior LOD2, one derived from BIM an interior LOD3. The
//! measurement method is recorded per feature in the data quality attribute
//! (`geometrySrcDescLod4`), and the profile's `[lod4]` table says which codes
//! mean which. A feature whose code decides nothing takes the configured
//! fallback — LOD3, LOD2 or drop — and is reported.
//!
//! So the fold is two mappings, chosen by where an element sits:
//!
//! * **Interior content** — rooms, interior surfaces, furniture, interior
//!   installations — folds to the decided LOD (2 or 3). It never collides:
//!   2.0 gave these features no geometry outside LOD4.
//! * **The exterior shell at LOD4** — `lod4Solid` / `lod4MultiSurface` on the
//!   building itself and on its exterior surfaces — folds into the LOD3 slot,
//!   the highest the exterior model has, and only where that slot is empty;
//!   a building that already carries LOD3 keeps it and the LOD4 shell is
//!   dropped and reported.
//!
//! Only LOD4 *geometry* triggers a decision. PLATEAU writes the LOD4 quality
//! descriptors on every building — `geometrySrcDescLod4 = 999` (未作成) on one
//! that has no LOD4 — so a descriptor without geometry describes nothing and
//! is dropped rather than folded into a real LOD.

use crate::error::Result;
use crate::profile::{Lod4Fallback, Lod4Rules, Rules};
use crate::report::Warnings;
use crate::xml::{Element, Name, Node};

/// Solid-valued GML geometries, to decide what a `lod4Geometry` becomes.
const SOLIDS: &[&str] = &["Solid", "CompositeSolid", "MultiSolid"];

/// Quality descriptors that are recorded per LOD and so have an LOD4 form.
///
/// The `Lod0`–`Lod3` counterparts of the first three accept repeats, so
/// folding adds a value. `srcScaleLodN` admits one value, so a folded
/// `srcScaleLod4` yields to an existing one.
const QUALITY_LOD4: &[&str] = &[
    "geometrySrcDescLod4",
    "appearanceSrcDescLod4",
    "publicSurveySrcDescLod4",
    "srcScaleLod4",
];

/// Where one feature's LOD4 content goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    /// Interior content folds to this LOD; the exterior shell to LOD3.
    Lod(char),
    /// Remove LOD4 geometry and descriptors alike.
    Drop,
    /// The feature has LOD4 descriptors but no LOD4 geometry, so there is
    /// nothing to place and the descriptors describe nothing.
    Absent,
}

#[derive(Debug, Clone)]
pub struct Lod4Rewrite {
    core: String,
    bldg: String,
    con: String,
    /// Where the folded quality descriptors are declared in i-UR 4.0.
    urc: Option<String>,
    /// The i-UR namespaces a quality descriptor may still sit in after rename.
    iur: Vec<String>,
    rules: Lod4Rules,
    display: Rules,
}

impl Lod4Rewrite {
    /// `fallback` overrides the profile's when given (the CLI's `--lod4-fallback`).
    pub fn new(rules: &Rules, fallback: Option<Lod4Fallback>) -> Result<Self> {
        let mut lod4 = rules.lod4().clone();
        if let Some(fallback) = fallback {
            lod4.fallback = fallback;
        }
        let mut iur = Vec::new();
        for prefix in ["uro", "urc", "urf", "urg", "urt"] {
            if let Ok(uri) = rules.output_ns(prefix) {
                iur.push(uri.to_owned());
            }
        }
        Ok(Lod4Rewrite {
            core: rules.output_ns("core")?.to_owned(),
            bldg: rules.output_ns("bldg")?.to_owned(),
            con: rules.output_ns("con")?.to_owned(),
            urc: rules.output_ns("urc").ok().map(str::to_owned),
            iur,
            rules: lod4,
            display: rules.clone(),
        })
    }

    /// Rewrites one top-level member and its descendants in place.
    ///
    /// The decision is made once per member from the code it carries and then
    /// applied to every LOD4 element under it: the quality attribute describes
    /// the feature as a whole.
    pub fn apply(&self, member: &mut Element, warnings: &mut Warnings) {
        self.suffix_lod_types(member, false, warnings);
        let target = if has_lod4_geometry(member, &self.bldg) {
            self.decide(member, warnings)
        } else if has_lod4_quality(member, &self.iur) {
            Target::Absent
        } else {
            return;
        };
        if matches!(target, Target::Lod(_)) {
            self.warn_missing_storeys(member, warnings);
        }
        self.rewrite(member, false, target, warnings);
    }

    /// Rewrites the plain 2.0-era `lodType` codes: the i-UR 4.0
    /// `Building_lodType` list tells the exterior and interior models apart
    /// with an `_exterior`/`_interior` suffix instead. A `4.x` value is left
    /// for the LOD4 fold, which knows which interior LOD it became;
    /// everything else classifies the exterior.
    fn suffix_lod_types(&self, el: &mut Element, in_building: bool, warnings: &mut Warnings) {
        let in_building = in_building || el.name.in_ns(&self.bldg);
        let children = std::mem::take(&mut el.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            match child {
                Node::Element(mut child) => {
                    if in_building && self.is_plain_lod_type(&child) {
                        let old = child.text().trim().to_owned();
                        if EXTERIOR_LOD_TYPES.contains(&old.as_str()) {
                            child.children = vec![Node::Text(format!("{old}_exterior"))];
                            warnings.add(format!(
                                "lodType {old} became {old}_exterior: the i-UR 4.0 \
                                 Building_lodType codes classify the exterior and \
                                 interior models separately"
                            ));
                        } else if old == "8" {
                            warnings.add(
                                "lodType 8 (不明) has no entry in the i-UR 4.0 \
                                 Building_lodType code list and was dropped",
                            );
                            continue;
                        } else {
                            warnings.add(format!(
                                "lodType {old} has no entry in the i-UR 4.0 \
                                 Building_lodType code list; the value was kept"
                            ));
                        }
                    }
                    self.suffix_lod_types(&mut child, in_building, warnings);
                    out.push(Node::Element(child));
                }
                other => out.push(other),
            }
        }
        el.children = out;
    }

    /// A `lodType` holding a pre-3.0 plain code — anything but the `4.x`
    /// values the LOD4 fold owns.
    fn is_plain_lod_type(&self, el: &Element) -> bool {
        el.name.local == "lodType"
            && self.iur.iter().any(|ns| el.name.in_ns(ns))
            && !el.text().trim().starts_with('4')
    }

    /// CityGML 3.0 requires `bldg:Storey` in every interior LOD, and CityGML
    /// 2.0 records nothing to build one from — PLATEAU's per-floor
    /// CityObjectGroups name the floors but not which rooms are on them — so
    /// a converted interior model is missing a required feature. That cannot
    /// be fixed here, only said.
    fn warn_missing_storeys(&self, member: &Element, warnings: &mut Warnings) {
        if !has_descendant(member, &self.bldg, "BuildingRoom")
            || has_descendant(member, &self.bldg, "Storey")
        {
            return;
        }
        warnings.add(format!(
            "{}'s interior model has no bldg:Storey, which CityGML 3.0 requires \
             in every interior LOD; CityGML 2.0 records nothing to derive \
             storeys from, so none were created",
            self.feature_name(member)
        ));
    }

    /// Names the feature for diagnostics. A member is usually
    /// `core:cityObjectMember` wrapping the feature; name the feature, which
    /// is what a reader will look for.
    fn feature_name(&self, member: &Element) -> String {
        let feature = member
            .elements()
            .next()
            .filter(|_| member.name.local.starts_with(char::is_lowercase))
            .map(|f| &f.name)
            .unwrap_or(&member.name);
        self.display.display_name(feature)
    }

    /// The interior LOD this member's LOD4 content becomes, from the
    /// measurement code.
    fn decide(&self, member: &Element, warnings: &mut Warnings) -> Target {
        let feature = self.feature_name(member);
        let fallback = self.rules.fallback;

        let Some(attribute) = &self.rules.attribute else {
            warnings.add(format!(
                "{feature} holds LOD4 content but the profile's [lod4] names no \
                 attribute to decide by; LOD4 was handled by the fallback ({})",
                fallback.as_str()
            ));
            return target_from(fallback);
        };
        let mut codes: Vec<String> = Vec::new();
        collect_codes(member, attribute, &mut codes);
        let attribute = self.display.display_name(attribute);

        let Some(code) = codes.first() else {
            warnings.add(format!(
                "{feature} holds LOD4 content but no {attribute} says how it was \
                 measured; LOD4 was handled by the fallback ({})",
                fallback.as_str()
            ));
            return target_from(fallback);
        };
        if codes.len() > 1 {
            warnings.add(format!(
                "{feature} carries several distinct {attribute} codes ({}); the \
                 first decided where its LOD4 content goes",
                codes.join(", ")
            ));
        }

        if self.rules.lod2.contains(code) {
            Target::Lod('2')
        } else if self.rules.lod3.contains(code) {
            Target::Lod('3')
        } else {
            warnings.add(format!(
                "{feature} has {attribute} = {code}, which the profile's [lod4] \
                 table lists under neither lod2 nor lod3; LOD4 was handled by the \
                 fallback ({})",
                fallback.as_str()
            ));
            target_from(fallback)
        }
    }

    /// `interior` says whether `el` belongs to the interior model, so its own
    /// LOD4 children fold to the decided LOD rather than into the exterior's
    /// LOD3 slot.
    fn rewrite(&self, el: &mut Element, interior: bool, target: Target, warnings: &mut Warnings) {
        // Names already filled by something that is not LOD4 content: a fold
        // landing on one of these yields to it.
        let occupied: Vec<Name> = el
            .elements()
            .filter(|c| !self.is_lod4_geometry(c) && !self.is_lod4_quality(c))
            .map(|c| c.name.clone())
            .collect();

        let children = std::mem::take(&mut el.children);
        let mut out: Vec<Node> = Vec::with_capacity(children.len());
        for child in children {
            match child {
                Node::Element(child) => {
                    if let Some(kept) =
                        self.rewrite_child(child, interior, &occupied, &out, target, warnings)
                    {
                        out.push(Node::Element(kept));
                    }
                }
                other => out.push(other),
            }
        }
        el.children = out;

        for child in el.elements_mut() {
            let child_interior = interior || self.marks_interior(&child.name);
            self.rewrite(child, child_interior, target, warnings);
        }
    }

    /// One child of an element: returns it, possibly retagged, or `None` to
    /// remove it.
    fn rewrite_child(
        &self,
        child: Element,
        interior: bool,
        occupied: &[Name],
        accepted: &[Node],
        target: Target,
        warnings: &mut Warnings,
    ) -> Option<Element> {
        if self.is_lod4_geometry(&child) {
            self.fold_geometry(child, interior, occupied, accepted, target, warnings)
        } else if self.is_lod4_quality(&child) {
            self.fold_quality(child, occupied, accepted, target, warnings)
        } else if self.is_lod4_lod_type(&child) {
            self.fold_lod_type(child, target, warnings)
        } else {
            Some(child)
        }
    }

    fn fold_geometry(
        &self,
        child: Element,
        interior: bool,
        occupied: &[Name],
        accepted: &[Node],
        target: Target,
        warnings: &mut Warnings,
    ) -> Option<Element> {
        let from = self.display.display_name(&child.name);
        let lod = match target {
            Target::Drop | Target::Absent => {
                warnings.add(format!(
                    "{from} was dropped: CityGML 3.0 has no LOD4 and the fallback is drop"
                ));
                return None;
            }
            // The exterior model tops out at LOD3 whatever the interior
            // decision; only interior content follows the measurement code.
            Target::Lod(lod) if interior => lod,
            Target::Lod(_) => '3',
        };

        let to = self.geometry_name(&child, lod, warnings);
        let to_display = self.display.display_name(&to);
        if taken(&to, occupied, accepted) {
            warnings.add(format!(
                "{from} was dropped: it would become {to_display}, which the feature \
                 already has, and CityGML 3.0 allows one; the existing one was kept"
            ));
            return None;
        }

        let side = if interior { "interior" } else { "exterior" };
        warnings.add(format!(
            "{from} became {to_display}: CityGML 3.0 has no LOD4, and this is \
             {side} content"
        ));
        let mut child = child;
        child.name = to;
        Some(child)
    }

    fn fold_quality(
        &self,
        child: Element,
        occupied: &[Name],
        accepted: &[Node],
        target: Target,
        warnings: &mut Warnings,
    ) -> Option<Element> {
        let from = self.display.display_name(&child.name);
        let lod = match target {
            Target::Absent => {
                warnings.add(format!(
                    "{from} was dropped: it describes LOD4 content the feature does \
                     not have (PLATEAU writes it on every building, as 999 = 未作成)"
                ));
                return None;
            }
            Target::Drop => {
                warnings.add(format!(
                    "{from} was dropped: CityGML 3.0 has no LOD4 and the fallback is drop"
                ));
                return None;
            }
            Target::Lod(lod) => lod,
        };

        let Some(urc) = &self.urc else {
            warnings.add(format!(
                "{from} was dropped: the profile declares no urc namespace to \
                 hold its LOD{lod} form"
            ));
            return None;
        };
        let to = Name::qualified(urc, child.name.local.replace("Lod4", &format!("Lod{lod}")));
        let to_display = self.display.display_name(&to);

        // srcScaleLodN admits one value; the descriptor lists accept repeats.
        if child.name.local == "srcScaleLod4" && taken(&to, occupied, accepted) {
            warnings.add(format!(
                "{from} was dropped: it would become {to_display}, which the feature \
                 already has, and CityGML 3.0 allows one; the existing one was kept"
            ));
            return None;
        }

        warnings.add(format!(
            "{from} became {to_display}: CityGML 3.0 has no LOD4, and the interior \
             model it described is LOD{lod}"
        ));
        let mut child = child;
        child.name = to;
        Some(child)
    }

    /// `lodType` records the LOD sub-level (`4.1`); a folded interior model's
    /// entry follows it (`2.1` or `3.1`).
    fn fold_lod_type(
        &self,
        child: Element,
        target: Target,
        warnings: &mut Warnings,
    ) -> Option<Element> {
        let old = child.text().trim().to_owned();
        match target {
            Target::Lod(lod) => {
                let new = format!("{lod}{}_interior", &old[1..]);
                warnings.add(format!(
                    "lodType {old} became {new}: CityGML 3.0 has no LOD4, and the \
                     content it classified is the interior model at LOD{lod}"
                ));
                // Keep the element (and its codeSpace); only the value moves.
                let mut child = child;
                child.children = vec![Node::Text(new)];
                Some(child)
            }
            Target::Drop | Target::Absent => {
                warnings.add(format!(
                    "lodType {old} was dropped along with the LOD4 content it classified"
                ));
                None
            }
        }
    }

    /// The 3.0 name for a `bldg:lod4*` geometry property folded into `lod`.
    fn geometry_name(&self, child: &Element, lod: char, warnings: &mut Warnings) -> Name {
        let local = match child.name.local.as_str() {
            "lod4Solid" => format!("lod{lod}Solid"),
            "lod4MultiSurface" => format!("lod{lod}MultiSurface"),
            "lod4MultiCurve" => format!("lod{lod}MultiCurve"),
            "lod4TerrainIntersection" => format!("lod{lod}TerrainIntersectionCurve"),
            "lod4ImplicitRepresentation" => format!("lod{lod}ImplicitRepresentation"),
            // lod4Geometry held any geometry; 3.0 has a slot per kind.
            _ => {
                let inner = child.elements().next().map(|g| g.name.local.clone());
                let solid = inner.as_deref().is_some_and(|g| SOLIDS.contains(&g));
                if !solid && !matches!(inner.as_deref(), Some("MultiSurface")) {
                    warnings.add(format!(
                        "bldg:lod4Geometry held gml:{} and became a multi-surface \
                         property, which expects a gml:MultiSurface; check the result",
                        inner.as_deref().unwrap_or("(nothing)")
                    ));
                }
                if solid {
                    format!("lod{lod}Solid")
                } else {
                    format!("lod{lod}MultiSurface")
                }
            }
        };
        Name::qualified(&self.core, local)
    }

    /// True for an element that puts everything under it in the interior
    /// model: a room, furniture, an interior surface, or the property that
    /// carried an interior installation in 2.0.
    fn marks_interior(&self, name: &Name) -> bool {
        if name.in_ns(&self.bldg) {
            return matches!(
                name.local.as_str(),
                "BuildingRoom"
                    | "buildingRoom"
                    | "BuildingFurniture"
                    | "buildingFurniture"
                    | "interiorBuildingInstallation"
                    | "roomInstallation"
            );
        }
        if name.in_ns(&self.con) {
            return matches!(
                name.local.as_str(),
                "CeilingSurface" | "InteriorWallSurface" | "FloorSurface"
            );
        }
        false
    }

    fn is_lod4_geometry(&self, el: &Element) -> bool {
        el.name.in_ns(&self.bldg) && lod4_geometry(&el.name.local)
    }

    fn is_lod4_quality(&self, el: &Element) -> bool {
        self.iur.iter().any(|ns| el.name.in_ns(ns))
            && QUALITY_LOD4.contains(&el.name.local.as_str())
    }

    fn is_lod4_lod_type(&self, el: &Element) -> bool {
        el.name.local == "lodType"
            && self.iur.iter().any(|ns| el.name.in_ns(ns))
            && el.text().trim().starts_with('4')
    }
}

/// Building lodType codes the published i-UR 4.0 list defines for the
/// exterior model. The 2.0-era plain codes map onto these by suffix; a code
/// outside this set (3.2, 3.3) has no 4.0 home.
const EXTERIOR_LOD_TYPES: &[&str] = &[
    "0.0", "0.1", "1.0", "1.1", "2.0", "2.1", "2.2", "2.3", "3.0", "3.1",
];

fn target_from(fallback: Lod4Fallback) -> Target {
    match fallback {
        Lod4Fallback::Lod3 => Target::Lod('3'),
        Lod4Fallback::Lod2 => Target::Lod('2'),
        Lod4Fallback::Drop => Target::Drop,
    }
}

/// True when `name` is already present among the element's other children.
fn taken(name: &Name, occupied: &[Name], accepted: &[Node]) -> bool {
    occupied.contains(name)
        || accepted
            .iter()
            .any(|n| matches!(n, Node::Element(e) if e.name == *name))
}

/// The `bldg:lod4*` property names CityGML 2.0 declares.
fn lod4_geometry(local: &str) -> bool {
    matches!(
        local,
        "lod4Solid"
            | "lod4MultiSurface"
            | "lod4MultiCurve"
            | "lod4TerrainIntersection"
            | "lod4ImplicitRepresentation"
            | "lod4Geometry"
    )
}

/// True when any `bldg:lod4*` geometry property sits under `el`.
fn has_lod4_geometry(el: &Element, bldg: &str) -> bool {
    el.elements().any(|child| {
        (child.name.in_ns(bldg) && lod4_geometry(&child.name.local))
            || has_lod4_geometry(child, bldg)
    })
}

/// True when any LOD4 quality descriptor sits under `el`.
fn has_lod4_quality(el: &Element, iur: &[String]) -> bool {
    el.elements().any(|child| {
        (iur.iter().any(|ns| child.name.in_ns(ns))
            && QUALITY_LOD4.contains(&child.name.local.as_str()))
            || has_lod4_quality(child, iur)
    })
}

/// True when an element named `(ns, local)` sits anywhere under `el`.
fn has_descendant(el: &Element, ns: &str, local: &str) -> bool {
    el.elements()
        .any(|child| child.is(ns, local) || has_descendant(child, ns, local))
}

/// The text of every `attribute` element under `el`, in document order.
fn collect_codes(el: &Element, attribute: &Name, out: &mut Vec<String>) {
    for child in el.elements() {
        if child.name == *attribute {
            let code = child.text().trim().to_owned();
            if !code.is_empty() && !out.contains(&code) {
                out.push(code);
            }
        }
        collect_codes(child, attribute, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PROFILES;
    use crate::profile::Profile;
    use crate::xml::ns;

    const URO: &str = "https://www.geospatial.jp/iur/uro/4.0";
    const URC: &str = "https://www.geospatial.jp/iur/urc/4.0";

    /// The 3.1 profile with a code table controlled by the tests.
    fn rules() -> Rules {
        rules_with(vec!["L2".into()], vec!["L3".into()]).unwrap()
    }

    fn rules_with(lod2: Vec<String>, lod3: Vec<String>) -> Result<Rules> {
        let mut profile = Profile::load(PROFILES[1].1).unwrap();
        let mut policy = profile.lod4.take().unwrap_or_default();
        policy.lod2 = lod2;
        policy.lod3 = lod3;
        profile.lod4 = Some(policy);
        Rules::compile(&profile)
    }

    fn rewrite(fallback: Option<Lod4Fallback>) -> Lod4Rewrite {
        Lod4Rewrite::new(&rules(), fallback).unwrap()
    }

    fn geometry(local: &str, inner: &str) -> Element {
        let mut prop = Element::new(Name::qualified(ns::BUILDING_3, local));
        prop.push(Element::new(Name::qualified(ns::GML_32, inner)));
        prop
    }

    fn quality(code: Option<&str>) -> Element {
        let mut class = Element::new(Name::qualified(URC, "ExteriorDataQualityAttribute"));
        if let Some(code) = code {
            class.push(Element::with_text(
                Name::qualified(URO, "geometrySrcDescLod4"),
                code,
            ));
        }
        class.push(Element::with_text(
            Name::qualified(URO, "srcScaleLod4"),
            "1",
        ));
        let mut prop = Element::new(Name::qualified(URO, "bldgDataQualityAttribute"));
        prop.push(class);
        prop
    }

    /// A room whose only geometry is LOD4, the shape 2.0 interiors have.
    fn room(children: Vec<Element>) -> Element {
        let mut room = Element::new(Name::qualified(ns::BUILDING_3, "BuildingRoom"));
        for c in children {
            room.push(c);
        }
        let mut prop = Element::new(Name::qualified(ns::BUILDING_3, "buildingRoom"));
        prop.push(room);
        prop
    }

    fn building(children: Vec<Element>) -> Element {
        let mut b = Element::new(Name::qualified(ns::BUILDING_3, "Building"));
        for c in children {
            b.push(c);
        }
        b
    }

    fn locals(el: &Element) -> Vec<String> {
        el.elements().map(|e| e.name.local.clone()).collect()
    }

    fn the_room(b: &Element) -> &Element {
        b.child(ns::BUILDING_3, "buildingRoom")
            .unwrap()
            .elements()
            .next()
            .unwrap()
    }

    #[test]
    fn the_code_decides_the_interior_lod() {
        for (code, lod) in [("L2", "lod2Solid"), ("L3", "lod3Solid")] {
            let mut b = building(vec![
                room(vec![geometry("lod4Solid", "Solid")]),
                quality(Some(code)),
            ]);
            let mut w = Warnings::new();
            rewrite(None).apply(&mut b, &mut w);
            assert!(
                the_room(&b).child(ns::CITYGML_3, lod).is_some(),
                "{code} -> {lod}"
            );
        }
    }

    /// The exterior model tops out at LOD3, so the building's own LOD4 shell
    /// goes there whatever the interior decision — but never displaces an
    /// exterior LOD3 the building already has.
    #[test]
    fn the_exterior_shell_folds_to_lod3_only_where_empty() {
        // Empty slot: the shell fills it, even when the interior goes to LOD2.
        let mut b = building(vec![
            geometry("lod4Solid", "Solid"),
            room(vec![geometry("lod4Solid", "Solid")]),
            quality(Some("L2")),
        ]);
        let mut w = Warnings::new();
        rewrite(None).apply(&mut b, &mut w);
        assert!(b.child(ns::CITYGML_3, "lod3Solid").is_some());
        assert!(the_room(&b).child(ns::CITYGML_3, "lod2Solid").is_some());

        // Occupied slot: the existing LOD3 wins, whichever side of the LOD4
        // element it sits, and the drop is reported.
        for lod3_first in [true, false] {
            let mut existing = Element::new(Name::qualified(ns::CITYGML_3, "lod3Solid"));
            let mut solid = Element::new(Name::qualified(ns::GML_32, "Solid"));
            solid.set_attr(Name::qualified(ns::GML_32, "id"), "original");
            existing.push(solid);
            let mut children = vec![existing, geometry("lod4Solid", "Solid")];
            if !lod3_first {
                children.swap(0, 1);
            }
            children.push(quality(Some("L3")));
            let mut b = building(children);
            let mut w = Warnings::new();
            rewrite(None).apply(&mut b, &mut w);

            let solids: Vec<_> = b
                .elements()
                .filter(|e| e.is(ns::CITYGML_3, "lod3Solid"))
                .collect();
            assert_eq!(solids.len(), 1, "lod3_first={lod3_first}");
            assert_eq!(
                solids[0]
                    .elements()
                    .next()
                    .and_then(|s| s.attr(Some(ns::GML_32), "id")),
                Some("original")
            );
            assert!(w.iter().any(|(m, _)| m.contains("already has")));
        }
    }

    /// An exterior boundary surface's LOD4 geometry is exterior content.
    #[test]
    fn an_exterior_wall_folds_to_lod3_even_when_the_interior_is_lod2() {
        let mut wall = Element::new(Name::qualified(ns::CONSTRUCTION_3, "WallSurface"));
        wall.push(geometry("lod4MultiSurface", "MultiSurface"));
        let mut boundary = Element::new(Name::qualified(ns::CITYGML_3, "boundary"));
        boundary.push(wall);
        let mut b = building(vec![
            boundary,
            room(vec![geometry("lod4Solid", "Solid")]),
            quality(Some("L2")),
        ]);
        rewrite(None).apply(&mut b, &mut Warnings::new());
        let wall = b
            .child(ns::CITYGML_3, "boundary")
            .unwrap()
            .elements()
            .next()
            .unwrap();
        assert!(wall.child(ns::CITYGML_3, "lod3MultiSurface").is_some());
    }

    /// Interior surfaces, interior installations and furniture follow the
    /// interior decision.
    #[test]
    fn interior_content_follows_the_decision() {
        let mut floor = Element::new(Name::qualified(ns::CONSTRUCTION_3, "FloorSurface"));
        floor.push(geometry("lod4MultiSurface", "MultiSurface"));
        let mut boundary = Element::new(Name::qualified(ns::CITYGML_3, "boundary"));
        boundary.push(floor);

        let mut furniture = Element::new(Name::qualified(ns::BUILDING_3, "BuildingFurniture"));
        furniture.push(geometry("lod4Geometry", "Solid"));
        let mut furniture_prop = Element::new(Name::qualified(ns::BUILDING_3, "buildingFurniture"));
        furniture_prop.push(furniture);

        let mut inst = Element::new(Name::qualified(ns::BUILDING_3, "BuildingInstallation"));
        inst.push(geometry("lod4Geometry", "MultiSurface"));
        let mut inst_prop = Element::new(Name::qualified(
            ns::BUILDING_3,
            "interiorBuildingInstallation",
        ));
        inst_prop.push(inst);

        let mut b = building(vec![
            room(vec![
                geometry("lod4Solid", "Solid"),
                boundary,
                furniture_prop,
            ]),
            inst_prop,
            quality(Some("L2")),
        ]);
        rewrite(None).apply(&mut b, &mut Warnings::new());

        let room = the_room(&b);
        assert!(room.child(ns::CITYGML_3, "lod2Solid").is_some());
        let floor = room
            .child(ns::CITYGML_3, "boundary")
            .unwrap()
            .elements()
            .next()
            .unwrap();
        assert!(floor.child(ns::CITYGML_3, "lod2MultiSurface").is_some());
        let furniture = room
            .child(ns::BUILDING_3, "buildingFurniture")
            .unwrap()
            .elements()
            .next()
            .unwrap();
        assert!(furniture.child(ns::CITYGML_3, "lod2Solid").is_some());
        let inst = b
            .child(ns::BUILDING_3, "interiorBuildingInstallation")
            .unwrap()
            .elements()
            .next()
            .unwrap();
        assert!(inst.child(ns::CITYGML_3, "lod2MultiSurface").is_some());
    }

    #[test]
    fn quality_descriptors_and_lod_type_follow_the_interior_decision() {
        let mut class = Element::new(Name::qualified(URC, "ExteriorDataQualityAttribute"));
        class.push(Element::with_text(
            Name::qualified(URO, "geometrySrcDescLod4"),
            "L2",
        ));
        class.push(Element::with_text(
            Name::qualified(URO, "srcScaleLod4"),
            "1",
        ));
        class.push(Element::with_text(Name::qualified(URC, "lodType"), "2.0"));
        class.push(Element::with_text(Name::qualified(URC, "lodType"), "4.1"));
        let mut b = building(vec![room(vec![geometry("lod4Solid", "Solid")]), class]);

        let mut w = Warnings::new();
        rewrite(None).apply(&mut b, &mut w);

        let class = b.elements().nth(1).unwrap();
        assert_eq!(
            class.child(URC, "geometrySrcDescLod2").unwrap().text(),
            "L2"
        );
        assert_eq!(class.child(URC, "srcScaleLod2").unwrap().text(), "1");
        let lod_types: Vec<String> = class
            .elements()
            .filter(|e| e.name.local == "lodType")
            .map(|e| e.text())
            .collect();
        assert_eq!(
            lod_types,
            ["2.0_exterior", "2.1_interior"],
            "4.1 followed the interior decision; 2.0 classifies the exterior"
        );
        assert!(
            w.iter()
                .any(|(m, _)| m.contains("lodType 4.1 became 2.1_interior"))
        );
    }

    /// Plain lodType codes gain the exterior suffix even when the feature has
    /// no LOD4 content at all; codes the 4.0 list dropped are kept or removed.
    #[test]
    fn plain_lod_types_are_suffixed_without_any_lod4() {
        let mut class = Element::new(Name::qualified(URC, "ExteriorDataQualityAttribute"));
        for value in ["2.0", "3.2", "8"] {
            class.push(Element::with_text(Name::qualified(URC, "lodType"), value));
        }
        let mut b = building(vec![class]);

        let mut w = Warnings::new();
        rewrite(None).apply(&mut b, &mut w);

        let class = b.elements().next().unwrap();
        let lod_types: Vec<String> = class
            .elements()
            .filter(|e| e.name.local == "lodType")
            .map(|e| e.text())
            .collect();
        assert_eq!(
            lod_types,
            ["2.0_exterior", "3.2"],
            "2.0 gains the suffix, 3.2 has no 4.0 home and stays, 8 is dropped"
        );
        assert!(
            w.iter()
                .any(|(m, _)| m.contains("lodType 3.2 has no entry"))
        );
        assert!(w.iter().any(|(m, _)| m.contains("lodType 8 (不明)")));
    }

    /// srcScaleLodN admits one value; an existing one wins.
    #[test]
    fn a_folded_src_scale_yields_to_an_existing_one() {
        let mut class = Element::new(Name::qualified(URC, "ExteriorDataQualityAttribute"));
        class.push(Element::with_text(
            Name::qualified(URC, "srcScaleLod3"),
            "2",
        ));
        class.push(Element::with_text(
            Name::qualified(URO, "srcScaleLod4"),
            "1",
        ));
        class.push(Element::with_text(
            Name::qualified(URC, "geometrySrcDescLod3"),
            "a",
        ));
        class.push(Element::with_text(
            Name::qualified(URO, "geometrySrcDescLod4"),
            "L3",
        ));
        let mut b = building(vec![room(vec![geometry("lod4Solid", "Solid")]), class]);
        rewrite(None).apply(&mut b, &mut Warnings::new());
        let class = b.elements().nth(1).unwrap();
        assert_eq!(
            locals(class),
            ["srcScaleLod3", "geometrySrcDescLod3", "geometrySrcDescLod3"],
            "the scale yields, the repeatable descriptor accumulates"
        );
        assert_eq!(class.child(URC, "srcScaleLod3").unwrap().text(), "2");
    }

    #[test]
    fn a_missing_code_takes_the_fallback_and_is_reported() {
        let mut b = building(vec![room(vec![geometry("lod4Solid", "Solid")])]);
        let mut w = Warnings::new();
        rewrite(None).apply(&mut b, &mut w);
        assert!(
            the_room(&b).child(ns::CITYGML_3, "lod3Solid").is_some(),
            "lod3 by default"
        );
        assert!(
            w.iter()
                .any(|(m, _)| m.contains("no uro:geometrySrcDescLod4"))
        );

        let mut b = building(vec![room(vec![geometry("lod4Solid", "Solid")])]);
        rewrite(Some(Lod4Fallback::Lod2)).apply(&mut b, &mut Warnings::new());
        assert!(the_room(&b).child(ns::CITYGML_3, "lod2Solid").is_some());

        let mut b = building(vec![
            room(vec![geometry("lod4Solid", "Solid")]),
            quality(None),
        ]);
        let mut w = Warnings::new();
        rewrite(Some(Lod4Fallback::Drop)).apply(&mut b, &mut w);
        assert!(the_room(&b).children.is_empty(), "the room's LOD4 goes");
        let class = b.elements().nth(1).unwrap().elements().next().unwrap();
        assert!(class.children.is_empty(), "the LOD4 descriptors go too");
        assert!(w.iter().any(|(m, _)| m.contains("dropped")));
    }

    #[test]
    fn an_unlisted_code_takes_the_fallback_and_is_reported() {
        let mut b = building(vec![
            room(vec![geometry("lod4Solid", "Solid")]),
            quality(Some("??")),
        ]);
        let mut w = Warnings::new();
        rewrite(None).apply(&mut b, &mut w);
        assert!(the_room(&b).child(ns::CITYGML_3, "lod3Solid").is_some());
        assert!(w.iter().any(|(m, _)| m.contains("neither lod2 nor lod3")));
    }

    /// PLATEAU writes geometrySrcDescLod4 = 999 on every building. Without
    /// LOD4 geometry there is nothing for it to describe: it goes, and no
    /// decision (and no fallback warning) is made.
    #[test]
    fn lod4_descriptors_without_lod4_geometry_are_dropped_not_folded() {
        let mut b = building(vec![geometry("lod3Solid", "Solid"), quality(Some("999"))]);
        let mut w = Warnings::new();
        rewrite(None).apply(&mut b, &mut w);
        let class = b
            .child(URO, "bldgDataQualityAttribute")
            .unwrap()
            .child(URC, "ExteriorDataQualityAttribute")
            .unwrap();
        assert!(class.children.is_empty(), "{:?}", locals(class));
        assert!(b.child(ns::BUILDING_3, "lod3Solid").is_some(), "untouched");
        assert!(w.iter().any(|(m, _)| m.contains("does not have")));
        assert!(!w.iter().any(|(m, _)| m.contains("neither lod2 nor lod3")));
    }

    /// 3.0 requires bldg:Storey in every interior LOD and 2.0 has nothing to
    /// build one from, so a folded interior is reported as missing it.
    #[test]
    fn a_folded_interior_without_storeys_is_reported() {
        let mut b = building(vec![
            room(vec![geometry("lod4Solid", "Solid")]),
            quality(Some("L3")),
        ]);
        let mut w = Warnings::new();
        rewrite(None).apply(&mut b, &mut w);
        assert!(
            w.iter().any(|(m, _)| m.contains("no bldg:Storey")),
            "{:?}",
            w.iter().collect::<Vec<_>>()
        );

        // No warning when the LOD4 content is exterior-only (no rooms) or
        // when the interior is dropped outright.
        let mut b = building(vec![geometry("lod4Solid", "Solid"), quality(Some("L3"))]);
        let mut w = Warnings::new();
        rewrite(None).apply(&mut b, &mut w);
        assert!(!w.iter().any(|(m, _)| m.contains("no bldg:Storey")));

        let mut b = building(vec![
            room(vec![geometry("lod4Solid", "Solid")]),
            quality(None),
        ]);
        let mut w = Warnings::new();
        rewrite(Some(Lod4Fallback::Drop)).apply(&mut b, &mut w);
        assert!(!w.iter().any(|(m, _)| m.contains("no bldg:Storey")));
    }

    #[test]
    fn a_feature_without_lod4_is_untouched_and_silent() {
        let mut b = building(vec![geometry("lod3Solid", "Solid")]);
        let mut w = Warnings::new();
        rewrite(None).apply(&mut b, &mut w);
        assert_eq!(locals(&b), ["lod3Solid"]);
        assert!(w.is_empty());
    }

    #[test]
    fn a_profile_may_not_list_a_code_twice() {
        assert!(rules_with(vec!["X".into()], vec!["X".into()]).is_err());
    }
}
