//! Structural rewrites for the appearance module.
//!
//! Like [`crate::bldg`] this runs *after* [`crate::transform::rename`], so the
//! elements arrive in the CityGML 3.0 appearance namespace. The pure renames —
//! the attachment properties moving into `core`, `surfaceDataMember` becoming
//! `surfaceData` — are profile rows; this pass handles the change a rename
//! table cannot express: how a texture binds to geometry.
//!
//! CityGML 2.0 bound a `ParameterizedTexture` to a surface with a `target`
//! property whose `uri` XML attribute named the geometry and whose content was
//! the parameterization, and each `textureCoordinates` list named its ring in
//! a `ring` XML attribute. CityGML 3.0 makes the association an object: a
//! `textureParameterization` property holds an `app:TextureAssociation` — a
//! GML object, so it needs a `gml:id` — carrying `target` as an element, and
//! `TexCoordList` lists coordinates and rings as parallel elements.
//!
//! ```text
//! 2.0   <app:target uri="#s"><app:TexCoordList>
//!         <app:textureCoordinates ring="#r">…</app:textureCoordinates>
//!       </app:TexCoordList></app:target>
//! 3.0   <app:textureParameterization><app:TextureAssociation gml:id="…">
//!         <app:target>#s</app:target>
//!         <app:textureParameterization><app:TexCoordList>
//!           <app:textureCoordinates>…</app:textureCoordinates>
//!           <app:ring>#r</app:ring>
//!         </app:TexCoordList></app:textureParameterization>
//!       </app:TextureAssociation></app:textureParameterization>
//! ```
//!
//! `GeoreferencedTexture` and `X3DMaterial` already wrote `target` as a plain
//! URI element in 2.0 and still do in 3.0, so they pass through untouched. The
//! appearance module is shared by every thematic module (`bldg`, `tran`,
//! `frn`, …), which is why this is its own pass rather than part of
//! [`crate::bldg`].

use crate::error::Result;
use crate::profile::Rules;
use crate::report::Warnings;
use crate::transform::IdGen;
use crate::xml::{Element, Name, Node};

/// The appearance rewrites, bound to one profile's output namespaces.
#[derive(Debug, Clone)]
pub struct AppearanceRewrite {
    app: String,
    gml: String,
}

