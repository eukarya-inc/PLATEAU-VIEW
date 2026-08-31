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
    /// The CityGML and i-UR versions this profile accepts.
    #[serde(default)]
    pub source: Provenance,
    /// The versions it produces.
    #[serde(default)]
    pub target: Provenance,
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
    /// i-UR class -> the CityGML property that carries it. See [`Rules::ade_hook`].
    #[serde(default)]
    pub ade_hooks: IndexMap<String, String>,
    #[serde(default)]
    pub height: HeightDefaults,
    /// Where CityGML 2.0 LOD4 goes, since 3.0 has no LOD4. See [`Lod4Policy`].
    #[serde(default)]
    pub lod4: Lod4Policy,
    /// How the input's code lists are carried into the output. See
    /// [`CodelistsPolicy`].
    #[serde(default)]
    pub codelists: CodelistsPolicy,
}

/// One end of a conversion: what a profile reads, or what it writes.
///
/// `iur` is the discriminator. i-UR puts its minor version in the namespace URI,
/// so the namespaces a document declares say which version it is, and that is
/// what picks a profile.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Provenance {
    /// Human-readable, e.g. `CityGML 2.0 + i-UR 3.1`. Shown by `inspect`.
    pub label: String,
    pub citygml: Option<String>,
    /// The i-UR minor version, written plainly rather than parsed back out of
    /// the URIs below. On the target side this is what `--target-iur` selects.
    pub iur_version: String,
    pub iur: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamespaceSet {
    #[serde(default)]
    pub namespaces: IndexMap<String, NamespaceValue>,
}

/// The namespace URI, or URIs, an input prefix stands for.
///
/// i-UR publishes a new minor version most years and PLATEAU data carries
/// whichever one was current when it was published, so the same element lives
/// in three namespaces at once across a corpus. Binding a prefix to the whole
/// family keeps the element table one row per element instead of one row per
/// version, and stops a rule written against 3.0 from silently missing 3.1 and
/// 3.2 data.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NamespaceValue {
    One(String),
    Many(Vec<String>),
}

impl NamespaceValue {
    pub fn uris(&self) -> &[String] {
        match self {
            NamespaceValue::One(uri) => std::slice::from_ref(uri),
            NamespaceValue::Many(uris) => uris,
        }
    }
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

/// What to do with an LOD4 element when the data does not say which LOD it
/// should become.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lod4Fallback {
    /// Fold into LOD3, the closest 3.0 has to an interior model.
    Lod3,
    /// Fold into LOD2.
    Lod2,
    /// Remove the LOD4 geometry and its LOD4 quality descriptors.
    Drop,
}

impl Lod4Fallback {
    pub fn as_str(self) -> &'static str {
        match self {
            Lod4Fallback::Lod3 => "lod3",
            Lod4Fallback::Lod2 => "lod2",
            Lod4Fallback::Drop => "drop",
        }
    }
}

/// How CityGML 2.0 LOD4 is placed in 3.0, which stops at LOD3.
///
/// PLATEAU decides a model's LOD by how it was measured, and records that per
/// LOD in the quality attribute. The `attribute` here is the element holding
/// the LOD4 measurement code; `lod2` and `lod3` list the codes that send the
/// LOD4 content to each. A feature with no such code, or one in neither list,
/// takes `fallback` and is reported.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Lod4Policy {
    /// The element carrying the LOD4 measurement code, written with *output*
    /// prefixes as it stands after the rename pass. A profile that names none
    /// sends every LOD4 feature to `fallback`.
    pub attribute: Option<String>,
    /// Codes whose LOD4 becomes LOD2.
    pub lod2: Vec<String>,
    /// Codes whose LOD4 becomes LOD3.
    pub lod3: Vec<String>,
    pub fallback: Lod4Fallback,
}

impl Default for Lod4Policy {
    fn default() -> Self {
        Lod4Policy {
            attribute: None,
            lod2: Vec::new(),
            lod3: Vec::new(),
            fallback: Lod4Fallback::Lod3,
        }
    }
}

/// [`Lod4Policy`] with its attribute resolved to an expanded name.
#[derive(Debug, Clone)]
pub struct Lod4Rules {
    pub attribute: Option<Name>,
    pub lod2: Vec<String>,
    pub lod3: Vec<String>,
    pub fallback: Lod4Fallback,
}

