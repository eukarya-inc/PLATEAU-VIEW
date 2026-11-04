//! Structural rewrites for other constructions (`con:OtherConstruction`)
//! that a rename table cannot express.
//!
//! Runs after [`crate::construction`] and before [`crate::iur`], so every
//! name here is CityGML 3.0 and i-UR 4.0, and the i-UR attributes still sit in
//! their 2.0 properties.
//!
//! ```text
//! 2.0   <uro:OtherConstruction>
//!         <uro:consStructureAttribute><uro:EmbankmentAttribute>…</…></…>
//!         <uro:constructionInstallation><uro:ConstructionInstallation>…</…></…>
//!
//! 3.0   <con:OtherConstruction>
//!         <uro:consStructureAttribute><uro:EmbankmentAttribute>
//!           …
//!           <uro:otherConstructionInstallation>
//!             <uro:OtherConstructionInstallation>…</…>
//!           </…>
//!         </…></…>
//! ```
//!
//! An installation goes into the construction's first structure attribute,
//! meaning an i-UR class whose 4.0 content model declares
//! `uro:otherConstructionInstallation`. A construction with none gains a
//! `uro:ConstructionStructureAttribute` in `con:adeOfOtherConstruction` to
//! hold them. Every move is reported.

use crate::error::Result;
use crate::iur::IurModules;
use crate::profile::Rules;
use crate::report::Warnings;
use crate::schema::ChildOrder;
use crate::xml::{Element, Name, Node};

#[derive(Debug, Clone)]
pub struct ConsRewrite {
    feature: Name,
    /// The 2.0 installation property, as the rename pass leaves it.
    installation: Name,
    /// The 4.0 property a structure attribute holds installations in.
    holder: Name,
    structure: Name,
    hook: Name,
    iur: IurModules,
    schema: ChildOrder,
    display: Rules,
}

impl ConsRewrite {
    /// `schema` holds the target schemas' content models.
    pub fn new(rules: &Rules, schema: &ChildOrder) -> Result<Self> {
        let con = rules.output_ns("con")?;
        let uro = rules.output_ns("uro")?;
        Ok(ConsRewrite {
            feature: Name::qualified(con, "OtherConstruction"),
            installation: Name::qualified(uro, "constructionInstallation"),
            holder: Name::qualified(uro, "otherConstructionInstallation"),
            structure: Name::qualified(uro, "ConstructionStructureAttribute"),
            hook: Name::qualified(con, "adeOfOtherConstruction"),
            iur: IurModules::new(rules),
            schema: schema.clone(),
            display: rules.clone(),
        })
    }

    /// Rewrites every other construction under `member`, `member` included.
    pub fn apply(&self, member: &mut Element, warnings: &mut Warnings) {
        if member.name == self.feature {
            self.feature(member, warnings);
            return;
        }
        for child in member.elements_mut() {
            self.apply(child, warnings);
        }
    }

    fn feature(&self, feature: &mut Element, warnings: &mut Warnings) {
        let children = std::mem::take(&mut feature.children);
        let mut out = Vec::with_capacity(children.len());
        let mut installations = Vec::new();
        for child in children {
            match child {
                Node::Element(child) if child.name == self.installation => {
                    installations.push(child);
                }
                other => out.push(other),
            }
        }
        feature.children = out;
        if installations.is_empty() {
            return;
        }

        let from = self.display.display_name(&self.installation);
        let to = self.display.display_name(&self.holder);
        let label = self.display.display_name(&feature.name);
        let target = match self.structure_attribute(feature) {
            Some(class) => {
                warnings.add(format!(
                    "{from} on the {label} became {to} in its {}: i-UR 4.0 holds an other \
                     construction's installations in its structure attribute",
                    self.display.display_name(&class.name)
                ));
                class
            }
            None => {
                let mut hook = Element::new(self.hook.clone());
                hook.push(Element::new(self.structure.clone()));
                feature.push(hook);
                warnings.add(format!(
                    "{from} on the {label} became {to} in a new {}: i-UR 4.0 holds an other \
                     construction's installations in its structure attribute, and the \
                     {label} had none",
                    self.display.display_name(&self.structure)
                ));
                feature
                    .elements_mut()
                    .last()
                    .and_then(|hook| hook.elements_mut().next())
                    .expect("the structure attribute just added")
            }
        };
        for mut installation in installations {
            installation.name = self.holder.clone();
            target.push(installation);
        }
    }

    /// The first structure attribute under `feature`'s own properties.
    fn structure_attribute<'a>(&self, feature: &'a mut Element) -> Option<&'a mut Element> {
        feature
            .elements_mut()
            .flat_map(|property| property.elements_mut())
            .find(|class| {
                self.iur.contains(&class.name) && self.schema.declares(&class.name, &self.holder)
            })
    }
}
