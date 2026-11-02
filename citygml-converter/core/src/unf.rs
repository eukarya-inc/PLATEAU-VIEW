//! Structural rewrites for the underground utility features that a rename
//! table cannot express.
//!
//! Runs after [`crate::transform::rename`] and before [`crate::lod4`], so every
//! name here is CityGML 3.0 and i-UR 4.0. An underground utility is an i-UR
//! element whose type the target schemas derive from `frn:CityFurniture`.
//!
//! ```text
//! 2.0   <uro:Manhole>
//!         <frn:lod2Geometry><gml:Solid>…</…></…>
//!         <frn:lod3Geometry><gml:MultiSurface>…</…></…>
//!         <frn:lod4Geometry><gml:MultiSurface>…</…></…>
//!         <uro:frnDmAttribute><uro:DmGeometricAttribute>
//!           <uro:lod0Geometry><gml:Point>…</…></…>
//!         </…></…>
//!
//! 3.0   <uro:Manhole>
//!         <core:lod0Point><gml:Point>…</…></…>
//!         <core:lod2Solid><gml:Solid>…</…></…>
//!         <core:lod2MultiSurface><gml:MultiSurface>…</…></…>
//!       </uro:Manhole>
//! ```
//!
//! Per feature it does the following:
//!
//! * moves `frn:lodNGeometry`, `frn:lodNImplicitRepresentation` and
//!   `frn:lodNTerrainIntersection` into the `core` property for the LOD and,
//!   for geometry, the GML type it holds, with LOD3 written as LOD2
//! * removes every LOD4 property and records the `gml:id`s under it
//! * writes the `lod0Geometry` of the first Dm attribute that carries one as
//!   the feature's `core:lod0Point`, `core:lod0MultiCurve` or
//!   `core:lod0MultiSurface`
//! * removes or keeps the Dm attributes as [`UnfDm`] says
//! * writes the LOD3 data quality descriptors as LOD2 and removes the LOD4
//!   ones
//!
//! A geometry with no property to go to, and a property whose slot is already
//! taken, are removed. Every removal is reported.

use std::collections::HashSet;

use crate::error::Result;
use crate::profile::{Rules, UnfDm};
use crate::report::Warnings;
use crate::schema::ChildOrder;
use crate::xml::{Element, Name, Node};

/// The per-LOD data quality descriptors, by stem.
const QUALITY_STEMS: &[&str] = &[
    "geometrySrcDesc",
    "appearanceSrcDesc",
    "publicSurveySrcDesc",
    "srcScale",
];

const SOLIDS: &[&str] = &["Solid", "CompositeSolid"];
const SURFACES: &[&str] = &[
    "Polygon",
    "Surface",
    "CompositeSurface",
    "OrientableSurface",
];
const CURVES: &[&str] = &["LineString", "Curve", "CompositeCurve", "OrientableCurve"];

/// The three LOD-indexed property families of CityGML 2.0 city furniture.
#[derive(Debug, Clone, Copy)]
enum Family {
    Geometry,
    Implicit,
    Terrain,
}

#[derive(Debug, Clone)]
pub struct UnfRewrite {
    core: String,
    frn: String,
    gml: String,
    /// The i-UR namespaces an element may sit in after rename.
    iur: Vec<String>,
    /// `frn:adeOfCityFurniture`, which every underground utility declares.
    furniture_hook: Name,
    schema: ChildOrder,
    dm: UnfDm,
    /// The codes the published `Common_dmCode.xml` defines.
    dm_codes: HashSet<String>,
    display: Rules,
}

