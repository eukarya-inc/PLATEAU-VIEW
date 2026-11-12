//! Placing a GML geometry in the CityGML 3.0 `core` property that can hold
//! it. Knows nothing about thematic modules.
//!
//! A 2.0 `lodNGeometry` property holds any GML geometry, while 3.0 has one
//! property per geometry type and LOD. [`slot`] names the property for a
//! geometry type at a LOD, and [`wrap_single`] wraps a single surface or curve
//! in the multi-geometry that property expects.

use std::collections::HashSet;

use crate::xml::{Element, Name, Node};

/// GML solids a `core:lodNSolid` holds.
pub const SOLIDS: &[&str] = &["Solid", "CompositeSolid"];
/// GML surfaces a `gml:MultiSurface` member holds.
pub const SURFACES: &[&str] = &[
    "Polygon",
    "Surface",
    "CompositeSurface",
    "OrientableSurface",
];
/// GML curves a `gml:MultiCurve` member holds.
pub const CURVES: &[&str] = &["LineString", "Curve", "CompositeCurve", "OrientableCurve"];

/// The geometry property families a `lodN` prefix is followed by, in `core`
/// or in a module's namespace, including the 2.0 names a module's property
/// still carries before its rewrite, such as a `lod4*` or a plant cover's
/// `lodNMultiSolid`.
const FAMILIES: &[&str] = &[
    "Solid",
    "MultiSurface",
    "MultiCurve",
    "MultiSolid",
    "Point",
    "Geometry",
    "TerrainIntersection",
    "TerrainIntersectionCurve",
    "ImplicitRepresentation",
];

/// The LOD digit of a geometry property of local name `local`, in any
/// namespace, or `None` for anything else.
pub fn lod_of(local: &str) -> Option<char> {
    let rest = local.strip_prefix("lod")?;
    let mut chars = rest.chars();
    let lod = chars.next().filter(char::is_ascii_digit)?;
    FAMILIES.contains(&chars.as_str()).then_some(lod)
}

/// True when any geometry property sits under `el`.
pub fn has_geometry(el: &Element) -> bool {
    el.elements()
        .any(|child| lod_of(&child.name.local).is_some() || has_geometry(child))
}

/// The suffix of the `core:lod{lod}*` property a GML geometry of local name
/// `kind` goes to, one of `Solid`, `MultiSurface`, `MultiCurve` and `Point`,
/// or `None` when 3.0 has no property for it at that LOD.
pub fn slot(kind: &str, lod: char) -> Option<&'static str> {
    if SOLIDS.contains(&kind) {
        (lod != '0').then_some("Solid")
    } else if lod == '1' {
        None
    } else if kind == "Point" {
        (lod == '0').then_some("Point")
    } else {
        multi_of(kind)
    }
}

/// The multi-geometry a single surface or curve of local name `kind` belongs
/// in, the multi-geometry itself included.
pub fn multi_of(kind: &str) -> Option<&'static str> {
    if kind == "MultiSurface" || SURFACES.contains(&kind) {
        Some("MultiSurface")
    } else if kind == "MultiCurve" || CURVES.contains(&kind) {
        Some("MultiCurve")
    } else {
        None
    }
}

/// Wraps the single geometry under `property` in a `gml:<multi>` unless it
/// already is one. `multi` is `MultiSurface` or `MultiCurve`, and `gml` the
/// GML namespace. The wrapper takes the geometry's `srsName` and
/// `srsDimension`.
pub fn wrap_single(property: &mut Element, multi: &str, gml: &str) {
    let Some(Node::Element(inner)) = property
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(_)))
        .map(|i| property.children.remove(i))
    else {
        return;
    };
    if inner.is(gml, multi) {
        property.push(inner);
        return;
    }
    let member = if multi == "MultiSurface" {
        "surfaceMember"
    } else {
        "curveMember"
    };
    let mut wrapper = Element::new(Name::qualified(gml, multi));
    for attr in ["srsName", "srsDimension"] {
        if let Some(value) = inner.attr(None, attr) {
            wrapper.set_attr(Name::unqualified(attr), value);
        }
    }
    let mut item = Element::new(Name::qualified(gml, member));
    item.push(inner);
    wrapper.push(item);
    property.push(wrapper);
}

/// Adds the `gml:id` of `el` and of every descendant to `out`.
pub fn collect_ids(el: &Element, gml: &str, out: &mut HashSet<String>) {
    if let Some(id) = el.attr(Some(gml), "id") {
        out.insert(id.to_owned());
    }
    for child in el.elements() {
        collect_ids(child, gml, out);
    }
}
