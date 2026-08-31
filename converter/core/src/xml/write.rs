use std::io;

use quick_xml::escape::{escape, partial_escape};

use super::node::{Element, Name, Node};
use crate::error::Result;

/// Maps namespace URIs to the prefixes the output document declares for them.
///
/// The writer never invents a prefix: an element whose namespace is missing from
/// the map is a bug in the conversion profile, and is reported rather than
/// silently emitted unqualified.
#[derive(Clone, Debug, Default)]
pub struct PrefixMap {
    /// Declaration order is preserved so output stays byte-stable.
    entries: Vec<(String, String)>,
}

impl PrefixMap {
    pub fn new() -> Self {
        PrefixMap::default()
    }

    pub fn insert(&mut self, prefix: impl Into<String>, uri: impl Into<String>) {
        let (prefix, uri) = (prefix.into(), uri.into());
        if let Some(e) = self.entries.iter_mut().find(|(_, u)| *u == uri) {
            e.0 = prefix;
        } else {
            self.entries.push((prefix, uri));
        }
    }

    pub fn prefix_of(&self, uri: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(_, u)| u == uri)
            .map(|(p, _)| p.as_str())
    }

    pub fn declarations(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(p, u)| (p.as_str(), u.as_str()))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Indent {
    None,
    Spaces(usize),
    Tab,
}

impl Indent {
    fn write(&self, out: &mut impl io::Write, depth: usize) -> io::Result<()> {
        match self {
            Indent::None => Ok(()),
            Indent::Spaces(n) => {
                out.write_all(b"\n")?;
                for _ in 0..depth * n {
                    out.write_all(b" ")?;
                }
                Ok(())
            }
            Indent::Tab => {
                out.write_all(b"\n")?;
                for _ in 0..depth {
                    out.write_all(b"\t")?;
                }
                Ok(())
            }
        }
    }

    fn enabled(&self) -> bool {
        !matches!(self, Indent::None)
    }
}

/// Serialises [`Element`] trees, assigning prefixes from a [`PrefixMap`].
pub struct Writer<W: io::Write> {
    out: W,
    prefixes: PrefixMap,
    indent: Indent,
    /// Namespaces encountered that the prefix map does not cover.
    missing: Vec<String>,
}

impl<W: io::Write> Writer<W> {
    pub fn new(out: W, prefixes: PrefixMap, indent: Indent) -> Self {
        Writer {
            out,
            prefixes,
            indent,
            missing: Vec::new(),
        }
    }

