//! The profile-driven passes, namely: rename and drop, child reordering, and
//! GML 3.2 `gml:id` assignment.
//!
//! [`rename`] runs first, so every later pass sees 3.0 names only. [`reorder`]
//! runs last, once the structural rewrites have finished adding and removing
//! children.

use crate::profile::Rules;
use crate::xml::{Element, Name, Node};

/// Applies the profile's element rules and namespace bump to a subtree.
///
/// Returns `None` when the profile drops the element outright.
pub fn rename(rules: &Rules, mut el: Element) -> Option<Element> {
    el.name = rules.map_element(&el.name)?;
    for attr in &mut el.attrs {
        attr.name = rules.map_attribute(&attr.name);
    }
    let children = std::mem::take(&mut el.children);
    el.children = children
        .into_iter()
        .filter_map(|child| match child {
            Node::Element(child) => rename(rules, child).map(Node::Element),
            other => Some(other),
        })
        .collect();
    Some(el)
}

/// Sorts children into the order the profile declares for their parent's type.
///
/// Children the profile does not mention keep their relative order and follow
/// the ones it does. An element mixing character data with child elements is
/// left alone. A comment does not block the sort and travels with the element
/// that follows it.
pub fn reorder(rules: &Rules, el: &mut Element) {
    if let Some(order) = rules.child_order(&el.name) {
        let mixed = el
            .children
            .iter()
            .any(|c| matches!(c, Node::Text(_) | Node::CData(_)));
        if !mixed {
            let mut groups: Vec<(usize, Vec<Node>)> = Vec::new();
            let mut pending: Vec<Node> = Vec::new();
            for child in std::mem::take(&mut el.children) {
                match child {
                    Node::Element(e) => {
                        let key = order
                            .iter()
                            .position(|o| *o == e.name)
                            .unwrap_or(usize::MAX);
                        pending.push(Node::Element(e));
                        groups.push((key, std::mem::take(&mut pending)));
                    }
                    other => pending.push(other),
                }
            }
            groups.sort_by_key(|(key, _)| *key);
            el.children = groups.into_iter().flat_map(|(_, nodes)| nodes).collect();
            el.children.append(&mut pending);
        }
    }
    for child in el.elements_mut() {
        reorder(rules, child);
    }
}

/// GML element names that take a mandatory `gml:id` in GML 3.2 and are left
/// without one by PLATEAU 2.0 files.
const ID_REQUIRED: &[&str] = &[
    "Point",
    "MultiPoint",
    "LineString",
    "Curve",
    "CompositeCurve",
    "MultiCurve",
    "Polygon",
    "Surface",
    "OrientableSurface",
    "CompositeSurface",
    "TriangulatedSurface",
    "Tin",
    "MultiSurface",
    "Solid",
    "CompositeSolid",
    "MultiSolid",
    "MultiGeometry",
    "GeometricComplex",
];

/// Mints deterministic `gml:id` values within one feature.
///
/// Ids are `<seed>_<n>` with `n` counted from 1, so the same input always
/// produces the same ids.
#[derive(Debug, Clone)]
pub struct IdGen {
    seed: String,
    next: usize,
}

impl IdGen {
    /// `seed` is normally the enclosing feature's `gml:id`, and is sanitised
    /// so the result is a valid `xsd:ID`.
    pub fn new(seed: &str) -> Self {
        IdGen {
            seed: id_seed(seed),
            next: 1,
        }
    }

    pub fn mint(&mut self) -> String {
        let id = format!("{}_{}", self.seed, self.next);
        self.next += 1;
        id
    }
}

/// Assigns a `gml:id` to geometries in `el` that lack one.
pub fn assign_gml_ids(el: &mut Element, gml_ns: &str, ids: &mut IdGen) {
    if el.name.in_ns(gml_ns)
        && ID_REQUIRED.contains(&el.name.local.as_str())
        && el.attr(Some(gml_ns), "id").is_none()
    {
        let id = ids.mint();
        el.set_attr(Name::qualified(gml_ns, "id"), id);
    }
    for child in el.elements_mut() {
        assign_gml_ids(child, gml_ns, ids);
    }
}

