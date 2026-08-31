use std::fs::File;
use std::path::{Path, PathBuf};

use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

use super::node::{Attribute, Element, Name, Node};
use crate::error::{Error, Result};

/// Reads a CityGML file into a `String`, honouring a UTF-8 BOM and falling back
/// to Shift_JIS for the occasional legacy file.
/// The namespace URIs declared on a document's root element.
///
/// [`Reader`] resolves prefixes away and discards `xmlns` attributes, which is
/// right for conversion but loses exactly what version detection needs: a
/// document declares the i-UR namespaces it uses, and those carry the version.
/// Parsing stops at the root start tag, so this reads the head of the file only.
pub fn root_namespaces(source: &str) -> Vec<String> {
    let mut reader = quick_xml::Reader::from_str(source);
    let mut declared = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(start)) | Ok(Event::Empty(start)) => {
                for attr in start.attributes().with_checks(false).flatten() {
                    if attr.key.as_namespace_binding().is_some() {
                        let uri = attr.value.to_string();
                        if !declared.contains(&uri) {
                            declared.push(uri);
                        }
                    }
                }
                return declared;
            }
            Ok(Event::Eof) | Err(_) => return declared,
            _ => {}
        }
    }
}

/// How much of a file [`read_root_namespaces`] looks at.
///
/// The largest root start tag across a sample of PLATEAU packages is under
/// 3 KiB; 64 KiB leaves room for one far wider than anything observed.
const HEAD_BYTES: usize = 64 * 1024;

/// The namespaces declared on the root element of the document at `path`,
/// reading only the head of the file.
///
/// PLATEAU `.gml` files reach the better part of a gigabyte, so the whole point
/// is not to load one to read its first tag. Namespace URIs are ASCII, so a
/// truncated read is decoded lossily rather than sniffed for Shift_JIS the way
/// [`read_to_string`] does: a cut multi-byte character would otherwise flip the
/// whole buffer to the wrong encoding.
pub fn read_root_namespaces(path: &Path) -> Result<Vec<String>> {
    use std::io::Read;

    let file = File::open(path).map_err(|e| Error::io(path, e))?;
    let mut head = Vec::new();
    file.take(HEAD_BYTES as u64)
        .read_to_end(&mut head)
        .map_err(|e| Error::io(path, e))?;
    let head = head.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&head);
    Ok(root_namespaces(&String::from_utf8_lossy(head)))
}

pub fn read_to_string(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
    Ok(decode(&bytes))
}

fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned(),
    }
}

/// One unit of document structure handed out by [`Reader`].
///
/// The root element arrives on its own ([`Chunk::RootStart`]) so that the caller
/// can rewrite and emit it before any feature is read; everything directly
/// inside it arrives one subtree at a time. Peak memory is therefore one feature,
/// not one file.
#[derive(Debug)]
pub enum Chunk {
    /// The XML declaration. Its contents are deliberately dropped: the output is
    /// always UTF-8, so repeating the input's `encoding` would be a lie.
    Decl,
    /// A comment sitting outside the root element.
    Prologue(Node),
    /// The root element's start tag; `children` is always empty.
    RootStart(Element),
    /// A complete direct child of the root element.
    Member(Element),
    /// Non-element content directly inside the root element.
    RootContent(Node),
    /// The root element's end tag.
    RootEnd,
}

pub struct Reader<'i> {
    inner: NsReader<&'i [u8]>,
    path: PathBuf,
    in_root: bool,
    /// A self-closing root was reported; its `RootEnd` is owed on the next call.
    pending_root_end: bool,
    finished: bool,
}

impl<'i> Reader<'i> {
    pub fn new(path: impl Into<PathBuf>, source: &'i str) -> Self {
        let mut inner = NsReader::from_str(source);
        inner.config_mut().trim_text(false);
        Reader {
            inner,
            path: path.into(),
            in_root: false,
            pending_root_end: false,
            finished: false,
        }
    }

    /// Returns the next chunk, or `None` once the document is exhausted.
    pub fn next_chunk(&mut self) -> Result<Option<Chunk>> {
        if self.pending_root_end {
            self.pending_root_end = false;
            self.finished = true;
            return Ok(Some(Chunk::RootEnd));
        }
        if self.finished {
            return Ok(None);
        }
        loop {
            let event = self
                .inner
                .read_event()
                .map_err(|e| Error::xml(&self.path, e))?;
            match event {
                Event::Decl(_) => return Ok(Some(Chunk::Decl)),
                Event::Comment(c) => {
                    let node = Node::Comment(c.into_inner().into_owned());
                    return Ok(Some(if self.in_root {
                        Chunk::RootContent(node)
                    } else {
                        Chunk::Prologue(node)
                    }));
                }
                Event::Text(t) => {
                    // Inter-element whitespace carries no information; the writer
                    // re-indents. Anything else at this level is a document error
                    // we simply pass through as content.
                    let text = t.xml10_content();
                    if text.trim().is_empty() {
                        continue;
                    }
                    if self.in_root {
                        return Ok(Some(Chunk::RootContent(Node::Text(text.into_owned()))));
                    }
                    continue;
                }
                Event::Start(start) => {
                    if self.in_root {
                        let el = self.read_subtree(&start)?;
                        return Ok(Some(Chunk::Member(el)));
                    }
                    self.in_root = true;
                    return Ok(Some(Chunk::RootStart(self.element(&start)?)));
                }
                Event::Empty(start) => {
                    let el = self.element(&start)?;
                    if self.in_root {
                        return Ok(Some(Chunk::Member(el)));
                    }
                    // A self-closing root: report the start now and its end on
                    // the next call, so the caller sees the same chunk sequence
                    // an empty <root></root> produces.
                    self.pending_root_end = true;
                    return Ok(Some(Chunk::RootStart(el)));
                }
                Event::End(_) => {
                    // The only End we can see here is the root's own.
                    self.finished = true;
                    return Ok(Some(Chunk::RootEnd));
                }
                Event::CData(c) if self.in_root => {
                    return Ok(Some(Chunk::RootContent(Node::CData(
                        c.into_inner().into_owned(),
                    ))));
                }
                Event::Eof => {
                    self.finished = true;
                    return Ok(None);
                }
                _ => continue,
            }
        }
    }

