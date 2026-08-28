//! Just enough XML Schema reading to answer the questions the mapping asks.
//!
//! Not a schema processor: it reads element declarations and their
//! `substitutionGroup`, which is all the i-UR mapping is derived from.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use quick_xml::events::Event;

/// One `xs:element` declaration.
#[derive(Debug, Clone)]
pub struct Decl {
    /// Declared at the top level, so it can appear as an ADE property's content.
    pub global: bool,
    pub substitution_group: Option<String>,
    /// The named `xs:complexType` this declaration sits inside, if any.
    ///
    /// A nested name means nothing on its own -- `lod1MultiSurface` is declared
    /// by several modules -- so what decides where it moves is whether the class
    /// holding it moved.
    pub owner: Option<String>,
}

/// The element declarations of one schema document, by local name.
#[derive(Debug, Clone, Default)]
pub struct Schema {
    pub version: String,
    pub decls: BTreeMap<String, Decl>,
}

impl Schema {
    pub fn read(path: &Path) -> Result<Schema> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Ok(Schema::parse(&text))
    }

    pub fn parse(text: &str) -> Schema {
        let mut reader = quick_xml::Reader::from_str(text);
        let mut schema = Schema::default();
        // Depth 1 is a direct child of xs:schema, i.e. a global declaration.
        let mut depth = 0usize;
        // (depth, name) of the named complexType currently open, if any.
        let mut owner: Option<(usize, String)> = None;

        while let Ok(event) = reader.read_event() {
            let (start, empty) = match &event {
                Event::Start(s) => (Some(s), false),
                Event::Empty(s) => (Some(s), true),
                Event::End(_) => {
                    depth = depth.saturating_sub(1);
                    if owner.as_ref().is_some_and(|(d, _)| *d >= depth) {
                        owner = None;
                    }
                    continue;
                }
                Event::Eof => break,
                _ => continue,
            };
            let Some(start) = start else { continue };
            let qname = start.name();
            let local = local_name(qname.as_ref());

            if local == "schema" {
                schema.version = attr(start, "version").unwrap_or_default();
            } else if local == "complexType" && !empty {
                if let Some(name) = attr(start, "name") {
                    owner = Some((depth, name));
                }
            } else if local == "element"
                && let Some(name) = attr(start, "name")
            {
                schema.decls.entry(name).or_insert(Decl {
                    global: depth == 1,
                    substitution_group: attr(start, "substitutionGroup"),
                    owner: owner.as_ref().map(|(_, n)| n.clone()),
                });
            }
            if !empty {
                depth += 1;
            }
        }
        schema
    }

    /// Every global declaration, in document order by name.
    pub fn globals(&self) -> impl Iterator<Item = (&String, &Decl)> {
        self.decls.iter().filter(|(_, d)| d.global)
    }

    pub fn has(&self, name: &str) -> bool {
        self.decls.contains_key(name)
    }
}

fn local_name(qname: &str) -> &str {
    match qname.rsplit_once(':') {
        Some((_, local)) => local,
        None => qname,
    }
}

fn attr(start: &quick_xml::events::BytesStart<'_>, wanted: &str) -> Option<String> {
    start
        .attributes()
        .with_checks(false)
        .flatten()
        .find(|a| local_name(a.key.as_ref()) == wanted)
        .map(|a| a.value.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"
      <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" version="4.0.0">
        <xs:element name="BuildingIDAttribute" substitutionGroup="bldg:ADEOfAbstractBuilding" type="x"/>
        <xs:complexType name="T">
          <xs:sequence><xs:element name="buildingID" type="xs:string"/></xs:sequence>
        </xs:complexType>
      </xs:schema>"#;

    #[test]
    fn reads_the_schema_version() {
        assert_eq!(Schema::parse(SRC).version, "4.0.0");
    }

    #[test]
    fn tells_global_declarations_from_nested_ones() {
        let s = Schema::parse(SRC);
        assert!(
            s.decls["BuildingIDAttribute"].global,
            "a direct child of xs:schema"
        );
        assert!(
            !s.decls["buildingID"].global,
            "declared inside a complexType"
        );
    }

    #[test]
    fn records_the_class_a_nested_declaration_belongs_to() {
        let s = Schema::parse(SRC);
        assert_eq!(s.decls["buildingID"].owner.as_deref(), Some("T"));
        assert_eq!(
            s.decls["BuildingIDAttribute"].owner, None,
            "a global has no owner"
        );
    }

    #[test]
    fn records_the_substitution_group() {
        let s = Schema::parse(SRC);
        assert_eq!(
            s.decls["BuildingIDAttribute"].substitution_group.as_deref(),
            Some("bldg:ADEOfAbstractBuilding")
        );
        assert_eq!(s.decls["buildingID"].substitution_group, None);
    }
}
