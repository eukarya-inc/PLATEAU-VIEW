//! The child order each output element requires, read off the vendored target
//! schemas.
//!
//! A type's content is the concatenation of the `xs:sequence` of every type in
//! its `xs:extension` chain, base first. [`ChildOrder`] maps each global
//! element of CityGML 3.0 and i-UR 4.0 to that concatenation. GML elements get
//! no entry, and neither does a type whose content repeats a run of several
//! elements, since no single sequence describes it.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use quick_xml::events::Event;

use crate::error::{Error, Result};
use crate::xml::{Chunk, Element, Name, Reader, ns};

const XS: &str = "http://www.w3.org/2001/XMLSchema";

/// Each output element's children, ranked by the position the schema declares.
#[derive(Debug, Clone, Default)]
pub struct ChildOrder {
    by_element: HashMap<Name, Arc<HashMap<Name, usize>>>,
}

impl ChildOrder {
    /// Reads the schemas the converter targets, [`crate::OGC_SCHEMAS`] and
    /// [`crate::IUR_4_0_SCHEMAS`].
    pub fn target() -> Result<ChildOrder> {
        let docs: Vec<(&str, &str)> = crate::OGC_SCHEMAS
            .iter()
            .chain(crate::IUR_4_0_SCHEMAS)
            .copied()
            .collect();
        ChildOrder::read(&docs)
    }

    /// Reads `(path, text)` schema documents. `path` names a document in
    /// errors only. A base type or group none of them declares is an error.
    pub fn read(docs: &[(&str, &str)]) -> Result<ChildOrder> {
        let mut set = SchemaSet::default();
        for (path, text) in docs {
            set.add(path, text)?;
        }

        let mut by_type: HashMap<Name, Option<Arc<HashMap<Name, usize>>>> = HashMap::new();
        let mut by_element = HashMap::new();
        for (element, ty) in &set.elements {
            if !by_type.contains_key(ty) {
                let ranks = set
                    .sequence(ty)?
                    .filter(|names| !names.is_empty())
                    .map(|names| {
                        let mut ranks = HashMap::new();
                        for name in names {
                            let next = ranks.len();
                            ranks.entry(name).or_insert(next);
                        }
                        Arc::new(ranks)
                    });
                by_type.insert(ty.clone(), ranks);
            }
            if let Some(Some(ranks)) = by_type.get(ty) {
                by_element.insert(element.clone(), Arc::clone(ranks));
            }
        }
        Ok(ChildOrder { by_element })
    }

    /// The position of each child `element` may hold, or `None` when the
    /// schemas give it no single order.
    pub fn of(&self, element: &Name) -> Option<&HashMap<Name, usize>> {
        self.by_element.get(element).map(|ranks| ranks.as_ref())
    }
}

/// One particle of a content model, in document order.
#[derive(Debug, Clone)]
enum Particle {
    Element(Name),
    Group(Name),
}

/// A content model as declared, before its base or groups are resolved.
#[derive(Debug, Clone, Default)]
struct Model {
    /// The `xs:extension` base, whose content comes first.
    base: Option<Name>,
    particles: Vec<Particle>,
    /// A repeated `xs:sequence`, `xs:choice` or group reference holds more
    /// than one particle.
    repeats: bool,
}

#[derive(Debug, Default)]
struct SchemaSet {
    types: HashMap<Name, (String, Model)>,
    groups: HashMap<Name, (String, Model)>,
    /// Global element to its named type, for every document outside GML.
    elements: HashMap<Name, Name>,
}

