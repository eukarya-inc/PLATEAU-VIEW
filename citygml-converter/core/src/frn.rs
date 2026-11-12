//! Structural rewrites for city furniture that a rename table cannot express.
//!
//! Expects CityGML 3.0 and i-UR 4.0 names. Acts on each feature whose type
//! the profile's `[frn] lod1_surface` table lists.
//!
//! ```text
//! 2.0   <frn:CityFurniture>
//!         <frn:lod1Geometry><gml:MultiSurface>…</…></…>
//!
//! 3.0   <frn:CityFurniture>
//!         <frn:adeOfCityFurniture>
//!           <uro:CityFurnitureExtendedGeometry>
//!             <uro:lod1MultiSurface><gml:MultiSurface>…</…></…>
//!           </…>
//!         </…>
//!       </frn:CityFurniture>
//! ```
//!
//! A `lod1Geometry` holding a surface or a multi-surface moves into a new
//! instance of the class the table gives, held by the feature's
//! `adeOf<Type>` property, as that class's `lod1MultiSurface`. A single
//! surface is wrapped in a `gml:MultiSurface`. Any other `lod1Geometry` is
//! left to [`crate::construction`].

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::geometry::{multi_of, wrap_single};
use crate::profile::Rules;
use crate::report::Warnings;
use crate::schema::ChildOrder;
use crate::xml::{Element, Name, Node};

#[derive(Debug, Clone)]
pub struct FrnRewrite {
    /// Per feature type, the `adeOf<Type>` property and the class it holds.
    classes: HashMap<Name, (Name, Name)>,
    gml: String,
    display: Rules,
}

impl FrnRewrite {
    /// Fails when `schema` does not declare a listed type's `adeOf<Type>`
    /// property, or the class's `lod1MultiSurface`.
    pub fn new(rules: &Rules, schema: &ChildOrder) -> Result<Self> {
        let mut classes = HashMap::new();
        for (ty, class) in &rules.frn().lod1_surface {
            let hook = Name::qualified(
                ty.ns.clone().unwrap_or_default(),
                format!("adeOf{}", ty.local),
            );
            let surface = surface_of(class);
            if !schema.declares(ty, &hook) || !schema.declares(class, &surface) {
                return Err(Error::Profile(format!(
                    "[frn] lod1_surface: {} declares no {} holding a {} with a {}",
                    rules.display_name(ty),
                    rules.display_name(&hook),
                    rules.display_name(class),
                    rules.display_name(&surface)
                )));
            }
            classes.insert(ty.clone(), (hook, class.clone()));
        }
        Ok(FrnRewrite {
            classes,
            gml: rules.output_ns("gml")?.to_owned(),
            display: rules.clone(),
        })
    }

    /// Rewrites every listed feature under `member`, `member` included.
    pub fn apply(&self, member: &mut Element, warnings: &mut Warnings) {
        if self.classes.is_empty() || member.name.in_ns(&self.gml) {
            return;
        }
        if let Some((hook, class)) = self.classes.get(&member.name) {
            self.feature(member, hook, class, warnings);
            return;
        }
        for child in member.elements_mut() {
            self.apply(child, warnings);
        }
    }

    fn feature(&self, feature: &mut Element, hook: &Name, class: &Name, warnings: &mut Warnings) {
        let label = self.display.display_name(&feature.name);
        let ns = feature.name.ns.clone().unwrap_or_default();
        let lod1 = Name::qualified(ns, "lod1Geometry");
        let Some(at) = feature
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.name == lod1 && self.holds_surface(e)))
        else {
            return;
        };
        let Node::Element(mut property) = feature.children.remove(at) else {
            unreachable!()
        };
        let from = self.display.display_name(&property.name);
        wrap_single(&mut property, "MultiSurface", &self.gml);
        property.name = surface_of(class);
        warnings.add(format!(
            "{from} on the {label} held a surface and became {} in {}/{}. CityGML 3.0 has \
             no LOD1 surface property on a city furniture",
            self.display.display_name(&property.name),
            self.display.display_name(hook),
            self.display.display_name(class)
        ));
        let mut holder = Element::new(class.clone());
        holder.push(property);
        let mut ade = Element::new(hook.clone());
        ade.push(holder);
        feature.push(ade);
    }

    /// True when the geometry under `property` is a GML surface or
    /// multi-surface.
    fn holds_surface(&self, property: &Element) -> bool {
        property.elements().next().is_some_and(|g| {
            g.name.in_ns(&self.gml) && multi_of(&g.name.local) == Some("MultiSurface")
        })
    }
}

/// The `lod1MultiSurface` property of `class`, in its namespace.
fn surface_of(class: &Name) -> Name {
    Name::qualified(class.ns.clone().unwrap_or_default(), "lod1MultiSurface")
}
