//! Folds CityGML 2.0 LOD4 content into the CityGML 3.0 LOD slots.
//!
//! Like [`crate::bldg`] this runs *after* [`crate::transform::rename`] and so
//! speaks 3.0 / i-UR 4.0 names. The profile carries no rename rule for any
//! `lod4*` element, so they reach this pass still spelled `lod4`, such as a
//! `bldg:lod4Solid` in the 3.0 building namespace, and are the only elements
//! this pass touches.
//!
//! One decision is made per top-level member, from the data quality attribute
//! named by the profile's `[lod4]` table. Codes listed under `lod2` send the
//! member's interior model to LOD2, codes under `lod3` to LOD3, and a code
//! that decides neither takes the configured fallback of `lod3`, `lod2` or
//! `drop`.
//!
//! The decision applies differently by position:
//!
//! * **Interior content**, meaning rooms, interior surfaces, furniture and
//!   interior installations, folds to the decided LOD.
//! * **The exterior shell at LOD4**, meaning `lod4Solid` and
//!   `lod4MultiSurface` on the building itself and on its exterior surfaces,
//!   folds into the LOD3 slot and only where that slot is empty.
//!
//! A member whose LOD4 folded and that carries no `4.x` `lodType` code is
//! given the codes its type's `[lod_types]` table lists under `folded`, one
//! per side that received geometry, chosen by whether any of that geometry
//! sits on a boundary surface.
//!
//! A member holding LOD4 quality descriptors but no LOD4 geometry has those
//! descriptors dropped rather than folded. Every fold, drop and fallback is
//! recorded in [`Warnings`].

use crate::error::Result;
use crate::iur::IurModules;
use crate::profile::{Lod4Fallback, Lod4Rules, LodTypeTable, Rules};
use crate::report::Warnings;
use crate::xml::{Element, Name, Node};

/// GML geometry types that count as solids when naming a folded `lod4Geometry`.
const SOLIDS: &[&str] = &["Solid", "CompositeSolid", "MultiSolid"];

/// Where one feature's LOD4 content goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    /// Interior content folds to this LOD. The exterior shell folds to LOD3.
    Lod(char),
    /// Remove LOD4 geometry and descriptors alike.
    Drop,
    /// The feature carries LOD4 descriptors but no LOD4 geometry.
    Absent,
}

/// What one member's fold placed, per side. `Some` when geometry of that side
/// folded, holding whether any of it sits on a boundary surface.
#[derive(Debug, Default)]
struct Folded {
    exterior: Option<bool>,
    interior: Option<bool>,
}

impl Folded {
    fn record(&mut self, interior: bool, surface: bool) {
        let side = if interior {
            &mut self.interior
        } else {
            &mut self.exterior
        };
        *side = Some(side.unwrap_or(false) || surface);
    }
}

#[derive(Debug, Clone)]
pub struct Lod4Rewrite {
    core: String,
    /// The properties whose child is a boundary surface.
    surface_properties: Vec<Name>,
    /// Namespace the folded quality descriptors are written into, when the
    /// profile declares one.
    urc: Option<String>,
    /// The i-UR namespaces a quality descriptor may still sit in after rename.
    iur: IurModules,
    rules: Lod4Rules,
    display: Rules,
}

impl Lod4Rewrite {
    /// `fallback` overrides the profile's `[lod4]` fallback when given.
    pub fn new(rules: &Rules, fallback: Option<Lod4Fallback>) -> Result<Self> {
        let mut lod4 = rules.lod4().clone();
        if let Some(fallback) = fallback {
            lod4.fallback = fallback;
        }
        let core = rules.output_ns("core")?;
        let mut surface_properties = vec![Name::qualified(core, "boundary")];
        if let Ok(con) = rules.output_ns("con") {
            surface_properties.push(Name::qualified(con, "fillingSurface"));
        }
        Ok(Lod4Rewrite {
            core: core.to_owned(),
            surface_properties,
            urc: rules.output_ns("urc").ok().map(str::to_owned),
            iur: IurModules::new(rules),
            rules: lod4,
            display: rules.clone(),
        })
    }

