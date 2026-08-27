//! The declarative half of the mapping, loaded from a TOML profile.

use std::collections::HashMap;
use std::sync::Arc;

use indexmap::IndexMap;
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::xml::{Name, PrefixMap};

/// A conversion profile exactly as it appears on disk.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub input: NamespaceSet,
    pub output: Output,
    /// Input namespace URI -> output namespace URI, applied to any name without
    /// a more specific rule.
    #[serde(default)]
    pub namespace_map: IndexMap<String, String>,
    #[serde(default)]
    pub element: Vec<ElementRule>,
    #[serde(default)]
    pub order_group: Vec<OrderGroup>,
    #[serde(default)]
    pub height: HeightDefaults,
    #[serde(default)]
    pub review: Vec<ReviewNote>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamespaceSet {
    #[serde(default)]
    pub namespaces: IndexMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub namespaces: IndexMap<String, String>,
    #[serde(default)]
    pub schema_locations: IndexMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementRule {
    pub from: String,
    #[serde(default)]
    pub to: Option<String>,
    /// Remove the element (and its subtree) instead of renaming it.
    #[serde(default)]
    pub drop: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderGroup {
    pub types: Vec<String>,
    pub children: Vec<String>,
}

/// An element the profile deliberately leaves alone because converting it needs
/// a decision the converter must not make on its own.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewNote {
    /// The name as it appears *after* conversion, using output prefixes.
    pub element: String,
    pub note: String,
}

/// The parts of a `con:Height` that CityGML 2.0 does not record and the
/// converter therefore has to assume.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct HeightDefaults {
    /// `con:status`. An enumeration in 3.0, so it carries no `codeSpace`.
    pub status: String,
    /// `con:lowReference`, a code from the elevation-reference code list.
    pub low_reference: String,
    /// `con:highReference`, likewise.
    pub high_reference: String,
    /// `codeSpace` written on both reference elements.
    pub reference_code_space: Option<String>,
}

impl Default for HeightDefaults {
    fn default() -> Self {
        HeightDefaults {
            status: "measured".to_string(),
            low_reference: "6".to_string(),
            high_reference: "2".to_string(),
            reference_code_space: Some(
                "../../codelists/Elevation_elevationReference.xml".to_string(),
            ),
        }
    }
}

/// A profile with every `prefix:local` resolved to an expanded name, ready to
/// apply.
#[derive(Debug, Clone)]
pub struct Rules {
    name: String,
    namespace_map: HashMap<String, String>,
    /// `None` means "drop this element".
    renames: HashMap<Name, Option<Name>>,
    order: HashMap<Name, Arc<Vec<Name>>>,
    output_namespaces: IndexMap<String, String>,
    prefixes: PrefixMap,
    schema_location: Option<String>,
    height: HeightDefaults,
    reviews: HashMap<Name, Arc<str>>,
}

impl Rules {
    pub fn from_toml(source: &str) -> Result<Self> {
        Self::compile(&toml::from_str::<Profile>(source)?)
    }

