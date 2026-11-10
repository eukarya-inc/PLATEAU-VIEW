//! Structural rewrites for water bodies that a rename table cannot express.
//!
//! Runs after [`crate::lodmap`] and before [`crate::iur`], so every name here
//! is CityGML 3.0 and i-UR 4.0. Acts on each feature whose type the profile's
//! `[wtr] lod1_surface` table lists.
//!
//! ```text
//! 2.0   <wtr:WaterBody>
//!         <wtr:lod1MultiSurface>…</…>
//!
//! 3.0   <wtr:WaterBody>
//!         <core:boundary>
//!           <wtr:WaterSurface gml:id="…">
//!             <core:lod1MultiSurface>…</…>
//!           </…>
//!         </…>
//! ```
//!
//! The feature's `core:lod1MultiSurface` moves into one new boundary surface
//! of the type the table gives, with a minted `gml:id`. A
//! `core:lodNMultiSurface` of a boundary surface holding a single
//! surface, which a 2.0 `lodNSurface` becomes in the rename pass, is wrapped
//! in a `gml:MultiSurface`.

use std::collections::HashMap;

use crate::error::Result;
use crate::geometry::{lod_of, wrap_single};
use crate::profile::Rules;
use crate::transform::IdGen;
use crate::xml::{Element, Name, Node};

#[derive(Debug, Clone)]
pub struct WtrRewrite {
    surfaces: HashMap<Name, Name>,
    boundary: Name,
    lod1: Name,
    gml: String,
    core: String,
}

impl WtrRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        let core = rules.output_ns("core")?.to_owned();
        Ok(WtrRewrite {
            surfaces: rules.wtr().lod1_surface.clone(),
            boundary: Name::qualified(&core, "boundary"),
            lod1: Name::qualified(&core, "lod1MultiSurface"),
            gml: rules.output_ns("gml")?.to_owned(),
            core,
        })
    }

    /// Rewrites every listed feature under `member`, `member` included.
    /// `ids` mints the new boundary surfaces' `gml:id`.
    pub fn apply(&self, member: &mut Element, ids: &mut IdGen) {
        if self.surfaces.is_empty() {
            return;
        }
        if let Some(surface) = self.surfaces.get(&member.name) {
            self.feature(member, surface, ids);
            return;
        }
        for child in member.elements_mut() {
            self.apply(child, ids);
        }
    }

    fn feature(&self, feature: &mut Element, surface: &Name, ids: &mut IdGen) {
        for boundary in feature.elements_mut().filter(|e| e.name == self.boundary) {
            for part in boundary.elements_mut() {
                for property in part.elements_mut() {
                    if property.name.in_ns(&self.core)
                        && lod_of(&property.name.local).is_some()
                        && property.name.local.ends_with("MultiSurface")
                    {
                        wrap_single(property, "MultiSurface", &self.gml);
                    }
                }
            }
        }

        let (lod1, rest): (Vec<Node>, Vec<Node>) = std::mem::take(&mut feature.children)
            .into_iter()
            .partition(|n| matches!(n, Node::Element(e) if e.name == self.lod1));
        feature.children = rest;
        if lod1.is_empty() {
            return;
        }
        let mut part = Element::new(surface.clone());
        part.set_attr(Name::qualified(&self.gml, "id"), ids.mint());
        part.children = lod1;
        let mut boundary = Element::new(self.boundary.clone());
        boundary.push(part);
        feature.push(boundary);
    }
}
