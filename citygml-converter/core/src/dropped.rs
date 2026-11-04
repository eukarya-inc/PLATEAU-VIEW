//! The geometry the passes remove from a document, and restoring what the
//! geometry left behind still links to.
//!
//! The removing passes hand each subtree they remove to [`Dropped::add`],
//! which records every `gml:id` in it. [`Dropped::begin`] reads the links a
//! member's GML elements make by `xlink:href` before those passes run, so
//! `add` keeps a copy of each removed GML object one of them names.
//! [`Dropped::restore`] then writes that copy in place of the first link
//! still naming it, and its ids stop counting as removed. Later links to it
//! resolve to the restored copy. Has no module knowledge.

use std::collections::{HashMap, HashSet};

use crate::report::Warnings;
use crate::xml::{Element, Name, ns};

#[derive(Debug)]
pub struct Dropped {
    gml: String,
    xlink: String,
    /// Every `gml:id` removed so far in the document and not restored.
    ids: HashSet<String>,
    /// The ids the current member's GML elements link to.
    linked: HashSet<String>,
    /// Copies of the removed GML objects of the current member named in
    /// `linked`, by id.
    held: HashMap<String, Element>,
}

impl Default for Dropped {
    fn default() -> Self {
        Dropped::new(ns::GML_32, ns::XLINK)
    }
}

impl Dropped {
    /// `gml` and `xlink` are the output namespaces of GML and XLink.
    pub fn new(gml: &str, xlink: &str) -> Self {
        Dropped {
            gml: gml.to_owned(),
            xlink: xlink.to_owned(),
            ids: HashSet::new(),
            linked: HashSet::new(),
            held: HashMap::new(),
        }
    }

    /// Starts a member, reading the ids its GML elements link to.
    pub fn begin(&mut self, member: &Element) {
        self.held.clear();
        self.linked.clear();
        let mut linked = HashSet::new();
        self.links(member, &mut linked);
        self.linked = linked;
    }

    /// Records the ids under `el`, which a pass removed.
    pub fn add(&mut self, el: &Element) {
        if let Some(id) = el.attr(Some(&self.gml), "id") {
            if el.name.in_ns(&self.gml) && self.linked.contains(id) && !self.held.contains_key(id) {
                self.held.insert(id.to_owned(), el.clone());
            }
            self.ids.insert(id.to_owned());
        }
        for child in el.elements() {
            self.add(child);
        }
    }

    /// Writes each removed GML object of the current member in place of the
    /// first link under `member` that names it, and reports each link to a
    /// removed object that is left, which names one removed from another
    /// member.
    pub fn restore(&mut self, member: &mut Element, warnings: &mut Warnings) {
        if self.ids.is_empty() {
            return;
        }
        if let Some(id) = self.link(member).filter(|id| self.ids.contains(*id)) {
            let id = id.to_owned();
            match self.held.remove(&id) {
                Some(object) => {
                    member.take_attr(Some(&self.xlink), "href");
                    let mut restored = HashSet::new();
                    collect(&object, &self.gml, &mut restored);
                    self.ids.retain(|i| !restored.contains(i));
                    warnings.add(format!(
                        "gml:{} was written inside {} in place of its link: a pass dropped \
                         it, and kept geometry still linked to it",
                        object.name.local,
                        qualified(&member.name, &self.gml)
                    ));
                    member.push(object);
                }
                None => warnings.add(format!(
                    "{} links to geometry dropped from another member, and the link was \
                     kept",
                    qualified(&member.name, &self.gml)
                )),
            }
        }
        for child in member.elements_mut() {
            self.restore(child, warnings);
        }
    }

    pub fn ids(&self) -> &HashSet<String> {
        &self.ids
    }

    pub fn contains(&self, id: &str) -> bool {
        self.ids.contains(id)
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// The id an empty GML element links to by a same-document
    /// `xlink:href`.
    fn link<'a>(&self, el: &'a Element) -> Option<&'a str> {
        if !el.name.in_ns(&self.gml) || el.has_element_children() {
            return None;
        }
        el.attr(Some(&self.xlink), "href")?.strip_prefix('#')
    }

    fn links(&self, el: &Element, out: &mut HashSet<String>) {
        if let Some(id) = self.link(el) {
            out.insert(id.to_owned());
        }
        for child in el.elements() {
            self.links(child, out);
        }
    }
}

fn collect(el: &Element, gml: &str, out: &mut HashSet<String>) {
    if let Some(id) = el.attr(Some(gml), "id") {
        out.insert(id.to_owned());
    }
    for child in el.elements() {
        collect(child, gml, out);
    }
}

fn qualified(name: &Name, gml: &str) -> String {
    if name.in_ns(gml) {
        format!("gml:{}", name.local)
    } else {
        name.local.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gml(local: &str) -> Element {
        Element::new(Name::qualified(ns::GML_32, local))
    }

    fn with_id(mut el: Element, id: &str) -> Element {
        el.set_attr(Name::qualified(ns::GML_32, "id"), id);
        el
    }

    fn link(local: &str, id: &str) -> Element {
        let mut el = gml(local);
        el.set_attr(Name::qualified(ns::XLINK, "href"), format!("#{id}"));
        el
    }

    /// A removed polygon two kept links name comes back once, at the first,
    /// and stops counting as removed, while an unlinked one stays removed.
    #[test]
    fn a_linked_removal_comes_back_at_its_first_link() {
        let mut kept = gml("Solid");
        kept.push(link("surfaceMember", "p1"));
        kept.push(link("surfaceMember", "p1"));
        let mut gone = gml("MultiSurface");
        let mut ring = gml("LinearRing");
        ring = with_id(ring, "r1");
        let mut polygon = with_id(gml("Polygon"), "p1");
        polygon.push(ring);
        gone.push(polygon);
        gone.push(with_id(gml("Polygon"), "p2"));
        let mut member = gml("member");
        member.push(kept);
        member.push(gone.clone());

        let mut dropped = Dropped::default();
        dropped.begin(&member);
        dropped.add(&gone);
        member.children.pop();
        dropped.restore(&mut member, &mut Warnings::new());

        let solid = member.elements().next().unwrap();
        let members: Vec<_> = solid.elements().collect();
        assert_eq!(
            members[0]
                .elements()
                .next()
                .map(|p| p.attr(Some(ns::GML_32), "id")),
            Some(Some("p1"))
        );
        assert_eq!(members[0].attr(Some(ns::XLINK), "href"), None);
        assert_eq!(members[1].attr(Some(ns::XLINK), "href"), Some("#p1"));
        assert!(!dropped.contains("p1") && !dropped.contains("r1"));
        assert!(dropped.contains("p2"));
    }
}
