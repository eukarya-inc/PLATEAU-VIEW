//! Extrudes a horizontal `gml:MultiSurface` into a `gml:Solid`.
//!
//! The input is a multi-surface of inline `gml:Polygon`s whose rings are
//! `gml:posList` values of three-dimensional tuples. Each polygon becomes one
//! solid with a `gml:Shell` of the polygon reversed as its bottom, the polygon
//! raised by the height as its top, and one wall per edge of every ring.
//! Several polygons yield a `gml:CompositeSolid`.
//!
//! Horizontal tokens are copied as written, and only the third token of each
//! tuple is parsed, raised and written back. `srsName` and `srsDimension` are
//! copied from the input multi-surface onto the result.

use crate::xml::{Element, Name};

const SRS_ATTRIBUTES: &[&str] = &["srsName", "srsDimension"];

/// Extrudes `multi_surface` upward by `height`.
///
/// `gml` is the GML namespace and `id` the `gml:id` of the result, from which
/// every solid and face id is derived. The error names what in the input
/// could not be extruded.
pub fn extrude(
    multi_surface: &Element,
    height: f64,
    gml: &str,
    id: &str,
) -> Result<Element, String> {
    let mut polygons = Vec::new();
    for member in multi_surface.elements() {
        if !member.is(gml, "surfaceMember") {
            continue;
        }
        let Some(polygon) = member.elements().next() else {
            return Err("a gml:surfaceMember holds no inline surface".to_owned());
        };
        if !polygon.is(gml, "Polygon") {
            return Err(format!(
                "a gml:surfaceMember holds a gml:{}, not a gml:Polygon",
                polygon.name.local
            ));
        }
        polygons.push(read_polygon(polygon, gml)?);
    }
    let mut out = match polygons.len() {
        0 => return Err("the multi-surface holds no polygon".to_owned()),
        1 => solid(&polygons[0], height, gml, id),
        _ => {
            let mut composite = Element::new(Name::qualified(gml, "CompositeSolid"));
            composite.set_attr(Name::qualified(gml, "id"), id);
            for (k, polygon) in polygons.iter().enumerate() {
                let mut member = Element::new(Name::qualified(gml, "solidMember"));
                member.push(solid(polygon, height, gml, &format!("{id}_{}", k + 1)));
                composite.push(member);
            }
            composite
        }
    };
    for name in SRS_ATTRIBUTES {
        if let Some(value) = multi_surface.attr(None, name) {
            let value = value.to_owned();
            out.set_attr(Name::unqualified(*name), value);
        }
    }
    Ok(out)
}

/// One vertex, its horizontal tokens as written and its height parsed.
#[derive(Debug, Clone)]
struct Vertex {
    horizontal: [String; 2],
    z: f64,
    z_text: String,
}

/// A ring as a cycle of distinct vertices, without the closing repeat.
type Ring = Vec<Vertex>;

/// The exterior ring followed by the interior rings.
type Polygon = Vec<Ring>;

fn read_polygon(polygon: &Element, gml: &str) -> Result<Polygon, String> {
    let mut rings = Vec::new();
    let exterior = polygon
        .child(gml, "exterior")
        .ok_or("a gml:Polygon has no gml:exterior")?;
    rings.push(read_ring(exterior, gml)?);
    for interior in polygon.elements().filter(|e| e.is(gml, "interior")) {
        rings.push(read_ring(interior, gml)?);
    }
    Ok(rings)
}

fn read_ring(boundary: &Element, gml: &str) -> Result<Ring, String> {
    let ring = boundary
        .elements()
        .next()
        .filter(|e| e.is(gml, "LinearRing"))
        .ok_or("a polygon boundary holds no gml:LinearRing")?;
    let pos_list = ring
        .child(gml, "posList")
        .ok_or("a gml:LinearRing holds no gml:posList")?;
    if let Some(dimension) = pos_list.attr(None, "srsDimension") {
        if dimension.trim() != "3" {
            return Err(format!("a gml:posList has srsDimension {dimension}"));
        }
    }
    let text = pos_list.text();
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.len() % 3 != 0 {
        return Err(format!(
            "a gml:posList holds {} values, which is not a multiple of 3",
            tokens.len()
        ));
    }
    let mut vertices: Ring = Vec::with_capacity(tokens.len() / 3);
    for tuple in tokens.chunks(3) {
        let parsed: Result<Vec<f64>, _> = tuple.iter().map(|t| t.parse::<f64>()).collect();
        let Ok(parsed) = parsed else {
            return Err("a gml:posList holds a value that is not a number".to_owned());
        };
        let vertex = Vertex {
            horizontal: [tuple[0].to_owned(), tuple[1].to_owned()],
            z: parsed[2],
            z_text: tuple[2].to_owned(),
        };
        let same_as_last = vertices.last().is_some_and(|last: &Vertex| {
            last.horizontal == vertex.horizontal && last.z == vertex.z
        });
        if !same_as_last {
            vertices.push(vertex);
        }
    }
    if vertices.len() > 1 {
        let (first, last) = (&vertices[0], &vertices[vertices.len() - 1]);
        if first.horizontal == last.horizontal && first.z == last.z {
            vertices.pop();
        }
    }
    if vertices.len() < 3 {
        return Err("a ring has fewer than three distinct points".to_owned());
    }
    Ok(vertices)
}