    pub fn write_declaration(&mut self, decl: Option<&str>) -> Result<()> {
        match decl {
            Some(d) => writeln!(self.out, "<?{d}?>")?,
            None => writeln!(self.out, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?,
        }
        Ok(())
    }

    pub fn write_comment(&mut self, text: &str) -> Result<()> {
        writeln!(self.out, "<!--{text}-->")?;
        Ok(())
    }

    /// Writes a comment indented to `depth`, for comments inside the root.
    pub fn write_comment_at(&mut self, text: &str, depth: usize) -> Result<()> {
        self.indent.write(&mut self.out, depth)?;
        write!(self.out, "<!--{text}-->")?;
        Ok(())
    }

    /// Writes the root element's start tag together with every namespace
    /// declaration in the prefix map, plus an optional `xsi:schemaLocation`.
    pub fn write_root_start(
        &mut self,
        root: &Element,
        schema_location: Option<&str>,
    ) -> Result<()> {
        let name = self.qname(&root.name);
        write!(self.out, "<{name}")?;

        for (prefix, uri) in self.prefixes.declarations() {
            let uri = escape(uri);
            if prefix.is_empty() {
                write!(self.out, "\n\txmlns=\"{uri}\"")?;
            } else {
                write!(self.out, "\n\txmlns:{prefix}=\"{uri}\"")?;
            }
        }

        // Skip any inherited schemaLocation; the caller supplies the 3.0 one.
        let xsi = super::ns::XSI;
        for attr in &root.attrs {
            if attr.name.is(xsi, "schemaLocation") {
                continue;
            }
            let attr_name = self.qname(&attr.name);
            write!(self.out, " {attr_name}=\"{}\"", escape(&attr.value))?;
        }
        if let Some(loc) = schema_location {
            let xsi_prefix = self.prefixes.prefix_of(xsi).unwrap_or("xsi").to_owned();
            write!(
                self.out,
                "\n\t{xsi_prefix}:schemaLocation=\"{}\"",
                escape(loc)
            )?;
        }
        write!(self.out, ">")?;
        Ok(())
    }

    pub fn write_root_end(&mut self, root: &Element) -> Result<()> {
        let name = self.qname(&root.name);
        if self.indent.enabled() {
            self.out.write_all(b"\n")?;
        }
        writeln!(self.out, "</{name}>")?;
        Ok(())
    }

    /// Writes one element (and its subtree) at the given nesting depth.
    pub fn write_element(&mut self, el: &Element, depth: usize) -> Result<()> {
        self.indent.write(&mut self.out, depth)?;
        self.write_element_inner(el, depth)
    }

    fn write_element_inner(&mut self, el: &Element, depth: usize) -> Result<()> {
        let name = self.qname(&el.name);
        write!(self.out, "<{name}")?;
        for attr in &el.attrs {
            let attr_name = self.qname(&attr.name);
            write!(self.out, " {attr_name}=\"{}\"", escape(&attr.value))?;
        }

        if el.children.is_empty() {
            write!(self.out, "/>")?;
            return Ok(());
        }
        write!(self.out, ">")?;

        // An element that mixes text with elements must be written verbatim;
        // re-indenting it would change its value.
        let breakable = el.has_element_children();
        for child in &el.children {
            match child {
                Node::Element(child) => {
                    if breakable {
                        self.indent.write(&mut self.out, depth + 1)?;
                    }
                    self.write_element_inner(child, depth + 1)?;
                }
                Node::Text(t) => write!(self.out, "{}", partial_escape(t))?,
                Node::CData(t) => write!(self.out, "<![CDATA[{t}]]>")?,
                Node::Comment(t) => {
                    if breakable {
                        self.indent.write(&mut self.out, depth + 1)?;
                    }
                    write!(self.out, "<!--{t}-->")?;
                }
            }
        }

        if breakable {
            self.indent.write(&mut self.out, depth)?;
        }
        write!(self.out, "</{name}>")?;
        Ok(())
    }

    /// Namespaces that were written without a declaration. Non-empty means the
    /// profile is incomplete for this dataset.
    pub fn missing_namespaces(&self) -> &[String] {
        &self.missing
    }

    pub fn flush(&mut self) -> Result<()> {
        self.out.flush()?;
        Ok(())
    }

    pub fn into_inner(self) -> W {
        self.out
    }

    fn qname(&mut self, name: &Name) -> String {
        let Some(uri) = name.ns.as_deref() else {
            return name.local.clone();
        };
        match self.prefixes.prefix_of(uri).map(str::to_owned) {
            // The default namespace: no prefix to write.
            Some(p) if p.is_empty() => name.local.clone(),
            Some(p) => format!("{p}:{}", name.local),
            None => {
                let uri = uri.to_owned();
                if !self.missing.contains(&uri) {
                    self.missing.push(uri);
                }
                name.local.clone()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml::node::Attribute;

    fn prefixes() -> PrefixMap {
        let mut p = PrefixMap::new();
        p.insert("core", "urn:core");
        p.insert("gml", "urn:gml");
        p
    }

    fn render(el: &Element, indent: Indent) -> String {
        let mut w = Writer::new(Vec::new(), prefixes(), indent);
        w.write_element(el, 0).unwrap();
        String::from_utf8(w.into_inner()).unwrap()
    }

    #[test]
    fn escapes_text_and_attributes() {
        let mut el = Element::with_text(Name::qualified("urn:core", "v"), "a < b & c");
        el.attrs
            .push(Attribute::new(Name::unqualified("q"), "\"x\""));
        assert_eq!(
            render(&el, Indent::None),
            "<core:v q=\"&quot;x&quot;\">a &lt; b &amp; c</core:v>"
        );
    }

    #[test]
    fn undeclared_namespace_is_reported() {
        let el = Element::new(Name::qualified("urn:nope", "v"));
        let mut w = Writer::new(Vec::new(), prefixes(), Indent::None);
        w.write_element(&el, 0).unwrap();
        assert_eq!(w.missing_namespaces(), ["urn:nope"]);
    }
}
