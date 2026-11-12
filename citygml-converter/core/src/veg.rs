//! Structural rewrites for vegetation objects that a rename table cannot
//! express.
//!
//! Expects CityGML 3.0 names. Acts on the `veg` geometry properties the
//! rename pass leaves in place.
//!
//! ```text
//! 2.0   <veg:PlantCover>
//!         <veg:lod1MultiSolid>
//!           <gml:MultiSolid><gml:solidMember><gml:Solid>…</…></…></…>
//!         </…>
//!
//! 3.0   <veg:PlantCover>
//!         <core:lod1Solid><gml:Solid>…</…></…>
//!       </veg:PlantCover>
//! ```
//!
//! A `veg:lodNMultiSolid` whose multi-solid holds one solid becomes
//! `core:lodNSolid` holding that solid, which takes the multi-solid's
//! `srsName` and `srsDimension` when it has none of its own, and the
//! `gml:id`s left under the multi-solid are added to the caller's set. A
//! multi-solid holding any other number of solids and a
//! `veg:lod1MultiSurface` have no CityGML 3.0 property and are removed. Every
//! removal is reported, and the `gml:id`s under a removed property are added
//! to the caller's set.

use crate::dropped::Dropped;
use crate::error::Result;
use crate::geometry::{SOLIDS, lod_of};
use crate::profile::Rules;
use crate::report::Warnings;
use crate::xml::{Element, Name, Node};

#[derive(Debug, Clone)]
pub struct VegRewrite {
    veg: String,
    core: String,
    gml: String,
    display: Rules,
}

impl VegRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        Ok(VegRewrite {
            veg: rules.output_ns("veg")?.to_owned(),
            core: rules.output_ns("core")?.to_owned(),
            gml: rules.output_ns("gml")?.to_owned(),
            display: rules.clone(),
        })
    }

    /// Rewrites the `veg` geometry properties under `member`.
    pub fn apply(&self, member: &mut Element, dropped: &mut Dropped, warnings: &mut Warnings) {
        if member.name.in_ns(&self.gml) {
            return;
        }
        if member.name.in_ns(&self.veg) {
            self.feature(member, dropped, warnings);
        }
        for child in member.elements_mut() {
            self.apply(child, dropped, warnings);
        }
    }

    /// Rewrites the `veg` geometry properties of `member` itself.
    fn feature(&self, member: &mut Element, dropped: &mut Dropped, warnings: &mut Warnings) {
        let label = self.display.display_name(&member.name);
        let children = std::mem::take(&mut member.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            match child {
                Node::Element(child) if self.is_property(&child, "MultiSolid") => {
                    if let Some(child) = self.multi_solid(child, &label, dropped, warnings) {
                        out.push(Node::Element(child));
                    }
                }
                Node::Element(child) if self.is_property(&child, "MultiSurface") => {
                    let lod = lod_of(&child.name.local).unwrap_or('?');
                    dropped.add(&child);
                    warnings.add(format!(
                        "{} on the {label} was dropped. CityGML 3.0 has no LOD{lod} surface \
                         property on a vegetation object",
                        self.display.display_name(&child.name)
                    ));
                }
                other => out.push(other),
            }
        }
        member.children = out;
    }

    /// True for a `veg:lodN<family>` property.
    fn is_property(&self, el: &Element, family: &str) -> bool {
        el.name.in_ns(&self.veg)
            && lod_of(&el.name.local).is_some()
            && el.name.local[4..] == *family
    }

    fn multi_solid(
        &self,
        mut property: Element,
        label: &str,
        dropped: &mut Dropped,
        warnings: &mut Warnings,
    ) -> Option<Element> {
        let from = self.display.display_name(&property.name);
        let lod = lod_of(&property.name.local).unwrap_or('?');
        let to = Name::qualified(&self.core, format!("lod{lod}Solid"));
        let Some(solid) = self.only_solid(&mut property, dropped) else {
            dropped.add(&property);
            warnings.add(format!(
                "{from} on the {label} was dropped. Its gml:MultiSolid does not hold exactly \
                 one solid, and CityGML 3.0 holds a single solid at LOD{lod}"
            ));
            return None;
        };
        warnings.add(format!(
            "{from} on the {label} became {}, holding the one solid of its gml:MultiSolid",
            self.display.display_name(&to)
        ));
        let mut out = Element::new(to);
        out.attrs = property.attrs;
        out.push(solid);
        Some(out)
    }

    /// Takes the solid out of the `gml:MultiSolid` under `property` when it
    /// holds exactly one, giving it the multi-solid's `srsName` and
    /// `srsDimension` where it has none, and adds the `gml:id`s left under
    /// the multi-solid to `dropped`.
    fn only_solid(&self, property: &mut Element, dropped: &mut Dropped) -> Option<Element> {
        let multi = property.elements_mut().next()?;
        if !self.holds_one_solid(multi) {
            return None;
        }
        let srs: Vec<(String, String)> = ["srsName", "srsDimension"]
            .into_iter()
            .filter_map(|a| multi.attr(None, a).map(|v| (a.to_owned(), v.to_owned())))
            .collect();
        let member = multi.elements_mut().find(|m| m.has_element_children())?;
        let at = member
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(_)))?;
        let Node::Element(mut solid) = member.children.remove(at) else {
            unreachable!()
        };
        dropped.add(multi);
        for (attr, value) in srs {
            if solid.attr(None, &attr).is_none() {
                solid.set_attr(Name::unqualified(&attr), value);
            }
        }
        Some(solid)
    }

    /// True when `multi` is a `gml:MultiSolid` whose member properties hold
    /// one GML solid between them.
    fn holds_one_solid(&self, multi: &Element) -> bool {
        if !multi.is(&self.gml, "MultiSolid") {
            return false;
        }
        let is_member = |m: &Element| {
            m.name.in_ns(&self.gml)
                && matches!(m.name.local.as_str(), "solidMember" | "solidMembers")
        };
        if !multi.elements().all(is_member) {
            return false;
        }
        let mut solids = multi.elements().flat_map(Element::elements);
        match (solids.next(), solids.next()) {
            (Some(only), None) => {
                only.name.in_ns(&self.gml) && SOLIDS.contains(&only.name.local.as_str())
            }
            _ => false,
        }
    }
}