impl UnfRewrite {
    /// `schema` holds the target schemas' content models. `dm` overrides the
    /// profile's `[unf] dm` when given.
    pub fn new(rules: &Rules, schema: &ChildOrder, dm: Option<UnfDm>) -> Result<Self> {
        let mut iur = Vec::new();
        for prefix in ["uro", "urc", "urf", "urg", "urt"] {
            if let Ok(uri) = rules.output_ns(prefix) {
                iur.push(uri.to_owned());
            }
        }
        let frn = rules.output_ns("frn")?.to_owned();
        let dm_codes = crate::CODELISTS_4_0
            .iter()
            .find(|(name, _)| *name == "Common_dmCode.xml")
            .map(|(_, text)| crate::convert::definition_codes(text).into_iter().collect())
            .unwrap_or_default();
        Ok(UnfRewrite {
            core: rules.output_ns("core")?.to_owned(),
            gml: rules.output_ns("gml")?.to_owned(),
            furniture_hook: Name::qualified(&frn, "adeOfCityFurniture"),
            frn,
            iur,
            schema: schema.clone(),
            dm: dm.unwrap_or(rules.unf().dm),
            dm_codes,
            display: rules.clone(),
        })
    }

    /// Rewrites one top-level member in place, looking through a property
    /// that wraps the feature. The `gml:id` of every element removed with an
    /// LOD4 property is added to `dropped`. Returns true when anything was
    /// added.
    pub fn apply(
        &self,
        member: &mut Element,
        dropped: &mut HashSet<String>,
        warnings: &mut Warnings,
    ) -> bool {
        let before = dropped.len();
        if self.is_feature(member) {
            self.feature(member, dropped, warnings);
        } else {
            for child in member.elements_mut() {
                if self.is_feature(child) {
                    self.feature(child, dropped, warnings);
                }
            }
        }
        dropped.len() > before
    }

    fn is_feature(&self, el: &Element) -> bool {
        self.is_iur(&el.name) && self.schema.declares(&el.name, &self.furniture_hook)
    }

    fn feature(
        &self,
        feature: &mut Element,
        dropped: &mut HashSet<String>,
        warnings: &mut Warnings,
    ) {
        let label = self.display.display_name(&feature.name);
        let children = std::mem::take(&mut feature.children);
        let mut out = Vec::with_capacity(children.len());
        let mut geometry: Vec<Element> = Vec::new();
        let mut dm: Vec<Element> = Vec::new();
        for child in children {
            let Node::Element(child) = child else {
                out.push(child);
                continue;
            };
            if let Some((family, lod)) = self.lod_property(&child) {
                if lod == '4' {
                    collect_ids(&child, &self.gml, dropped);
                    warnings.add(format!(
                        "{} on the {label} was dropped. CityGML 3.0 has no LOD4, and an \
                         underground utility's LOD4 holds its exterior and interior faces \
                         in one geometry that cannot be split",
                        self.display.display_name(&child.name)
                    ));
                    continue;
                }
                if let Some(property) = self.retag(child, family, lod, &label, warnings) {
                    if geometry.iter().any(|g| g.name == property.name) {
                        warnings.add(format!(
                            "a second {} on the {label} was dropped. The property holds one \
                             geometry",
                            self.display.display_name(&property.name)
                        ));
                    } else {
                        geometry.push(property);
                    }
                }
                continue;
            }
            if self.is_dm(&child) {
                dm.push(child);
                continue;
            }
            out.push(Node::Element(child));
        }

        let has_dm_geometry = dm
            .iter()
            .flat_map(|property| property.elements())
            .any(|class| self.lod0_geometry(class).is_some());
        let lod0 = self.lod0(&mut dm, &label, warnings);
        match lod0 {
            Some(property) => geometry.insert(0, property),
            None if has_dm_geometry => {}
            None => warnings.add(format!(
                "the {label} has no LOD0, which CityGML 3.0 requires. The source carries \
                 no Dm geometry to take it from, so none was written"
            )),
        }
        match self.dm {
            UnfDm::Drop => {
                for property in &dm {
                    warnings.add(format!(
                        "{} on the {label} was dropped, as [unf] dm = drop says",
                        self.display.display_name(&property.name)
                    ));
                }
            }
            UnfDm::Keep => {
                for property in dm {
                    self.check_dm_codes(&property, warnings);
                    out.push(Node::Element(property));
                }
            }
        }
        out.extend(geometry.into_iter().map(Node::Element));
        feature.children = out;
        self.renumber_quality(feature, &label, warnings);
    }