    /// Materialises the subtree opened by `start`, whose `Start` event has already
    /// been consumed.
    fn read_subtree(&mut self, start: &BytesStart<'i>) -> Result<Element> {
        let mut stack = vec![self.element(start)?];
        loop {
            let event = self
                .inner
                .read_event()
                .map_err(|e| Error::xml(&self.path, e))?;
            match event {
                Event::Start(e) => stack.push(self.element(&e)?),
                Event::Empty(e) => {
                    let el = self.element(&e)?;
                    // `stack` is never empty: it starts with one element and only
                    // an `End` pops it, at which point we return.
                    stack.last_mut().expect("non-empty stack").push(el);
                }
                Event::End(_) => {
                    let mut el = stack.pop().expect("non-empty stack");
                    tidy(&mut el);
                    match stack.last_mut() {
                        Some(parent) => parent.push(el),
                        None => return Ok(el),
                    }
                }
                Event::Text(t) => {
                    let text = t.xml10_content();
                    if !text.is_empty() {
                        stack
                            .last_mut()
                            .expect("non-empty stack")
                            .children
                            .push(Node::Text(text.into_owned()));
                    }
                }
                Event::CData(c) => stack
                    .last_mut()
                    .expect("non-empty stack")
                    .children
                    .push(Node::CData(c.into_inner().into_owned())),
                Event::Comment(c) => stack
                    .last_mut()
                    .expect("non-empty stack")
                    .children
                    .push(Node::Comment(c.into_inner().into_owned())),
                // quick-xml reports `&amp;` and `&#12345;` as their own events
                // rather than folding them into the surrounding text.
                Event::GeneralRef(reference) => {
                    let name = reference.into_inner();
                    let resolved = resolve_entity(&name).ok_or_else(|| {
                        Error::malformed(&self.path, format!("unknown entity `&{name};`"))
                    })?;
                    stack
                        .last_mut()
                        .expect("non-empty stack")
                        .children
                        .push(Node::Text(resolved));
                }
                Event::Eof => {
                    return Err(Error::malformed(
                        &self.path,
                        format!(
                            "unexpected end of document inside <{}>",
                            stack[0].name.local
                        ),
                    ));
                }
                _ => continue,
            }
        }
    }

    /// Builds a childless [`Element`] from a start tag, resolving the element and
    /// attribute names and discarding `xmlns` declarations.
    fn element(&self, start: &BytesStart<'i>) -> Result<Element> {
        let resolver = self.inner.resolver();
        let (ns, local) = resolver.resolve_element(start.name());
        let name = Name::new(namespace(&ns), local.as_ref());

        let mut attrs = Vec::new();
        for attr in start.attributes().with_checks(false) {
            let attr = attr.map_err(|e| Error::malformed(&self.path, e.to_string()))?;
            if attr.key.as_namespace_binding().is_some() {
                continue;
            }
            let value = attr
                .normalized_value(XmlVersion::Explicit1_0)
                .map_err(|e| Error::xml(&self.path, e))?
                .into_owned();
            let (attr_ns, attr_local) = resolver.resolve_attribute(attr.key);
            attrs.push(Attribute {
                name: Name::new(namespace(&attr_ns), attr_local.as_ref()),
                value,
            });
        }

        Ok(Element {
            name,
            attrs,
            children: Vec::new(),
        })
    }
}

/// Expands an entity reference by name (`amp`, `#38`, `#x26`).
///
/// CityGML has no DTD, so anything beyond the predefined entities and numeric
/// character references cannot be resolved — and guessing would silently corrupt
/// the document.
fn resolve_entity(name: &str) -> Option<String> {
    quick_xml::escape::unescape(&format!("&{name};"))
        .ok()
        .map(|text| text.into_owned())
}

fn namespace(result: &ResolveResult<'_>) -> Option<String> {
    match result {
        ResolveResult::Bound(ns) => Some(ns.0.to_owned()),
        // An unresolvable prefix is a defect in the source document. Treating it
        // as unqualified keeps the element (and a diffable output) rather than
        // aborting the whole file.
        ResolveResult::Unbound | ResolveResult::Unknown(_) => None,
    }
}