fn solid(polygon: &Polygon, height: f64, gml: &str, id: &str) -> Element {
    let mut faces: Vec<Vec<String>> = Vec::new();
    faces.push(
        polygon
            .iter()
            .map(|ring| ring_text(ring.iter().rev(), None))
            .collect(),
    );
    faces.push(
        polygon
            .iter()
            .map(|ring| ring_text(ring.iter(), Some(height)))
            .collect(),
    );
    for ring in polygon {
        for i in 0..ring.len() {
            let a = &ring[i];
            let b = &ring[(i + 1) % ring.len()];
            faces.push(vec![wall_text(a, b, height)]);
        }
    }

    let mut shell = Element::new(Name::qualified(gml, "Shell"));
    for (n, rings) in faces.iter().enumerate() {
        let mut member = Element::new(Name::qualified(gml, "surfaceMember"));
        member.push(face(rings, gml, &format!("{id}_{}", n + 1)));
        shell.push(member);
    }
    let mut exterior = Element::new(Name::qualified(gml, "exterior"));
    exterior.push(shell);
    let mut solid = Element::new(Name::qualified(gml, "Solid"));
    solid.set_attr(Name::qualified(gml, "id"), id);
    solid.push(exterior);
    solid
}

fn face(rings: &[String], gml: &str, id: &str) -> Element {
    let mut polygon = Element::new(Name::qualified(gml, "Polygon"));
    polygon.set_attr(Name::qualified(gml, "id"), id);
    for (i, text) in rings.iter().enumerate() {
        let role = if i == 0 { "exterior" } else { "interior" };
        let mut ring = Element::new(Name::qualified(gml, "LinearRing"));
        ring.push(Element::with_text(
            Name::qualified(gml, "posList"),
            text.clone(),
        ));
        let mut boundary = Element::new(Name::qualified(gml, role));
        boundary.push(ring);
        polygon.push(boundary);
    }
    polygon
}

fn ring_text<'a>(vertices: impl Iterator<Item = &'a Vertex>, raise: Option<f64>) -> String {
    let vertices: Vec<&Vertex> = vertices.collect();
    let mut tuples: Vec<String> = vertices.iter().map(|v| tuple(v, raise)).collect();
    tuples.push(tuple(vertices[0], raise));
    tuples.join(" ")
}

fn wall_text(a: &Vertex, b: &Vertex, height: f64) -> String {
    [
        tuple(a, None),
        tuple(b, None),
        tuple(b, Some(height)),
        tuple(a, Some(height)),
        tuple(a, None),
    ]
    .join(" ")
}