    /// Rewrites one top-level member and its descendants in place.
    ///
    /// One decision is taken from the code `member` carries and applied to
    /// every LOD4 element under it. Returns true when an interior model was
    /// folded to an LOD rather than dropped.
    pub fn apply(&self, member: &mut Element, warnings: &mut Warnings) -> bool {
        self.map_lod_types(member, None, warnings);
        let target = if self.has_lod4_geometry(member) {
            match self.rules.fixed.get(&feature(member).name) {
                Some(fixed) => {
                    warnings.add(format!(
                        "{} holds LOD4 content, which the profile's [lod4.fixed] sends to {} \
                         whatever its measurement code",
                        self.feature_name(member),
                        fixed.as_str()
                    ));
                    target_from(*fixed)
                }
                None => self.decide(member, warnings),
            }
        } else if has_lod4_quality(member, &self.iur) {
            Target::Absent
        } else {
            return false;
        };
        let coded = self.has_lod4_lod_type(member);
        let mut folded = Folded::default();
        self.rewrite(member, false, false, target, &mut folded, warnings);
        if let (Target::Lod(lod), false) = (target, coded) {
            self.add_folded_lod_types(member, lod, &folded, warnings);
        }
        matches!(target, Target::Lod(_))
    }

    /// Rewrites plain 2.0-era `lodType` codes under `el` by the profile's
    /// `[lod_types]` table of the feature they sit in, and points their
    /// `codeSpace` at that table's list.
    ///
    /// `host` is the table of the outermost feature `el` already sits in, if
    /// any. A `4.x` value is left for [`Self::fold_lod_type`], and a value
    /// already one of the table's published codes is left as it is.
    fn map_lod_types<'a>(
        &'a self,
        el: &mut Element,
        host: Option<&'a (Name, LodTypeTable)>,
        warnings: &mut Warnings,
    ) {
        let host = host.or_else(|| self.display.lod_types(&el.name));
        let children = std::mem::take(&mut el.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            match child {
                Node::Element(mut child) => {
                    if let Some(host) = host.filter(|_| self.is_lod_type(&child)) {
                        self.retarget_lod_type(&mut child, host, warnings);
                    }
                    let pending = host.filter(|(_, table)| {
                        self.is_plain_lod_type(&child)
                            && !table.map.values().any(|c| c == child.text().trim())
                    });
                    if let Some((_, table)) = pending {
                        let old = child.text().trim().to_owned();
                        let list = list_stem(&table.list);
                        if let Some(new) = table.map.get(&old) {
                            warnings.add(format!(
                                "lodType {old} became {new}, its code in the i-UR 4.0 \
                                 {list} code list"
                            ));
                            child.children = vec![Node::Text(new.clone())];
                        } else if let Some(label) = table.drop.get(&old) {
                            warnings.add(format!(
                                "lodType {old} ({label}) has no entry in the i-UR 4.0 \
                                 {list} code list and was dropped"
                            ));
                            continue;
                        } else {
                            warnings.add(format!(
                                "lodType {old} has no entry in the i-UR 4.0 {list} \
                                 code list; the value was kept"
                            ));
                        }
                    }
                    self.map_lod_types(&mut child, host, warnings);
                    out.push(Node::Element(child));
                }
                other => out.push(other),
            }
        }
        el.children = out;
    }

    /// Points a `lodType`'s `codeSpace` at `host`'s list when it names one of
    /// the lists the table says 2.0 data cites instead.
    fn retarget_lod_type(
        &self,
        el: &mut Element,
        (host, table): &(Name, LodTypeTable),
        warnings: &mut Warnings,
    ) {
        let Some(value) = el.attr(None, "codeSpace") else {
            return;
        };
        let (dir, file) = match value.rsplit_once('/') {
            Some((dir, file)) => (Some(dir), file),
            None => (None, value),
        };
        if !table.cited_as.iter().any(|c| c == file) {
            return;
        }
        let file = file.to_owned();
        let path = match dir {
            Some(dir) => format!("{dir}/{}", table.list),
            None => table.list.clone(),
        };
        el.set_attr(Name::unqualified("codeSpace"), path);
        warnings.add(format!(
            "codeSpace {file} became {} on {}: i-UR 4.0 publishes its lodType codes \
             as a list of their own",
            table.list,
            self.display.display_name(host)
        ));
    }

    /// An i-UR `lodType`, whatever code it holds.
    fn is_lod_type(&self, el: &Element) -> bool {
        el.name.local == "lodType" && self.iur.contains(&el.name)
    }

    /// A `lodType` holding a pre-3.0 plain code, meaning anything but the
    /// `4.x` values the LOD4 fold owns.
    fn is_plain_lod_type(&self, el: &Element) -> bool {
        self.is_lod_type(el) && !el.text().trim().starts_with('4')
    }

    /// The name to show for `member` in diagnostics. A lowercase-named member
    /// is a property wrapping the feature, and the feature is named instead.
    fn feature_name(&self, member: &Element) -> String {
        self.display.display_name(&feature(member).name)
    }

    /// The interior LOD this member's LOD4 content becomes, read from the
    /// measurement code it carries.
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

    /// Folds the LOD4 children of `el` and of its descendants. `interior`
    /// says whether `el` belongs to the interior model, in which case its own
    /// LOD4 children fold to `target` rather than into the exterior LOD3 slot.
    /// `surface` says whether `el` is a boundary surface. Each geometry
    /// property folded is recorded in `folded`.
    fn rewrite(
        &self,
        el: &mut Element,
        interior: bool,
        surface: bool,
        target: Target,
        folded: &mut Folded,
        warnings: &mut Warnings,
    ) {
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
                    let geometry = self.is_lod4_geometry(&child);
                    if let Some(kept) =
                        self.rewrite_child(child, interior, &occupied, &out, target, warnings)
                    {
                        if geometry {
                            folded.record(interior, surface);
                        }
                        out.push(Node::Element(kept));
                    }
                }
                other => out.push(other),
            }
        }
        el.children = out;

        let child_surface = self.surface_properties.contains(&el.name);
        for child in el.elements_mut() {
            let child_interior = interior || self.marks_interior(&child.name);
            self.rewrite(
                child,
                child_interior,
                child_surface,
                target,
                folded,
                warnings,
            );
        }
    }

    /// Adds to `member`'s feature the `folded` codes of its `[lod_types]`
    /// table for each side `folded` records, the interior one keyed by `lod`
    /// and the exterior one by LOD3. A code goes next to the feature's first
    /// `lodType`, or else among its first quality descriptors, and is skipped
    /// when already present.
    fn add_folded_lod_types(
        &self,
        member: &mut Element,
        lod: char,
        folded: &Folded,
        warnings: &mut Warnings,
    ) {
        let feature = feature_mut(member);
        let Some((_, table)) = self.display.lod_types(&feature.name) else {
            return;
        };
        let label = self.display.display_name(&feature.name);
        let sides = [
            ("exterior", '3', folded.exterior, &table.folded.exterior),
            ("interior", lod, folded.interior, &table.folded.interior),
        ];
        for (side, lod, split, codes) in sides {
            let (Some(split), Some(codes)) = (split, codes.get(&lod.to_string())) else {
                continue;
            };
            let code = if split { &codes.split } else { &codes.whole };
            let Some(path) = self.lod_type_holder(feature) else {
                warnings.add(format!(
                    "the {label} was given no lodType {code} for its folded LOD4 {side}: it \
                     carries no data quality attribute to hold one"
                ));
                continue;
            };
            let holder = path.iter().fold(&mut *feature, |el, &i| {
                el.elements_mut().nth(i).expect("a path just found")
            });
            if holder
                .elements()
                .any(|c| self.is_lod_type(c) && c.text().trim() == code)
            {
                continue;
            }
            holder.push(self.new_lod_type(holder, code, &table.list));
            let surfaces = if split { "with" } else { "without" };
            warnings.add(format!(
                "lodType {code} was added to the {label}: its LOD4 {side} became LOD{lod} \
                 {surfaces} boundary surfaces, and it carried no code of its own"
            ));
        }
    }

    /// The child indices from `el` down to the first element holding a
    /// `lodType`, or else the first holding a quality descriptor.
    fn lod_type_holder(&self, el: &Element) -> Option<Vec<usize>> {
        let lod_type = |e: &Element| e.elements().any(|c| self.is_lod_type(c));
        let descriptor = |e: &Element| {
            e.elements()
                .any(|c| self.iur.quality_descriptor(&c.name).is_some())
        };
        path_to(el, &lod_type).or_else(|| path_to(el, &descriptor))
    }

    /// A `lodType` holding `code`, named and with its `codeSpace` taken from
    /// a `lodType` in `holder`, or else from a quality descriptor there, with
    /// the code list `list`.
    fn new_lod_type(&self, holder: &Element, code: &str, list: &str) -> Element {
        let sibling = holder.elements().find(|c| self.is_lod_type(c));
        let (name, code_space) = match sibling {
            Some(sibling) => (
                sibling.name.clone(),
                sibling.attr(None, "codeSpace").map(str::to_owned),
            ),
            None => {
                let descriptor = holder
                    .elements()
                    .find(|c| self.iur.quality_descriptor(&c.name).is_some());
                let ns = descriptor
                    .and_then(|d| d.name.ns.clone())
                    .or_else(|| self.urc.clone())
                    .unwrap_or_default();
                let dir = descriptor
                    .and_then(|d| d.attr(None, "codeSpace"))
                    .and_then(|c| c.rsplit_once('/'))
                    .map(|(dir, _)| format!("{dir}/"));
                (
                    Name::qualified(&ns, "lodType"),
                    dir.map(|dir| format!("{dir}{list}")),
                )
            }
        };
        let mut el = Element::with_text(name, code);
        if let Some(code_space) = code_space {
            el.set_attr(Name::unqualified("codeSpace"), code_space);
        }
        el
    }

    /// True when a `4.x` `lodType` sits under `el`.
    fn has_lod4_lod_type(&self, el: &Element) -> bool {
        el.elements()
            .any(|child| self.is_lod4_lod_type(child) || self.has_lod4_lod_type(child))
    }

    /// Rewrites one child, returning it possibly retagged, or `None` to
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
                     not have (PLATEAU writes it on every feature, as 999 = 未作成)"
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

    /// Rewrites a `4.x` `lodType` value to the interior form of the decided
    /// LOD, turning `4.1` into `2.1_interior` or `3.1_interior`.
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

    /// The 3.0 name for a `lod4*` geometry property folded into `lod`.
    fn geometry_name(&self, child: &Element, lod: char, warnings: &mut Warnings) -> Name {
        let local = match child.name.local.as_str() {
            "lod4Solid" => format!("lod{lod}Solid"),
            "lod4MultiSurface" => format!("lod{lod}MultiSurface"),
            "lod4MultiCurve" => format!("lod{lod}MultiCurve"),
            "lod4TerrainIntersection" => format!("lod{lod}TerrainIntersectionCurve"),
            "lod4ImplicitRepresentation" => format!("lod{lod}ImplicitRepresentation"),
            _ => {
                let inner = child.elements().next().map(|g| g.name.local.clone());
                let solid = inner.as_deref().is_some_and(|g| SOLIDS.contains(&g));
                if !solid && !matches!(inner.as_deref(), Some("MultiSurface")) {
                    warnings.add(format!(
                        "{} held gml:{} and became a multi-surface property, which \
                         expects a gml:MultiSurface; check the result",
                        self.display.display_name(&child.name),
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
    /// model, namely: a room, furniture, an interior surface, or the property
    /// that carried an interior installation in 2.0.
    fn marks_interior(&self, name: &Name) -> bool {
        self.rules.interior.contains(name)
    }

    /// True for a `lod4*` geometry property of one of the profile's modules.
    fn is_lod4_geometry(&self, el: &Element) -> bool {
        lod4_geometry(&el.name.local) && self.rules.modules.iter().any(|m| el.name.in_ns(m))
    }

    /// True when any LOD4 geometry property sits under `el`.
    fn has_lod4_geometry(&self, el: &Element) -> bool {
        el.elements()
            .any(|child| self.is_lod4_geometry(child) || self.has_lod4_geometry(child))
    }

    fn is_lod4_quality(&self, el: &Element) -> bool {
        is_lod4_quality(&self.iur, el)
    }

    fn is_lod4_lod_type(&self, el: &Element) -> bool {
        el.name.local == "lodType"
            && self.iur.contains(&el.name)
            && el.text().trim().starts_with('4')
    }
}

/// The feature `member` holds, looking through a lowercase-named property
/// wrapping it.
fn feature(member: &Element) -> &Element {
    member
        .elements()
        .next()
        .filter(|_| member.name.local.starts_with(char::is_lowercase))
        .unwrap_or(member)
}

/// [`feature`], mutably.
fn feature_mut(member: &mut Element) -> &mut Element {
    if member.name.local.starts_with(char::is_lowercase) && member.elements().next().is_some() {
        return member.elements_mut().next().expect("checked above");
    }
    member
}

/// The child indices from `el` down to the first descendant, `el` included,
/// for which `found` holds.
fn path_to(el: &Element, found: &dyn Fn(&Element) -> bool) -> Option<Vec<usize>> {
    if found(el) {
        return Some(Vec::new());
    }
    el.elements().enumerate().find_map(|(i, child)| {
        path_to(child, found).map(|mut path| {
            path.insert(0, i);
            path
        })
    })
}

/// A code list's file name without its extension, for diagnostics.
fn list_stem(list: &str) -> &str {
    list.strip_suffix(".xml").unwrap_or(list)
}

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

/// The `lod4*` property names CityGML 2.0 declares.
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

/// True when any LOD4 quality descriptor sits under `el`.
fn has_lod4_quality(el: &Element, iur: &IurModules) -> bool {
    el.elements()
        .any(|child| is_lod4_quality(iur, child) || has_lod4_quality(child, iur))
}

/// True when `el` is a LOD4 quality descriptor.
fn is_lod4_quality(iur: &IurModules, el: &Element) -> bool {
    matches!(iur.quality_descriptor(&el.name), Some((_, '4')))
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
        let policy = &mut profile.lod4;
        policy.lod2 = lod2;
        policy.lod3 = lod3;

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

    /// A folded bridge gains one code per side, `split` for the side whose
    /// geometry sits on a boundary surface, here the exterior wall reached
    /// through `core:boundary`, and `whole` for the side of bare room solids.
    #[test]
    fn folded_codes_follow_whether_each_side_has_boundary_surfaces() {
        let brid =
            |local: &str| Name::qualified("http://www.opengis.net/citygml/bridge/3.0", local);
        let lod4 = |inner: &str| {
            let mut prop = Element::new(brid("lod4MultiSurface"));
            prop.push(Element::new(Name::qualified(ns::GML_32, inner)));
            prop
        };
        let mut wall = Element::new(Name::qualified(ns::CONSTRUCTION_3, "WallSurface"));
        wall.push(lod4("MultiSurface"));
        let mut boundary = Element::new(Name::qualified(ns::CITYGML_3, "boundary"));
        boundary.push(wall);
        let mut room = Element::new(brid("BridgeRoom"));
        let mut solid = Element::new(brid("lod4Solid"));
        solid.push(Element::new(Name::qualified(ns::GML_32, "Solid")));
        room.push(solid);
        let mut rooms = Element::new(brid("bridgeRoom"));
        rooms.push(room);
        let mut bridge = Element::new(brid("Bridge"));
        bridge.push(boundary);
        bridge.push(rooms);
        bridge.push(quality(Some("L2")));

        rewrite(None).apply(&mut bridge, &mut Warnings::new());

        let mut codes = Vec::new();
        collect_codes(&bridge, &Name::qualified(URC, "lodType"), &mut codes);
        assert_eq!(codes, ["3.1_exterior", "3.0_interior"]);
    }

    /// The building's own LOD4 shell folds to LOD3 whatever the interior
    /// decision, and never displaces an exterior LOD3 already present.
    #[test]
    fn the_exterior_shell_folds_to_lod3_only_where_empty() {
        let mut b = building(vec![
            geometry("lod4Solid", "Solid"),
            room(vec![geometry("lod4Solid", "Solid")]),
            quality(Some("L2")),
        ]);
        let mut w = Warnings::new();
        rewrite(None).apply(&mut b, &mut w);
        assert!(b.child(ns::CITYGML_3, "lod3Solid").is_some());
        assert!(the_room(&b).child(ns::CITYGML_3, "lod2Solid").is_some());

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
    /// no LOD4 content. Codes the 4.0 list dropped are kept or removed.
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

    /// An underground building cites its own list, which has no exterior
    /// LOD2 codes, so those are dropped while the rest gain the suffix.
    #[test]
    fn an_underground_building_uses_its_own_lod_type_list() {
        let mut class = Element::new(Name::qualified(URC, "ExteriorDataQualityAttribute"));
        for value in ["1.0", "2.1", "3.0", "4.1"] {
            let mut lod_type = Element::with_text(Name::qualified(URC, "lodType"), value);
            lod_type.set_attr(
                Name::unqualified("codeSpace"),
                "../../codelists/Building_lodType.xml",
            );
            class.push(lod_type);
        }
        let mut b = Element::new(Name::qualified(URO, "UndergroundBuilding"));
        b.push(class);
        b.push(room(vec![geometry("lod4Solid", "Solid")]));

        let mut w = Warnings::new();
        rewrite(None).apply(&mut b, &mut w);

        let class = b.elements().next().unwrap();
        let lod_types: Vec<_> = class
            .elements()
            .filter(|e| e.name.local == "lodType")
            .collect();
        assert_eq!(
            lod_types.iter().map(|e| e.text()).collect::<Vec<_>>(),
            [
                "1.0_exterior",
                "2.1_exterior",
                "3.0_exterior",
                "3.1_interior"
            ],
            "plain codes map as on a building, 4.1 folds"
        );
        for lod_type in lod_types {
            assert_eq!(
                lod_type.attr(None, "codeSpace"),
                Some("../../codelists/UndergroundBuilding_lodType.xml")
            );
        }
        assert!(
            w.iter()
                .any(|(m, _)| m.starts_with("lodType 2.1 became 2.1_exterior"))
        );
    }

    /// srcScaleLodN admits one value, and an existing one wins.
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

    /// A LOD4 descriptor on a feature with no LOD4 geometry is dropped, with
    /// no decision and no fallback warning.
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

    #[test]
    fn a_profile_may_not_list_a_code_twice() {
        assert!(rules_with(vec!["X".into()], vec!["X".into()]).is_err());
    }
}