impl AppearanceRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        Ok(AppearanceRewrite {
            app: rules.output_ns("app")?.to_owned(),
            gml: rules.output_ns("gml")?.to_owned(),
        })
    }

    /// Rewrites `el` and its descendants in place.
    pub fn apply(&self, el: &mut Element, ids: &mut IdGen, warnings: &mut Warnings) {
        if el.is(&self.app, "ParameterizedTexture") {
            self.texture_targets(el, ids, warnings);
        }
        if el.is(&self.app, "TexCoordList") {
            self.split_rings(el, warnings);
        }
        for child in el.elements_mut() {
            self.apply(child, ids, warnings);
        }
    }

    /// `app:target[@uri]` on a `ParameterizedTexture` ->
    /// `app:textureParameterization` holding an `app:TextureAssociation`
    /// object. A `target` without the 2.0 `uri` attribute is left alone and
    /// reported: there is no geometry reference to build the object from.
    fn texture_targets(&self, el: &mut Element, ids: &mut IdGen, warnings: &mut Warnings) {
        let children = std::mem::take(&mut el.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            match child {
                Node::Element(mut target) if target.is(&self.app, "target") => {
                    let Some(uri) = target.take_attr(None, "uri") else {
                        warnings.add(
                            "an app:target on a ParameterizedTexture has no uri \
                             attribute and was left unchanged; CityGML 3.0 expects \
                             the texture-to-surface link as an app:TextureAssociation",
                        );
                        out.push(Node::Element(target));
                        continue;
                    };

                    let mut assoc = Element::new(Name::qualified(&self.app, "TextureAssociation"));
                    assoc.set_attr(Name::qualified(&self.gml, "id"), ids.mint());
                    assoc.push(Element::with_text(
                        Name::qualified(&self.app, "target"),
                        uri,
                    ));
                    if target.has_element_children() {
                        let mut param =
                            Element::new(Name::qualified(&self.app, "textureParameterization"));
                        param.children = target.children;
                        assoc.push(param);
                    } else {
                        warnings.add(
                            "an app:target carried no texture parameterization, which \
                             CityGML 3.0 requires on an app:TextureAssociation; check \
                             the result",
                        );
                    }
                    warnings.add(
                        "app:target with a uri attribute became \
                         app:textureParameterization holding an app:TextureAssociation: \
                         CityGML 3.0 makes the texture-to-surface link an object with \
                         target as an element",
                    );

                    let mut prop =
                        Element::new(Name::qualified(&self.app, "textureParameterization"));
                    prop.push(assoc);
                    out.push(Node::Element(prop));
                }
                other => out.push(other),
            }
        }
        el.children = out;
    }

    /// `textureCoordinates[@ring]` -> parallel `textureCoordinates` and `ring`
    /// elements, rings in the same order as the coordinate lists they belong to.
    fn split_rings(&self, el: &mut Element, warnings: &mut Warnings) {
        let mut rings: Vec<String> = Vec::new();
        let mut missing = false;
        for child in el.elements_mut() {
            if !child.is(&self.app, "textureCoordinates") {
                continue;
            }
            match child.take_attr(None, "ring") {
                Some(ring) => rings.push(ring),
                None => missing = true,
            }
        }
        if rings.is_empty() {
            return; // already 3.0-shaped, or no coordinates at all
        }
        if missing {
            warnings.add(
                "an app:textureCoordinates without a ring attribute sits next to \
                 ones that have it; the app:ring elements may not line up with \
                 their coordinate lists",
            );
        }
        warnings.add(
            "the ring attribute of app:textureCoordinates became app:ring elements: \
             CityGML 3.0 lists coordinates and their rings as parallel elements",
        );
        for ring in rings {
            el.push(Element::with_text(Name::qualified(&self.app, "ring"), ring));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_PROFILE;

    const APP3: &str = "http://www.opengis.net/citygml/appearance/3.0";

    fn rewrite() -> AppearanceRewrite {
        AppearanceRewrite::new(&Rules::from_toml(DEFAULT_PROFILE).unwrap()).unwrap()
    }

    fn applied(mut el: Element) -> (Element, Warnings) {
        let mut warnings = Warnings::new();
        let mut ids = IdGen::new("ap_1");
        rewrite().apply(&mut el, &mut ids, &mut warnings);
        (el, warnings)
    }

    fn texture(targets: Vec<Element>) -> Element {
        let mut texture = Element::new(Name::qualified(APP3, "ParameterizedTexture"));
        texture.push(Element::with_text(
            Name::qualified(APP3, "imageURI"),
            "t.jpg",
        ));
        for target in targets {
            texture.push(target);
        }
        texture
    }

    fn target_20(uri: &str, ring: Option<&str>) -> Element {
        let mut coords =
            Element::with_text(Name::qualified(APP3, "textureCoordinates"), "0 0 1 0 1 1");
        if let Some(ring) = ring {
            coords.set_attr(Name::unqualified("ring"), ring);
        }
        let mut list = Element::new(Name::qualified(APP3, "TexCoordList"));
        list.push(coords);
        let mut target = Element::new(Name::qualified(APP3, "target"));
        target.set_attr(Name::unqualified("uri"), uri);
        target.push(list);
        target
    }

    #[test]
    fn a_target_without_a_uri_is_kept_and_reported() {
        let mut target = Element::new(Name::qualified(APP3, "target"));
        target.push(Element::new(Name::qualified(APP3, "TexCoordList")));
        let (texture, warnings) = applied(texture(vec![target]));
        assert!(texture.child(APP3, "target").is_some());
        assert!(warnings.iter().any(|(m, _)| m.contains("no uri")));
    }

    #[test]
    fn a_target_without_parameterization_is_reported() {
        let mut target = Element::new(Name::qualified(APP3, "target"));
        target.set_attr(Name::unqualified("uri"), "#s1");
        let (texture, warnings) = applied(texture(vec![target]));
        let assoc = texture
            .child(APP3, "textureParameterization")
            .unwrap()
            .child(APP3, "TextureAssociation")
            .unwrap();
        assert!(assoc.child(APP3, "textureParameterization").is_none());
        assert!(
            warnings
                .iter()
                .any(|(m, _)| m.contains("no texture parameterization"))
        );
    }

    /// Georeferenced textures and materials wrote `target` as a URI element in
    /// 2.0 already; they must pass through silently.
    #[test]
    fn material_and_georeferenced_targets_are_left_alone() {
        for kind in ["X3DMaterial", "GeoreferencedTexture"] {
            let mut owner = Element::new(Name::qualified(APP3, kind));
            owner.push(Element::with_text(Name::qualified(APP3, "target"), "#s1"));
            let (owner, warnings) = applied(owner);
            assert_eq!(owner.child(APP3, "target").unwrap().text(), "#s1", "{kind}");
            assert!(warnings.is_empty(), "{kind} must convert silently");
        }
    }

    /// Applying the pass to its own output must change nothing: the created
    /// target has no uri attribute and the rings are already elements.
    #[test]
    fn the_rewrite_is_idempotent() {
        let (once, _) = applied(texture(vec![target_20("#s1", Some("#r1"))]));
        let (twice, warnings) = applied(once.clone());
        assert_eq!(once, twice);
        assert!(warnings.is_empty());
    }

    #[test]
    fn coordinate_lists_split_their_rings_in_order() {
        let mut list = Element::new(Name::qualified(APP3, "TexCoordList"));
        for (coords, ring) in [("0 0", "#r1"), ("1 1", "#r2")] {
            let mut el = Element::with_text(Name::qualified(APP3, "textureCoordinates"), coords);
            el.set_attr(Name::unqualified("ring"), ring);
            list.push(el);
        }
        let (list, _) = applied(list);
        let locals: Vec<&str> = list.elements().map(|e| e.name.local.as_str()).collect();
        assert_eq!(
            locals,
            ["textureCoordinates", "textureCoordinates", "ring", "ring"]
        );
        let rings: Vec<String> = list
            .elements()
            .filter(|e| e.is(APP3, "ring"))
            .map(|e| e.text())
            .collect();
        assert_eq!(rings, ["#r1", "#r2"]);
    }
}