/// How the input's `codelists/` is carried into the output.
///
/// The published i-UR 4.0 lists replace same-named input files, so the codes a
/// converted package is checked against are the 4.0 ones. `local` names the
/// lists a municipality authors itself — replacing those with a published
/// template would destroy real content — and `retarget`/`kept_codes` record
/// the places where the published set moved or dropped something.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CodelistsPolicy {
    /// File-name patterns (at most one `*`) for municipality-authored lists:
    /// the input's file always wins, even over a published file of the same
    /// name.
    pub local: Vec<String>,
    /// File-name patterns for lists whose values the conversion itself
    /// rewrites into the published codes: the published file always wins,
    /// or the rewritten values would not resolve.
    pub superseded: Vec<String>,
    /// Code-list file name -> its name in the published 4.0 set, rewritten in
    /// every `codeSpace` path. Only for renames whose codes carry over
    /// unchanged.
    pub retarget: IndexMap<String, String>,
    /// Codes the published lists no longer define, per file. The values are
    /// kept and reported, not mapped to something they do not mean.
    pub kept_codes: IndexMap<String, Vec<String>>,
}

impl CodelistsPolicy {
    pub fn is_local(&self, file_name: &str) -> bool {
        self.local.iter().any(|p| glob_match(p, file_name))
    }

    pub fn is_superseded(&self, file_name: &str) -> bool {
        self.superseded.iter().any(|p| glob_match(p, file_name))
    }
}

/// `pattern` equality, with one `*` matching any run of characters.
fn glob_match(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        Some((prefix, suffix)) => {
            name.len() >= prefix.len() + suffix.len()
                && name.starts_with(prefix)
                && name.ends_with(suffix)
        }
        None => pattern == name,
    }
}