/// Turns an arbitrary string into something usable as the leading part of an
/// `xsd:ID`, which may not start with a digit.
pub fn id_seed(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 1);
    for (i, ch) in raw.chars().enumerate() {
        let ok = ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.';
        match (i, ok) {
            (0, true) if ch.is_alphanumeric() && !ch.is_alphabetic() => {
                out.push('_');
                out.push(ch);
            }
            (_, true) => out.push(ch),
            (_, false) => out.push('_'),
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_PROFILE;
    use crate::xml::ns;

    fn rules() -> Rules {
        Rules::from_toml(DEFAULT_PROFILE).unwrap()
    }

    #[test]
    fn renames_recursively_and_bumps_attribute_namespaces() {
        let mut solid = Element::new(Name::qualified(ns::GML_31, "Solid"));
        solid.set_attr(Name::qualified(ns::GML_31, "id"), "s1");
        let mut prop = Element::new(Name::qualified(ns::BUILDING_2, "lod1Solid"));
        prop.push(solid);
        let mut building = Element::new(Name::qualified(ns::BUILDING_2, "Building"));
        building.push(prop);

        let out = rename(&rules(), building).unwrap();
        assert!(out.is(ns::BUILDING_3, "Building"));
        let prop = out
            .child(ns::CITYGML_3, "lod1Solid")
            .expect("lod1Solid moved to core");
        let solid = prop.child(ns::GML_32, "Solid").expect("gml 3.2 Solid");
        assert_eq!(solid.attr(Some(ns::GML_32), "id"), Some("s1"));
    }

    /// A comment does not turn `reorder` into a no-op, and travels with the
    /// element that follows it.
    #[test]
    fn a_comment_does_not_disable_the_sort_and_travels_with_its_element() {
        let mut b = Element::new(Name::qualified(ns::BUILDING_3, "Building"));
        b.push(Element::new(Name::qualified(ns::CONSTRUCTION_3, "height")));
        b.children.push(Node::Comment(" the date ".into()));
        b.push(Element::new(Name::qualified(ns::CITYGML_3, "creationDate")));
        b.children.push(Node::Comment(" trailing ".into()));

        reorder(&rules(), &mut b);

        let names: Vec<&str> = b.elements().map(|e| e.name.local.as_str()).collect();
        assert_eq!(names, ["creationDate", "height"], "the sort still ran");
        let shape: Vec<&str> = b
            .children
            .iter()
            .map(|c| match c {
                Node::Element(e) => e.name.local.as_str(),
                Node::Comment(t) => t.trim(),
                _ => "?",
            })
            .collect();
        assert_eq!(shape, ["the date", "creationDate", "height", "trailing"]);
    }

    /// Text mixed in means the children are a value, so they stay put.
    #[test]
    fn mixed_content_is_still_left_alone() {
        let mut b = Element::new(Name::qualified(ns::BUILDING_3, "Building"));
        b.push(Element::new(Name::qualified(ns::CONSTRUCTION_3, "height")));
        b.children.push(Node::Text("value".into()));
        b.push(Element::new(Name::qualified(ns::CITYGML_3, "creationDate")));

        reorder(&rules(), &mut b);

        let names: Vec<&str> = b.elements().map(|e| e.name.local.as_str()).collect();
        assert_eq!(names, ["height", "creationDate"]);
    }

    #[test]
    fn reorder_puts_declared_children_first_and_keeps_the_rest_stable() {
        let mut b = Element::new(Name::qualified(ns::BUILDING_3, "Building"));
        b.push(Element::new(Name::qualified(
            ns::BUILDING_3,
            "buildingPart",
        )));
        b.push(Element::new(Name::qualified("urn:custom", "z")));
        b.push(Element::new(Name::qualified(ns::CONSTRUCTION_3, "height")));
        b.push(Element::new(Name::qualified("urn:custom", "a")));
        b.push(Element::new(Name::qualified(ns::CITYGML_3, "creationDate")));
        b.push(Element::new(Name::qualified(ns::CITYGML_3, "lod1Solid")));

        reorder(&rules(), &mut b);

        let names: Vec<&str> = b.elements().map(|e| e.name.local.as_str()).collect();
        assert_eq!(
            names,
            [
                "creationDate",
                "lod1Solid",
                "height",
                "buildingPart",
                "z",
                "a"
            ]
        );
    }

    #[test]
    fn assigns_ids_only_where_gml_32_requires_them() {
        let mut ms = Element::new(Name::qualified(ns::GML_32, "MultiSurface"));
        let mut poly = Element::new(Name::qualified(ns::GML_32, "Polygon"));
        poly.push(Element::new(Name::qualified(ns::GML_32, "LinearRing")));
        ms.push(poly);

        let mut ids = IdGen::new("bldg_x");
        assign_gml_ids(&mut ms, ns::GML_32, &mut ids);

        assert_eq!(ms.attr(Some(ns::GML_32), "id"), Some("bldg_x_1"));
        let poly = ms.child(ns::GML_32, "Polygon").unwrap();
        assert_eq!(poly.attr(Some(ns::GML_32), "id"), Some("bldg_x_2"));
        // LinearRing is a gml:AbstractRing rather than a gml:AbstractGML, so
        // it takes no id.
        assert_eq!(poly.child(ns::GML_32, "LinearRing").unwrap().attrs.len(), 0);
        assert_eq!(ids.mint(), "bldg_x_3");
    }

    #[test]
    fn id_seeds_never_start_with_a_digit() {
        assert_eq!(id_seed("52382287_bldg"), "_52382287_bldg");
        assert_eq!(id_seed("bldg_53e2"), "bldg_53e2");
        assert_eq!(id_seed("a b/c"), "a_b_c");
        assert_eq!(id_seed(""), "_");
    }
}