    /// The family and LOD digit of an `frn:lodN*` property.
    fn lod_property(&self, el: &Element) -> Option<(Family, char)> {
        if !el.name.in_ns(&self.frn) {
            return None;
        }
        let rest = el.name.local.strip_prefix("lod")?;
        let mut chars = rest.chars();
        let lod = chars.next().filter(|c| ('1'..='4').contains(c))?;
        let family = match chars.as_str() {
            "Geometry" => Family::Geometry,
            "ImplicitRepresentation" => Family::Implicit,
            "TerrainIntersection" => Family::Terrain,
            _ => return None,
        };
        Some((family, lod))
    }

    /// The `core` property `property` becomes, or `None` when its content
    /// has no property at that LOD.
    fn retag(
        &self,
        mut property: Element,
        family: Family,
        lod: char,
        label: &str,
        warnings: &mut Warnings,
    ) -> Option<Element> {
        let from = self.display.display_name(&property.name);
        let to_lod = if lod == '3' { '2' } else { lod };
        let local = match family {
            Family::Implicit => format!("lod{to_lod}ImplicitRepresentation"),
            Family::Terrain => format!("lod{to_lod}TerrainIntersectionCurve"),
            Family::Geometry => {
                let Some(inner) = property.elements().next() else {
                    warnings.add(format!(
                        "{from} on the {label} held no geometry and was dropped"
                    ));
                    return None;
                };
                let kind = inner.name.local.clone();
                let target = if !inner.name.in_ns(&self.gml) {
                    None
                } else if SOLIDS.contains(&kind.as_str()) {
                    Some("Solid")
                } else if to_lod == '1' {
                    None
                } else {
                    multi_of(&kind)
                };
                let Some(target) = target else {
                    warnings.add(format!(
                        "{from} on the {label} held a gml:{kind}, for which CityGML 3.0 has \
                         no LOD{to_lod} property, and was dropped"
                    ));
                    return None;
                };
                if target != "Solid" {
                    self.wrap_single(&mut property, target);
                }
                format!("lod{to_lod}{target}")
            }
        };
        let to = Name::qualified(&self.core, local);
        if lod == '3' {
            warnings.add(format!(
                "{from} became {}. Its definition is that of exterior LOD2.1, and CityGML \
                 3.0's LOD3 is kept for CAD-derived shapes",
                self.display.display_name(&to)
            ));
        }
        property.name = to;
        Some(property)
    }