/// A profile with every `prefix:local` resolved to an expanded name, ready to
/// apply.
#[derive(Debug, Clone)]
pub struct Rules {
    name: String,
    source: Provenance,
    target: Provenance,
    namespace_map: HashMap<String, String>,
    /// `None` means "drop this element".
    renames: HashMap<Name, Option<Name>>,
    order: HashMap<Name, Arc<Vec<Name>>>,
    output_namespaces: IndexMap<String, String>,
    prefixes: PrefixMap,
    schema_location: Option<String>,
    height: HeightDefaults,
    lod4: Lod4Rules,
    codelists: CodelistsPolicy,
    ade_hooks: HashMap<Name, Name>,
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
            let sources = parse_input_names(&rule.from, input)?;
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
            for from in sources {
                if let Some(previous) = renames.insert(from, to.clone()) {
                    return Err(Error::Profile(format!(
                        "duplicate rule for `{}` (previously mapped to {previous:?})",
                        rule.from
                    )));
                }
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

        let mut ade_hooks = HashMap::new();
        for (class, hook) in &profile.ade_hooks {
            ade_hooks.insert(parse_name(class, output)?, parse_name(hook, output)?);
        }

        let lod4 = Lod4Rules {
            attribute: profile
                .lod4
                .attribute
                .as_deref()
                .map(|a| parse_name(a, output))
                .transpose()?,
            lod2: profile.lod4.lod2.clone(),
            lod3: profile.lod4.lod3.clone(),
            fallback: profile.lod4.fallback,
        };
        if let Some(code) = lod4.lod2.iter().find(|c| lod4.lod3.contains(c)) {
            return Err(Error::Profile(format!(
                "[lod4] lists code `{code}` under both lod2 and lod3"
            )));
        }

        if let Some(pattern) = profile
            .codelists
            .local
            .iter()
            .find(|p| p.matches('*').count() > 1)
        {
            return Err(Error::Profile(format!(
                "[codelists] local pattern `{pattern}` has more than one `*`"
            )));
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
            source: profile.source.clone(),
            target: profile.target.clone(),
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
            lod4,
            codelists: profile.codelists.clone(),
            ade_hooks,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// What this profile accepts.
    pub fn source(&self) -> &Provenance {
        &self.source
    }

    /// What it produces.
    pub fn target(&self) -> &Provenance {
        &self.target
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

    /// Where LOD4 content goes. See [`Lod4Policy`].
    pub fn lod4(&self) -> &Lod4Rules {
        &self.lod4
    }

    /// The CityGML property an i-UR class hangs off in 3.0.
    ///
    /// CityGML 2.0 let an extension name its own property
    /// (`uro:buildingIDAttribute`); 3.0 declares one general hook per host class
    /// and the extension substitutes into it, so the wrapper is decided by the
    /// class inside rather than by the wrapper's own name.
    pub fn codelists(&self) -> &CodelistsPolicy {
        &self.codelists
    }

    pub fn ade_hook(&self, class: &Name) -> Option<&Name> {
        self.ade_hooks.get(class)
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

/// Resolves a `prefix:local` name on the *input* side, where one prefix may
/// stand for a family of namespace URIs. Returns one expanded name per URI, so
/// a single rule applies to every version in the family.
fn parse_input_names(
    text: &str,
    namespaces: &IndexMap<String, NamespaceValue>,
) -> Result<Vec<Name>> {
    let Some((prefix, local)) = text.split_once(':') else {
        return Ok(vec![Name::unqualified(text)]);
    };
    if local.is_empty() {
        return Err(Error::Profile(format!("`{text}` has no local name")));
    }
    let value = namespaces
        .get(prefix)
        .ok_or_else(|| Error::Profile(format!("`{text}` uses undeclared prefix `{prefix}`")))?;
    if value.uris().is_empty() {
        return Err(Error::Profile(format!(
            "`{text}` uses prefix `{prefix}`, which is bound to no namespace"
        )));
    }
    Ok(value
        .uris()
        .iter()
        .map(|uri| Name::qualified(uri.clone(), local))
        .collect())
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

    const URO: &str = "https://www.geospatial.jp/iur/uro";
    const URC_4: &str = "https://www.geospatial.jp/iur/urc/4.0";

    /// One rule, written once, has to match every i-UR minor version in
    /// circulation -- PLATEAU data is 3.0, 3.1 or 3.2 depending on its year.
    #[test]
    fn one_input_prefix_binds_a_whole_version_family() {
        let profile = r#"
name = "family"
[input.namespaces]
uro = [
  "https://www.geospatial.jp/iur/uro/3.0",
  "https://www.geospatial.jp/iur/uro/3.1",
  "https://www.geospatial.jp/iur/uro/3.2",
]
[output.namespaces]
urc = "https://www.geospatial.jp/iur/urc/4.0"
[[element]]
from = "uro:srcScaleLod0"
to = "urc:srcScaleLod0"
"#;
        let rules = Rules::from_toml(profile).unwrap();
        for version in ["3.0", "3.1", "3.2"] {
            let from = Name::qualified(format!("{URO}/{version}"), "srcScaleLod0");
            assert_eq!(
                rules.map_element(&from),
                Some(Name::qualified(URC_4, "srcScaleLod0")),
                "a rule must match uro {version}"
            );
        }
    }

    /// A prefix bound to a single URI keeps working.
    #[test]
    fn a_single_namespace_still_binds() {
        let profile = r#"
name = "single"
[input.namespaces]
bldg = "http://www.opengis.net/citygml/building/2.0"
[output.namespaces]
con = "http://www.opengis.net/citygml/construction/3.0"
[[element]]
from = "bldg:RoofSurface"
to = "con:RoofSurface"
"#;
        let rules = Rules::from_toml(profile).unwrap();
        let from = Name::qualified(ns::BUILDING_2, "RoofSurface");
        assert_eq!(
            rules.map_element(&from),
            Some(Name::qualified(ns::CONSTRUCTION_3, "RoofSurface"))
        );
    }

    #[test]
    fn a_prefix_bound_to_nothing_is_an_error() {
        let profile = r#"
name = "empty"
[input.namespaces]
uro = []
[output.namespaces]
urc = "https://www.geospatial.jp/iur/urc/4.0"
[[element]]
from = "uro:x"
to = "urc:x"
"#;
        assert!(Rules::from_toml(profile).is_err());
    }

    /// Every built-in profile declares what it accepts and what it produces, and
    /// covers all four i-UR modules -- a module missing from the table would be
    /// written unqualified rather than converted.
    #[test]
    fn every_profile_declares_its_source_and_target() {
        for (name, toml) in crate::PROFILES {
            let r = Rules::from_toml(toml).unwrap();
            assert_eq!(&r.name(), name);
            assert!(!r.source().label.is_empty(), "{name} has no source label");
            assert!(!r.target().label.is_empty(), "{name} has no target label");
            assert_eq!(r.source().iur.len(), 4, "{name}: uro, urf, urg, urt");
            assert_eq!(r.target().iur.len(), 5, "{name}: the four plus urc");
            assert_eq!(
                r.source().citygml.as_deref(),
                Some("http://www.opengis.net/citygml/2.0")
            );
        }
    }

    #[test]
    fn default_profile_compiles() {
        let r = rules();
        assert_eq!(r.name(), "iur-3.1-to-4.0");
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
    /// prefix on the output side and its elements are written unqualified. Real
    /// data is mostly 3.1 and 3.2. Each profile bumps its own i-UR minor and
    /// leaves the others alone: an unmapped namespace is what makes running the
    /// wrong profile visible instead of silently partial.
    #[test]
    fn a_profile_bumps_only_its_own_i_ur_minor() {
        for (name, toml) in crate::PROFILES {
            let r = Rules::from_toml(toml).unwrap();
            let own = name.trim_start_matches("iur-").trim_end_matches("-to-4.0");
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
                    let expected = if minor == own {
                        Name::qualified(
                            format!("https://www.geospatial.jp/iur/{module}/4.0"),
                            local,
                        )
                    } else {
                        from.clone()
                    };
                    assert_eq!(
                        r.map_element(&from).unwrap(),
                        expected,
                        "{name}: {module} {minor}"
                    );
                }
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