fn tuple(v: &Vertex, raise: Option<f64>) -> String {
    let z = match raise {
        Some(h) => (v.z + h).to_string(),
        None => v.z_text.clone(),
    };
    format!("{} {} {z}", v.horizontal[0], v.horizontal[1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml::ns;

    fn ring(role: &str, pos_list: &str) -> Element {
        let mut linear = Element::new(Name::qualified(ns::GML_32, "LinearRing"));
        linear.push(Element::with_text(
            Name::qualified(ns::GML_32, "posList"),
            pos_list,
        ));
        let mut boundary = Element::new(Name::qualified(ns::GML_32, role));
        boundary.push(linear);
        boundary
    }

    fn polygon(exterior: &str, interiors: &[&str]) -> Element {
        let mut polygon = Element::new(Name::qualified(ns::GML_32, "Polygon"));
        polygon.push(ring("exterior", exterior));
        for interior in interiors {
            polygon.push(ring("interior", interior));
        }
        polygon
    }

    fn multi_surface(polygons: Vec<Element>) -> Element {
        let mut ms = Element::new(Name::qualified(ns::GML_32, "MultiSurface"));
        ms.set_attr(Name::unqualified("srsName"), "EPSG:6697");
        ms.set_attr(Name::unqualified("srsDimension"), "3");
        for polygon in polygons {
            let mut member = Element::new(Name::qualified(ns::GML_32, "surfaceMember"));
            member.push(polygon);
            ms.push(member);
        }
        ms
    }

    fn faces(solid: &Element) -> Vec<&Element> {
        solid
            .child(ns::GML_32, "exterior")
            .unwrap()
            .child(ns::GML_32, "Shell")
            .unwrap()
            .elements()
            .filter_map(|m| m.elements().next())
            .collect()
    }

    fn pos_list(polygon: &Element, role: &str) -> String {
        polygon
            .child(ns::GML_32, role)
            .unwrap()
            .child(ns::GML_32, "LinearRing")
            .unwrap()
            .child(ns::GML_32, "posList")
            .unwrap()
            .text()
    }

    #[test]
    fn a_polygon_with_a_hole_becomes_a_closed_shell_of_ten_faces() {
        let outer = "0 0 0 0 10.5 0 10.5 10.5 0 10.5 0 0 0 0 0";
        let inner = "2 2 0 8 2 0 8 8 0 2 8 0 2 2 0";
        let ms = multi_surface(vec![polygon(outer, &[inner])]);
        let solid = extrude(&ms, 4.5, ns::GML_32, "s").unwrap();

        assert!(solid.is(ns::GML_32, "Solid"));
        assert_eq!(solid.attr(Some(ns::GML_32), "id"), Some("s"));
        assert_eq!(solid.attr(None, "srsName"), Some("EPSG:6697"));
        assert_eq!(solid.attr(None, "srsDimension"), Some("3"));
        let faces = faces(&solid);
        assert_eq!(faces.len(), 10);
        assert_eq!(faces[0].attr(Some(ns::GML_32), "id"), Some("s_1"));
        assert_eq!(faces[9].attr(Some(ns::GML_32), "id"), Some("s_10"));

        assert_eq!(
            pos_list(faces[0], "exterior"),
            "10.5 0 0 10.5 10.5 0 0 10.5 0 0 0 0 10.5 0 0"
        );
        assert_eq!(
            pos_list(faces[0], "interior"),
            "2 8 0 8 8 0 8 2 0 2 2 0 2 8 0"
        );
        assert_eq!(
            pos_list(faces[1], "exterior"),
            "0 0 4.5 0 10.5 4.5 10.5 10.5 4.5 10.5 0 4.5 0 0 4.5"
        );
        assert_eq!(
            pos_list(faces[1], "interior"),
            "2 2 4.5 8 2 4.5 8 8 4.5 2 8 4.5 2 2 4.5"
        );
        assert_eq!(
            pos_list(faces[2], "exterior"),
            "0 0 0 0 10.5 0 0 10.5 4.5 0 0 4.5 0 0 0"
        );
        assert_eq!(
            pos_list(faces[6], "exterior"),
            "2 2 0 8 2 0 8 2 4.5 2 2 4.5 2 2 0"
        );
        assert!(faces[2].child(ns::GML_32, "interior").is_none());
    }

    #[test]
    fn several_polygons_compose_and_a_reference_refuses() {
        let square = "0 0 0 1 0 0 1 1 0 0 1 0 0 0 0";
        let ms = multi_surface(vec![polygon(square, &[]), polygon(square, &[])]);
        let composite = extrude(&ms, 1.0, ns::GML_32, "c").unwrap();
        assert!(composite.is(ns::GML_32, "CompositeSolid"));
        assert_eq!(composite.attr(None, "srsName"), Some("EPSG:6697"));
        let solids: Vec<&Element> = composite
            .elements()
            .filter_map(|m| m.elements().next())
            .collect();
        assert_eq!(solids.len(), 2);
        assert_eq!(solids[1].attr(Some(ns::GML_32), "id"), Some("c_2"));
        assert_eq!(faces(solids[1]).len(), 6);

        let mut by_reference = Element::new(Name::qualified(ns::GML_32, "MultiSurface"));
        let mut member = Element::new(Name::qualified(ns::GML_32, "surfaceMember"));
        member.set_attr(Name::qualified(ns::XLINK, "href"), "#poly_1");
        by_reference.push(member);
        assert!(extrude(&by_reference, 1.0, ns::GML_32, "r").is_err());
    }
}