impl SchemaSet {
    fn add(&mut self, path: &str, text: &str) -> Result<()> {
        let prefixes = prefix_bindings(path, text)?;
        let mut reader = Reader::new(path, text);
        let mut target = None;
        while let Some(chunk) = reader.next_chunk()? {
            match chunk {
                Chunk::RootStart(root) => {
                    target = root.attr(None, "targetNamespace").map(str::to_owned);
                }
                Chunk::Member(decl) => {
                    let Some(tns) = target.as_deref() else {
                        return Err(malformed(path, "the schema declares no targetNamespace"));
                    };
                    let doc = Doc {
                        path,
                        tns,
                        prefixes: &prefixes,
                    };
                    self.declare(&doc, &decl)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn declare(&mut self, doc: &Doc<'_>, decl: &Element) -> Result<()> {
        if decl.name.ns.as_deref() != Some(XS) {
            return Ok(());
        }
        let Some(name) = decl.attr(None, "name") else {
            return Ok(());
        };
        let name = Name::qualified(doc.tns, name);
        match decl.name.local.as_str() {
            "complexType" => {
                let model = doc.model(decl)?;
                self.types.insert(name, (doc.path.to_owned(), model));
            }
            "group" => {
                let model = doc.model(decl)?;
                self.groups.insert(name, (doc.path.to_owned(), model));
            }
            "element" if doc.tns != ns::GML_32 => {
                if let Some(ty) = decl.attr(None, "type") {
                    self.elements.insert(name, doc.resolve(ty)?);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// The children of `ty` in declared order, or `None` when it repeats a
    /// run of several elements. A type the set does not declare has none,
    /// which covers the XML Schema built-in types.
    fn sequence(&self, ty: &Name) -> Result<Option<Vec<Name>>> {
        let Some((path, model)) = self.types.get(ty) else {
            return Ok(Some(Vec::new()));
        };
        if model.repeats {
            return Ok(None);
        }
        let mut out = match &model.base {
            Some(base) if self.types.contains_key(base) => match self.sequence(base)? {
                Some(names) => names,
                None => return Ok(None),
            },
            Some(base) if base.ns.as_deref() == Some(XS) => Vec::new(),
            Some(base) => {
                return Err(malformed(
                    path,
                    format!("{} extends {base:?}, which no schema declares", ty.local),
                ));
            }
            None => Vec::new(),
        };
        if !self.expand(path, &model.particles, &mut out, &mut HashSet::new())? {
            return Ok(None);
        }
        Ok(Some(out))
    }

    /// Appends `particles` to `out` with every group reference expanded.
    /// Returns `false` when a referenced group repeats.
    fn expand(
        &self,
        path: &str,
        particles: &[Particle],
        out: &mut Vec<Name>,
        seen: &mut HashSet<Name>,
    ) -> Result<bool> {
        for particle in particles {
            match particle {
                Particle::Element(name) => out.push(name.clone()),
                Particle::Group(group) => {
                    let Some((_, model)) = self.groups.get(group) else {
                        return Err(malformed(
                            path,
                            format!("group {group:?} is referenced but no schema declares it"),
                        ));
                    };
                    if model.repeats || !seen.insert(group.clone()) {
                        return Ok(false);
                    }
                    if !self.expand(path, &model.particles, out, seen)? {
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }
}

/// One schema document's context for resolving the names it writes.
struct Doc<'a> {
    path: &'a str,
    tns: &'a str,
    prefixes: &'a HashMap<String, String>,
}

impl Doc<'_> {
    /// The content model of a named `xs:complexType` or `xs:group`.
    fn model(&self, decl: &Element) -> Result<Model> {
        let mut model = Model::default();
        self.collect(decl, &mut model)?;
        Ok(model)
    }

    fn collect(&self, node: &Element, model: &mut Model) -> Result<()> {
        for child in node.elements().filter(|c| c.name.ns.as_deref() == Some(XS)) {
            match child.name.local.as_str() {
                "element" => {
                    let name = match (child.attr(None, "name"), child.attr(None, "ref")) {
                        (Some(local), _) => Name::qualified(self.tns, local),
                        (None, Some(reference)) => self.resolve(reference)?,
                        (None, None) => {
                            return Err(malformed(self.path, "an xs:element with no name or ref"));
                        }
                    };
                    model.particles.push(Particle::Element(name));
                }
                "group" => {
                    if let Some(reference) = child.attr(None, "ref") {
                        if repeated(child) {
                            model.repeats = true;
                        }
                        model
                            .particles
                            .push(Particle::Group(self.resolve(reference)?));
                    } else {
                        self.collect(child, model)?;
                    }
                }
                "sequence" | "choice" | "all" => {
                    let before = model.particles.len();
                    self.collect(child, model)?;
                    if repeated(child) && model.particles.len() - before > 1 {
                        model.repeats = true;
                    }
                }
                "complexContent" => self.collect(child, model)?,
                "extension" => {
                    if let Some(base) = child.attr(None, "base") {
                        model.base = Some(self.resolve(base)?);
                    }
                    self.collect(child, model)?;
                }
                "restriction" => self.collect(child, model)?,
                _ => {}
            }
        }
        Ok(())
    }

    /// Expands a `prefix:local` attribute value by the document's bindings.
    fn resolve(&self, qname: &str) -> Result<Name> {
        let (prefix, local) = qname.split_once(':').unwrap_or(("", qname));
        let uri = self.prefixes.get(prefix).ok_or_else(|| {
            malformed(
                self.path,
                format!("`{qname}` uses a prefix the schema root does not bind"),
            )
        })?;
        Ok(Name::qualified(uri.clone(), local))
    }
}

fn repeated(particle: &Element) -> bool {
    particle
        .attr(None, "maxOccurs")
        .is_some_and(|max| max != "1" && max != "0")
}

/// The namespace bindings on the root element, with the default namespace
/// under the empty prefix.
fn prefix_bindings(path: &str, text: &str) -> Result<HashMap<String, String>> {
    let mut reader = quick_xml::Reader::from_str(text);
    loop {
        match reader.read_event() {
            Ok(Event::Start(start) | Event::Empty(start)) => {
                let mut out = HashMap::new();
                for attr in start.attributes().with_checks(false).flatten() {
                    let key: &str = attr.key.as_ref();
                    let prefix = match key.split_once(':') {
                        Some(("xmlns", prefix)) => prefix.to_owned(),
                        None if key == "xmlns" => String::new(),
                        _ => continue,
                    };
                    out.insert(prefix, attr.value.into_owned());
                }
                return Ok(out);
            }
            Ok(Event::Eof) => return Err(malformed(path, "the document has no root element")),
            Ok(_) => {}
            Err(e) => return Err(malformed(path, e.to_string())),
        }
    }
}

fn malformed(path: &str, message: impl Into<String>) -> Error {
    Error::malformed(path, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRAN: &str = "http://www.opengis.net/citygml/transportation/3.0";

    fn order() -> ChildOrder {
        ChildOrder::target().expect("the vendored schemas must read")
    }

    fn position(order: &ChildOrder, ns: &str, element: &str, child: (&str, &str)) -> usize {
        order
            .of(&Name::qualified(ns, element))
            .unwrap_or_else(|| panic!("{element} has an order"))[&Name::qualified(child.0, child.1)]
    }

    /// The chain crosses from the thematic module into core and GML, so every
    /// level's schema has to be vendored for the base content to come first.
    #[test]
    fn an_inherited_property_precedes_the_type_s_own() {
        let order = order();
        let road = |child| position(&order, TRAN, "Road", child);
        assert!(road((ns::GML_32, "name")) < road((ns::CITYGML_3, "creationDate")));
        assert!(road((ns::CITYGML_3, "creationDate")) < road((ns::CITYGML_3, "boundary")));
        assert!(road((ns::CITYGML_3, "boundary")) < road((TRAN, "trafficSpace")));
        assert!(road((TRAN, "trafficSpace")) < road((TRAN, "class")));
    }

    /// Every i-UR 4.0 class resolves its chain, which reaches into CityGML
    /// for the classes that substitute into an ADE hook.
    #[test]
    fn i_ur_classes_are_ordered() {
        let order = order();
        let urc = "https://www.geospatial.jp/iur/urc/4.0";
        let lod_type = position(
            &order,
            urc,
            "ExteriorDataQualityAttribute",
            (urc, "lodType"),
        );
        let height_type = position(
            &order,
            urc,
            "ExteriorDataQualityAttribute",
            (urc, "lod1HeightType"),
        );
        assert!(lod_type < height_type);
    }

    /// GML geometry repeats point choices, so sorting it would scramble
    /// coordinates.
    #[test]
    fn gml_and_repeated_content_have_no_order() {
        let order = order();
        assert!(
            order
                .of(&Name::qualified(ns::GML_32, "LinearRing"))
                .is_none()
        );
        assert!(
            order
                .of(&Name::qualified(ns::CITYGML_3, "CityModel"))
                .is_none()
        );
    }
}
