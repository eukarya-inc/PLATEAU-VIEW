//! Removals for urban function features that a rename table cannot express.
//!
//! Runs right after the rename pass, so every name here is CityGML 3.0 and
//! i-UR 4.0. Acts on each urban function feature a top-level member holds,
//! that is each child of the member in the `urf` namespace.
//!
//! The geometry properties the profile's `[urf] removed` list names are taken
//! off the feature. A feature left with no `lod*` property is dropped, and the
//! member holding it with it. Each removal is reported with the feature's
//! `gml:id`, and what is removed, the whole feature when it is dropped, is
//! recorded with [`Dropped::add`].

use crate::dropped::Dropped;
use crate::error::Result;
use crate::profile::Rules;
use crate::report::Warnings;
use crate::xml::{Element, Name, Node};

#[derive(Debug, Clone)]
pub struct UrfRewrite {
    removed: Vec<Name>,
    urf: String,
    gml: String,
    display: Vec<String>,
}

impl UrfRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        let removed = rules.urf().removed.clone();
        let display = removed.iter().map(|n| rules.display_name(n)).collect();
        Ok(UrfRewrite {
            removed,
            urf: rules.output_ns("urf")?.to_owned(),
            gml: rules.output_ns("gml")?.to_owned(),
            display,
        })
    }

    /// Removes the listed properties from every urban function feature
    /// `member` holds. Returns true when a feature was left without geometry
    /// and `member` is to be dropped.
    pub fn apply(
        &self,
        member: &mut Element,
        dropped: &mut Dropped,
        warnings: &mut Warnings,
    ) -> bool {
        if self.removed.is_empty() {
            return false;
        }
        let mut drop = false;
        for feature in member.elements_mut() {
            if feature.name.in_ns(&self.urf) {
                drop |= self.feature(feature, dropped, warnings);
            }
        }
        drop
    }

    fn feature(
        &self,
        feature: &mut Element,
        dropped: &mut Dropped,
        warnings: &mut Warnings,
    ) -> bool {
        let mut removed = Vec::new();
        feature.children.retain(|node| match node {
            Node::Element(e) => match self.removed.iter().position(|n| *n == e.name) {
                Some(i) => {
                    dropped.add(e);
                    removed.push(i);
                    false
                }
                None => true,
            },
            _ => true,
        });
        if removed.is_empty() {
            return false;
        }
        let id = feature
            .attr(Some(&self.gml), "id")
            .unwrap_or("a feature without gml:id")
            .to_owned();
        let left = feature.elements().any(|e| e.name.local.starts_with("lod"));
        for i in removed {
            let property = &self.display[i];
            if left {
                warnings.add(format!(
                    "{property} was removed from {id}, since an urban function area takes \
                     no such geometry"
                ));
            } else {
                warnings.add(format!(
                    "{id} was dropped, since its only geometry was {property} and an urban \
                     function area takes no such geometry"
                ));
            }
        }
        if !left {
            dropped.add(feature);
        }
        !left
    }
}