    /// Wraps the single geometry under `property` in a `gml:<multi>` unless it
    /// already is one.
    fn wrap_single(&self, property: &mut Element, multi: &str) {
        let Some(Node::Element(inner)) = property
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(_)))
            .map(|i| property.children.remove(i))
        else {
            return;
        };
        if inner.is(&self.gml, multi) {
            property.push(inner);
            return;
        }
        let member = if multi == "MultiSurface" {
            "surfaceMember"
        } else {
            "curveMember"
        };
        let mut wrapper = Element::new(Name::qualified(&self.gml, multi));
        for attr in ["srsName", "srsDimension"] {
            if let Some(value) = inner.attr(None, attr) {
                wrapper.set_attr(Name::unqualified(attr), value);
            }
        }
        let mut item = Element::new(Name::qualified(&self.gml, member));
        item.push(inner);
        wrapper.push(item);
        property.push(wrapper);
    }

    /// True when `el` is a property holding one Dm class.
    fn is_dm(&self, el: &Element) -> bool {
        if !self.is_iur(&el.name) || el.name.local.starts_with(char::is_uppercase) {
            return false;
        }
        let mut classes = el.elements();
        match (classes.next(), classes.next()) {
            (Some(class), None) => {
                self.is_iur(&class.name)
                    && matches!(
                        class.name.local.as_str(),
                        "DmGeometricAttribute" | "DmAnnotation"
                    )
            }
            _ => false,
        }
    }

    /// The feature's LOD0 property, built from the `lod0Geometry` of the
    /// first Dm attribute in `dm` that carries one. Under [`UnfDm::Drop`] the
    /// geometry is moved out, and under [`UnfDm::Keep`] it is copied without
    /// its `gml:id`s. A Dm geometry that is an attached figure or a
    /// supplementary symbol is never used, and a feature left with none to
    /// use is reported.
    fn lod0(&self, dm: &mut [Element], label: &str, warnings: &mut Warnings) -> Option<Element> {
        let annex_only = dm
            .iter()
            .flat_map(|property| property.elements())
            .all(|class| self.lod0_geometry(class).is_none() || self.is_annex(class));
        let mut sources = dm
            .iter_mut()
            .filter_map(|property| property.elements_mut().next())
            .filter(|class| {
                class.name.local == "DmGeometricAttribute"
                    && !self.is_annex(class)
                    && self.lod0_geometry(class).is_some()
            });
        let Some(class) = sources.next() else {
            if !annex_only {
                return None;
            }
            if dm
                .iter()
                .flat_map(|p| p.elements())
                .any(|c| self.lod0_geometry(c).is_some())
            {
                warnings.add(format!(
                    "every Dm geometry of the {label} is an attached figure or a \
                     supplementary symbol, so the {label} got no LOD0"
                ));
            }
            return None;
        };
        if sources.next().is_some() {
            warnings.add(format!(
                "the {label} carries several Dm geometries. The first became its LOD0"
            ));
        }

        let source = self.lod0_geometry(class)?.elements().next()?;
        let kind = source.name.local.clone();
        let target = if !source.name.in_ns(&self.gml) {
            None
        } else if kind == "Point" {
            Some(("lod0Point", None))
        } else {
            multi_of(&kind).map(|multi| {
                let local = if multi == "MultiSurface" {
                    "lod0MultiSurface"
                } else {
                    "lod0MultiCurve"
                };
                (local, Some(multi))
            })
        };
        let Some((local, multi)) = target else {
            warnings.add(format!(
                "the Dm lod0Geometry of the {label} is a gml:{kind}, for which CityGML 3.0 \
                 has no LOD0 property, so the {label} got no LOD0"
            ));
            return None;
        };

        let geometry = match self.dm {
            UnfDm::Drop => {
                let holder = self.lod0_geometry_mut(class)?;
                let i = holder
                    .children
                    .iter()
                    .position(|n| matches!(n, Node::Element(_)))?;
                match holder.children.remove(i) {
                    Node::Element(geometry) => geometry,
                    _ => unreachable!(),
                }
            }
            UnfDm::Keep => {
                let mut geometry = source.clone();
                strip_ids(&mut geometry, &self.gml);
                geometry
            }
        };
        let mut property = Element::new(Name::qualified(&self.core, local));
        property.push(geometry);
        if let Some(multi) = multi {
            self.wrap_single(&mut property, multi);
        }
        warnings.add(format!(
            "the Dm lod0Geometry of the {label} became core:{local}. CityGML 3.0 gives an \
             underground utility a LOD0 of its own"
        ));
        Some(property)
    }

    /// True when a Dm class marks its figure as an attached figure or a
    /// supplementary symbol rather than the feature itself.
    fn is_annex(&self, class: &Element) -> bool {
        class.elements().any(|e| {
            self.is_iur(&e.name)
                && matches!(
                    e.name.local.as_str(),
                    "isInstallation" | "isSupplementarySymbol"
                )
                && matches!(e.text().trim(), "1" | "true")
        })
    }

    fn lod0_geometry<'e>(&self, class: &'e Element) -> Option<&'e Element> {
        class
            .elements()
            .find(|e| e.name.local == "lod0Geometry" && self.is_iur(&e.name))
            .filter(|e| e.has_element_children())
    }

    fn lod0_geometry_mut<'e>(&self, class: &'e mut Element) -> Option<&'e mut Element> {
        class
            .elements_mut()
            .find(|e| e.name.local == "lod0Geometry" && self.is_iur(&e.name))
            .filter(|e| e.has_element_children())
    }

    /// Reports each `dmCode` under `property` that the published code list
    /// does not define.
    fn check_dm_codes(&self, property: &Element, warnings: &mut Warnings) {
        if self.dm_codes.is_empty() {
            return;
        }
        for class in property.elements() {
            for code in class
                .elements()
                .filter(|e| e.name.local == "dmCode" && self.is_iur(&e.name))
            {
                let value = code.text().trim().to_owned();
                if !self.dm_codes.contains(&value) {
                    warnings.add(format!(
                        "a kept Dm attribute has dmCode {value}, which the published \
                         Common_dmCode.xml does not define"
                    ));
                }
            }
        }
    }

    /// Writes the LOD3 quality descriptors under `el` as LOD2, dropping one
    /// whose LOD2 sibling already holds the same value, and drops the LOD4
    /// ones.
    fn renumber_quality(&self, el: &mut Element, label: &str, warnings: &mut Warnings) {
        let children = std::mem::take(&mut el.children);
        let mut out: Vec<Node> = Vec::with_capacity(children.len());
        let mut renamed: Vec<Element> = Vec::new();
        for child in children {
            let Node::Element(mut child) = child else {
                out.push(child);
                continue;
            };
            let Some((stem, lod)) = self.quality_descriptor(&child) else {
                if !child.name.in_ns(&self.gml) {
                    self.renumber_quality(&mut child, label, warnings);
                }
                out.push(Node::Element(child));
                continue;
            };
            let from = self.display.display_name(&child.name);
            match lod {
                '4' => warnings.add(format!(
                    "{from} on the {label} was dropped with the LOD4 geometry it describes"
                )),
                '3' => {
                    child.name = Name::qualified(
                        child.name.ns.clone().unwrap_or_default(),
                        format!("{stem}Lod2"),
                    );
                    warnings.add(format!(
                        "{from} became {} with the geometry it describes",
                        self.display.display_name(&child.name)
                    ));
                    renamed.push(child);
                }
                _ => out.push(Node::Element(child)),
            }
        }
        for child in renamed {
            let value = child.text();
            let duplicate = out.iter().any(|n| {
                matches!(n, Node::Element(e) if e.name.local == child.name.local
                    && e.text().trim() == value.trim())
            });
            if !duplicate {
                out.push(Node::Element(child));
            }
        }
        el.children = out;
    }

    /// The stem and LOD digit of an i-UR quality descriptor, such as
    /// `("geometrySrcDesc", '3')` for `urc:geometrySrcDescLod3`.
    fn quality_descriptor(&self, el: &Element) -> Option<(&'static str, char)> {
        if !self.is_iur(&el.name) {
            return None;
        }
        let stem = QUALITY_STEMS
            .iter()
            .copied()
            .find(|stem| el.name.local.starts_with(stem))?;
        let rest = el.name.local[stem.len()..].strip_prefix("Lod")?;
        let mut chars = rest.chars();
        let digit = chars.next().filter(char::is_ascii_digit)?;
        chars.as_str().is_empty().then_some((stem, digit))
    }

    fn is_iur(&self, name: &Name) -> bool {
        self.iur.iter().any(|ns| name.in_ns(ns))
    }
}

/// The GML aggregate a geometry named `kind` belongs in, or `None` when it
/// is neither a surface nor a curve.
fn multi_of(kind: &str) -> Option<&'static str> {
    if kind == "MultiSurface" || SURFACES.contains(&kind) {
        Some("MultiSurface")
    } else if kind == "MultiCurve" || CURVES.contains(&kind) {
        Some("MultiCurve")
    } else {
        None
    }
}

/// Adds the `gml:id` of `el` and of every descendant to `out`.
fn collect_ids(el: &Element, gml: &str, out: &mut HashSet<String>) {
    if let Some(id) = el.attr(Some(gml), "id") {
        out.insert(id.to_owned());
    }
    for child in el.elements() {
        collect_ids(child, gml, out);
    }
}

/// Removes the `gml:id` of `el` and of every descendant.
fn strip_ids(el: &mut Element, gml: &str) {
    el.take_attr(Some(gml), "id");
    for child in el.elements_mut() {
        strip_ids(child, gml);
    }
}