    pub fn compile(profile: &Profile) -> Result<Self> {
        let input = &profile.input.namespaces;
        let output = &profile.output.namespaces;

        let mut renames = HashMap::new();
        for rule in &profile.element {
            let from = parse_name(&rule.from, input)?;
            let to = match (&rule.to, rule.drop) {
                (Some(_), true) => {
                    return Err(Error::Profile(format!(
                        "rule for `{}` sets both `to` and `drop`",
                        rule.from
                    )));
                }
                (Some(to), false) => Some(parse_name(to, output)?),
                (None, true) => None,
                (None, false) => {
                    return Err(Error::Profile(format!(
                        "rule for `{}` needs either `to` or `drop = true`",
                        rule.from
                    )));
                }
            };
            if let Some(previous) = renames.insert(from, to) {
                return Err(Error::Profile(format!(
                    "duplicate rule for `{}` (previously mapped to {previous:?})",
                    rule.from
                )));
            }
        }

        let mut order = HashMap::new();
        for group in &profile.order_group {
            let children: Vec<Name> = group
                .children
                .iter()
                .map(|c| parse_name(c, output))
                .collect::<Result<_>>()?;
            let children = Arc::new(children);
            for ty in &group.types {
                order.insert(parse_name(ty, output)?, Arc::clone(&children));
            }
        }

        let mut reviews = HashMap::new();
        for note in &profile.review {
            let name = parse_name(&note.element, output)?;
            if reviews
                .insert(name, Arc::from(note.note.as_str()))
                .is_some()
            {
                return Err(Error::Profile(format!(
                    "duplicate review note for `{}`",
                    note.element
                )));
            }
        }

        let mut prefixes = PrefixMap::new();
        for (prefix, uri) in output {
            prefixes.insert(prefix.clone(), uri.clone());
        }

        let schema_location = if profile.output.schema_locations.is_empty() {
            None
        } else {
            Some(
                profile
                    .output
                    .schema_locations
                    .iter()
                    .map(|(ns, loc)| format!("{ns} {loc}"))
                    .collect::<Vec<_>>()
                    .join("\n\t\t"),
            )
        };

        Ok(Rules {
            name: profile.name.clone(),
            namespace_map: profile
                .namespace_map
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            renames,
            order,
            output_namespaces: output.clone(),
            prefixes,
            schema_location,
            height: profile.height.clone(),
            reviews,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The output name for an element, or `None` if the profile drops it.
    pub fn map_element(&self, name: &Name) -> Option<Name> {
        match self.renames.get(name) {
            Some(mapped) => mapped.clone(),
            None => Some(self.bump_namespace(name)),
        }
    }

    /// The output name for an attribute. Element rules do not apply; only the
    /// namespace bump does.
    pub fn map_attribute(&self, name: &Name) -> Name {
        self.bump_namespace(name)
    }

    fn bump_namespace(&self, name: &Name) -> Name {
        match name.ns.as_deref().and_then(|ns| self.namespace_map.get(ns)) {
            Some(mapped) => Name::qualified(mapped.clone(), name.local.clone()),
            None => name.clone(),
        }
    }

    /// The required child order for an output type, if the profile declares one.
    pub fn child_order(&self, name: &Name) -> Option<&[Name]> {
        self.order.get(name).map(|v| v.as_slice())
    }

    pub fn prefixes(&self) -> &PrefixMap {
        &self.prefixes
    }

    pub fn schema_location(&self) -> Option<&str> {
        self.schema_location.as_deref()
    }

    pub fn height(&self) -> &HeightDefaults {
        &self.height
    }

    /// The note to report when this element survives conversion untouched, if
    /// the profile flags it as needing a human decision.
    pub fn review_note(&self, name: &Name) -> Option<&str> {
        self.reviews.get(name).map(|note| &**note)
    }

    /// Renders a name the way the output document writes it, for diagnostics.
    pub fn display_name(&self, name: &Name) -> String {
        match name
            .ns
            .as_deref()
            .and_then(|ns| self.prefixes.prefix_of(ns))
        {
            Some(prefix) if !prefix.is_empty() => format!("{prefix}:{}", name.local),
            _ => name.local.clone(),
        }
    }

    /// Looks up an output namespace URI by the prefix the profile assigns it.
    pub fn output_ns(&self, prefix: &str) -> Result<&str> {
        self.output_namespaces
            .get(prefix)
            .map(String::as_str)
            .ok_or_else(|| {
                Error::Profile(format!(
                    "profile `{}` declares no output namespace for prefix `{prefix}`",
                    self.name
                ))
            })
    }
}

/// Resolves a `prefix:local` name against a prefix table. A name with no prefix
/// is unqualified.
fn parse_name(text: &str, namespaces: &IndexMap<String, String>) -> Result<Name> {
    match text.split_once(':') {
        None => Ok(Name::unqualified(text)),
        Some((prefix, local)) => {
            if local.is_empty() {
                return Err(Error::Profile(format!("`{text}` has no local name")));
            }
            let uri = namespaces.get(prefix).ok_or_else(|| {
                Error::Profile(format!("`{text}` uses undeclared prefix `{prefix}`"))
            })?;
            Ok(Name::qualified(uri.clone(), local))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_PROFILE;
    use crate::xml::ns;

    fn rules() -> Rules {
        Rules::from_toml(DEFAULT_PROFILE).expect("default profile must compile")
    }

    #[test]
    fn default_profile_compiles() {
        let r = rules();
        assert_eq!(r.name(), "citygml-2.0-to-3.0");
        assert_eq!(r.prefixes().prefix_of(ns::CITYGML_3), Some("core"));
        assert!(r.schema_location().is_some());
    }

    #[test]
    fn bumps_namespaces_by_default() {
        let r = rules();
        // No rule for CityModel: only its namespace changes.
        let out = r
            .map_element(&Name::qualified(ns::CITYGML_2, "CityModel"))
            .unwrap();
        assert_eq!(out, Name::qualified(ns::CITYGML_3, "CityModel"));
        // gml 3.1.1 -> 3.2
        let out = r.map_attribute(&Name::qualified(ns::GML_31, "id"));
        assert_eq!(out, Name::qualified(ns::GML_32, "id"));
    }

    #[test]
    fn applies_element_rules_over_the_namespace_bump() {
        let r = rules();
        let out = r
            .map_element(&Name::qualified(ns::BUILDING_2, "lod1Solid"))
            .unwrap();
        assert_eq!(out, Name::qualified(ns::CITYGML_3, "lod1Solid"));
        let out = r
            .map_element(&Name::qualified(ns::BUILDING_2, "WallSurface"))
            .unwrap();
        assert_eq!(out, Name::qualified(ns::CONSTRUCTION_3, "WallSurface"));
    }

    #[test]
    fn leaves_unmapped_namespaces_alone() {
        let r = rules();
        // xlink and xsi are version-independent and must not be touched.
        let xlink = Name::qualified(ns::XLINK, "href");
        assert_eq!(r.map_attribute(&xlink), xlink);
        assert_eq!(r.map_element(&xlink).unwrap(), xlink);
        let xal = Name::qualified("urn:oasis:names:tc:ciq:xsdschema:xAL:2.0", "Country");
        assert_eq!(r.map_element(&xal).unwrap(), xal);
    }

    /// i-UR 3.2 and 4.0 are compatible in principle, so the whole ADE moves by
    /// namespace alone.
    ///
    /// Every published 3.x minor has to be listed. An unmapped namespace has no
    /// prefix on the output side and its elements are written unqualified — and
    /// because the `[[review]]` rules match on the 4.0 names, the flags for the
    /// urc migration go quiet as well. Real data is mostly 3.1 and 3.2.
    #[test]
    fn bumps_every_i_ur_3_x_minor_to_4_0() {
        let r = rules();
        for (module, local) in [
            ("uro", "buildingID"),
            ("urf", "Zone"),
            ("urg", "genericTag"),
        ] {
            for minor in ["3.0", "3.1", "3.2"] {
                let from = Name::qualified(
                    format!("https://www.geospatial.jp/iur/{module}/{minor}"),
                    local,
                );
                assert_eq!(
                    r.map_element(&from).unwrap(),
                    Name::qualified(format!("https://www.geospatial.jp/iur/{module}/4.0"), local),
                    "{module} {minor}"
                );
            }
        }
    }

    #[test]
    fn declares_child_order_for_buildings() {
        let r = rules();
        let order = r
            .child_order(&Name::qualified(ns::BUILDING_3, "Building"))
            .expect("order");
        assert!(order.contains(&Name::qualified(ns::CONSTRUCTION_3, "height")));
    }

    /// i-UR 4.0 consolidations that need a decision are flagged, not guessed at.
    #[test]
    fn flags_elements_that_need_review() {
        let r = rules();
        let quality = Name::qualified(
            "https://www.geospatial.jp/iur/uro/4.0",
            "DataQualityAttribute",
        );
        let note = r
            .review_note(&quality)
            .expect("DataQualityAttribute is flagged");
        assert!(note.contains("urc:ExteriorDataQualityAttribute"), "{note}");
        // A type with a settled mapping is not flagged.
        let id = Name::qualified(
            "https://www.geospatial.jp/iur/uro/4.0",
            "BuildingIDAttribute",
        );
        assert!(r.review_note(&id).is_none());
    }

    #[test]
    fn rejects_a_rule_with_an_unknown_prefix() {
        let err = Rules::from_toml(
            r#"
            name = "t"
            [output.namespaces]
            core = "urn:core"
            [[element]]
            from = "nope:x"
            to = "core:x"
            "#,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("undeclared prefix `nope`"),
            "{err}"
        );
    }

    #[test]
    fn rejects_a_rule_with_neither_to_nor_drop() {
        let err = Rules::from_toml(
            r#"
            name = "t"
            [output.namespaces]
            core = "urn:core"
            [input.namespaces]
            core = "urn:old"
            [[element]]
            from = "core:x"
            "#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("needs either"), "{err}");
    }
}