/// Drops whitespace-only text from mixed-free elements so the writer can indent.
fn tidy(el: &mut Element) {
    if !el.has_element_children() {
        return;
    }
    el.children.retain(|c| match c {
        Node::Text(t) => !t.trim().is_empty(),
        _ => true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_namespaces_are_read_without_the_rest_of_the_document() {
        let src = r#"<?xml version="1.0"?>
            <core:CityModel xmlns:core="urn:core" xmlns:uro="https://www.geospatial.jp/iur/uro/3.1"
                            xsi:schemaLocation="ignored">
              <core:cityObjectMember/>
            </core:CityModel>"#;
        let found = root_namespaces(src);
        assert!(found.contains(&"urn:core".to_string()));
        assert!(found.contains(&"https://www.geospatial.jp/iur/uro/3.1".to_string()));
        assert_eq!(
            found.len(),
            2,
            "only xmlns declarations, not other attributes"
        );
    }

    /// Detection must not depend on the document being complete: PLATEAU `.gml`
    /// files reach hundreds of megabytes and only the head is ever read.
    #[test]
    fn a_truncated_document_still_yields_its_root_namespaces() {
        let src = r#"<core:CityModel xmlns:core="urn:core" xmlns:uro="urn:uro"><core:cityObj"#;
        assert_eq!(root_namespaces(src), ["urn:core", "urn:uro"]);
    }

    #[test]
    fn resolves_namespaces_and_drops_xmlns_attrs() {
        let src = r#"<?xml version="1.0"?>
            <core:CityModel xmlns:core="urn:core" xmlns:gml="urn:gml">
              <core:cityObjectMember>
                <core:Thing gml:id="a" attr="b"/>
              </core:cityObjectMember>
            </core:CityModel>"#;
        let mut r = Reader::new("test.gml", src);

        assert!(matches!(r.next_chunk().unwrap(), Some(Chunk::Decl)));

        let Some(Chunk::RootStart(root)) = r.next_chunk().unwrap() else {
            panic!("root")
        };
        assert!(root.is("urn:core", "CityModel"));
        assert!(
            root.attrs.is_empty(),
            "xmlns declarations must not become attributes"
        );

        let Some(Chunk::Member(member)) = r.next_chunk().unwrap() else {
            panic!("member")
        };
        assert!(member.is("urn:core", "cityObjectMember"));
        let thing = member.child("urn:core", "Thing").expect("Thing");
        assert_eq!(thing.attr(Some("urn:gml"), "id"), Some("a"));
        assert_eq!(
            thing.attr(None, "attr"),
            Some("b"),
            "unprefixed attrs stay unqualified"
        );

        assert!(matches!(r.next_chunk().unwrap(), Some(Chunk::RootEnd)));
        assert!(r.next_chunk().unwrap().is_none());
    }

    /// A self-closing root must produce the same chunk sequence as an empty
    /// <root></root>: without the RootEnd, the writer never closes the root
    /// tag it wrote and the output is not well-formed.
    #[test]
    fn a_self_closing_root_still_yields_its_end() {
        let mut r = Reader::new("t.gml", r#"<a xmlns="urn:x"/>"#);
        let Some(Chunk::RootStart(root)) = r.next_chunk().unwrap() else {
            panic!("root")
        };
        assert!(root.is("urn:x", "a"));
        assert!(matches!(r.next_chunk().unwrap(), Some(Chunk::RootEnd)));
        assert!(r.next_chunk().unwrap().is_none());
    }

    #[test]
    fn keeps_text_and_unescapes_it() {
        let src = r#"<a xmlns="urn:x"><b>1 &amp; 2</b></a>"#;
        let mut r = Reader::new("t.gml", src);
        let Some(Chunk::RootStart(_)) = r.next_chunk().unwrap() else {
            panic!()
        };
        let Some(Chunk::Member(b)) = r.next_chunk().unwrap() else {
            panic!()
        };
        assert_eq!(b.text(), "1 & 2");
    }

    #[test]
    fn expands_entity_references_in_text() {
        assert_eq!(resolve_entity("amp").as_deref(), Some("&"));
        assert_eq!(resolve_entity("#12354").as_deref(), Some("あ"));
        assert_eq!(resolve_entity("#x3042").as_deref(), Some("あ"));
        assert_eq!(
            resolve_entity("nbsp"),
            None,
            "no DTD means no custom entities"
        );
    }

    #[test]
    fn rejects_an_unresolvable_entity() {
        let mut r = Reader::new("t.gml", r#"<a xmlns="urn:x"><b>&nbsp;</b></a>"#);
        r.next_chunk().unwrap();
        let err = r.next_chunk().unwrap_err();
        assert!(err.to_string().contains("&nbsp;"), "{err}");
    }

    #[test]
    fn decodes_bom_and_shift_jis() {
        assert_eq!(decode(b"\xEF\xBB\xBFhi"), "hi");
        assert_eq!(decode(&[0x93, 0xFA, 0x96, 0x7B]), "日本");
    }
}
