//! The declarative half of the mapping, loaded from a TOML profile.

use std::collections::HashMap;
use std::sync::Arc;

use indexmap::IndexMap;
use serde::Deserialize;
use toml::{Table, Value};

use crate::error::{Error, Result};
use crate::xml::{Name, PrefixMap};

/// A conversion profile exactly as it appears on disk.
///
/// A profile may be written out in full, or assembled from *fragments* it
/// names in [`base`](Profile::base). See [`Profile::load`]. Every table has a
/// documented default, so a profile declares only what it changes.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Built-in fragments to fold in before this file's own rules, in the order
    /// named. See [`Profile::load`].
    #[serde(default)]
    pub base: Vec<String>,
    /// The CityGML and i-UR versions this profile accepts.
    #[serde(default)]
    pub source: Provenance,
    /// The versions it produces.
    #[serde(default)]
    pub target: Provenance,
    #[serde(default)]
    pub input: NamespaceSet,
    #[serde(default)]
    pub output: Output,
    /// Input namespace URI to output namespace URI, applied to any name
    /// without a more specific rule.
    #[serde(default)]
    pub namespace_map: IndexMap<String, String>,
    #[serde(default)]
    pub element: Vec<ElementRule>,
    #[serde(default)]
    pub order_group: Vec<OrderGroup>,
    /// i-UR class to the CityGML property that carries it. See
    /// [`Rules::ade_hook`].
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
    /// The child a data quality attribute must carry. See [`QualityPolicy`].
    #[serde(default)]
    pub quality: QualityPolicy,
}

impl Profile {
    /// Parses a profile, folding in the built-in fragments its `base` names.
    ///
    /// The fold is a plain TOML merge over the parsed documents. Fragments
    /// fold in first, in the order named, and the file's own rules last, so
    /// the fragments' [`namespace_map`](Profile::namespace_map) rows keep
    /// their relative position ahead of the overlay's. A key two files both
    /// declare is an error, not an override.
    ///
    /// A profile that names no `base` is complete in itself and is parsed as
    /// it was written.
    pub fn load(source: &str) -> Result<Profile> {
        let doc: Table = toml::from_str(source)?;
        let bases = base_names(&doc);
        if bases.is_empty() {
            return Ok(Value::Table(doc).try_into()?);
        }

        let label = doc
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("the profile")
            .to_owned();
        let mut merged = Table::new();
        let mut origin = HashMap::new();
        for name in &bases {
            let mut fragment: Table = toml::from_str(builtin_fragment(name)?)
                .map_err(|e| Error::Profile(format!("in base `{name}`: {e}")))?;
            if !base_names(&fragment).is_empty() {
                return Err(Error::Profile(format!(
                    "base `{name}` names a base of its own; fragments do not nest"
                )));
            }
            for key in ["name", "description", "base"] {
                fragment.remove(key);
            }
            merge(&mut merged, fragment, name, "", &mut origin)?;
        }
        merge(&mut merged, doc, &label, "", &mut origin)?;
        Ok(Value::Table(merged).try_into()?)
    }
}

/// The text of a built-in fragment, by the name a `base` entry gives.
///
/// Fragments are named rather than pathed, so a profile loaded from disk with
/// `--profile` stays one file.
fn builtin_fragment(name: &str) -> Result<&'static str> {
    crate::FRAGMENTS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, text)| *text)
        .ok_or_else(|| {
            let available: Vec<&str> = crate::FRAGMENTS.iter().map(|(n, _)| *n).collect();
            Error::Profile(format!(
                "unknown base `{name}`; available: {}",
                available.join(", ")
            ))
        })
}

/// The fragments a document names, read before it is deserialised. A malformed
/// `base` is left to serde, which reports it where every other field error is.
fn base_names(doc: &Table) -> Vec<String> {
    doc.get("base")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|i| i.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Folds one profile document into another, refusing anything both declare.
///
/// Tables merge key by key and arrays of rules accumulate. Everything else is
/// a leaf, and a leaf two files both write is an error. `origin` remembers
/// which file each one came from, so the error can name both.
fn merge(
    into: &mut Table,
    from: Table,
    label: &str,
    path: &str,
    origin: &mut HashMap<String, String>,
) -> Result<()> {
    for (key, value) in from {
        let at = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        match value {
            Value::Table(src) => {
                let Value::Table(dst) = into
                    .entry(key)
                    .or_insert_with(|| Value::Table(Table::new()))
                else {
                    return Err(clash(&at, origin, label));
                };
                merge(dst, src, label, &at, origin)?;
            }
            Value::Array(src) if src.iter().all(Value::is_table) => {
                let Value::Array(dst) = into.entry(key).or_insert_with(|| Value::Array(Vec::new()))
                else {
                    return Err(clash(&at, origin, label));
                };
                for row in src {
                    for subject in row_subjects(&at, &row) {
                        if let Some(previous) =
                            origin.insert(format!("{at}[{subject}]"), label.to_owned())
                        {
                            return Err(Error::Profile(format!(
                                "[[{at}]] rules on `{subject}` in both `{previous}` and `{label}`"
                            )));
                        }
                    }
                    dst.push(row);
                }
            }
            leaf => {
                if let Some(previous) = origin.insert(at.clone(), label.to_owned()) {
                    return Err(Error::Profile(format!(
                        "`{at}` is declared in both `{previous}` and `{label}`"
                    )));
                }
                into.insert(key, leaf);
            }
        }
    }
    Ok(())
}

/// What an array-of-tables row rules on, so that a row two files both write can
/// be named. An `[[order_group]]` claims every type it orders.
fn row_subjects(path: &str, row: &Value) -> Vec<String> {
    let field = match path {
        "element" => "from",
        "order_group" => "types",
        _ => return Vec::new(),
    };
    match row.get(field) {
        Some(Value::String(one)) => vec![one.clone()],
        Some(Value::Array(many)) => many
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

fn clash(at: &str, origin: &HashMap<String, String>, label: &str) -> Error {
    let previous = origin.get(at).map_or("another file", String::as_str);
    Error::Profile(format!(
        "`{at}` is declared in both `{previous}` and `{label}`"
    ))
}

/// One end of a conversion, meaning what a profile reads or what it writes.
///
/// `iur` is the discriminator. i-UR puts its minor version in the namespace
/// URI, so the namespaces a document declares say which version it is.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Provenance {
    /// Human-readable, such as `CityGML 2.0 + i-UR 3.1`. Shown by `inspect`.
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
/// Binding a prefix to a whole i-UR minor-version family keeps the element
/// table at one row per element, and makes a rule written against 3.0 match
/// 3.1 and 3.2 data as well.
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

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    #[serde(default)]
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
    /// Remove the element, and its subtree, instead of renaming it.
    #[serde(default)]
    pub drop: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderGroup {
    pub types: Vec<String>,
    pub children: Vec<String>,
}

/// The parts of a `con:Height` that CityGML 2.0 does not record, which the
/// converter supplies.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct HeightDefaults {
    /// `con:status`. An enumeration in 3.0, so it carries no `codeSpace`.
    pub status: String,
    /// `con:lowReference`, a code from the elevation-reference code list.
    pub low_reference: String,
    /// `con:highReference`, also a code from the elevation-reference code
    /// list.
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
/// `attribute` is the element holding the LOD4 measurement code. `lod2` and
/// `lod3` list the codes that send the LOD4 content to each. A feature with no
/// such code, or one in neither list, takes `fallback` and is reported.
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
/// The published i-UR 4.0 lists replace same-named input files. `local` names
/// the lists a municipality authors itself, which are kept as shipped, while
/// `retarget` and `kept_codes` record where the published set moved or dropped
/// something.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CodelistsPolicy {
    /// File-name patterns, each with at most one `*`, for
    /// municipality-authored lists. The input's file always wins, even over a
    /// published file of the same name.
    pub local: Vec<String>,
    /// File-name patterns for lists whose values the conversion itself
    /// rewrites into the published codes. The published file always wins.
    pub superseded: Vec<String>,
    /// Code-list file name to its name in the published 4.0 set, rewritten in
    /// every `codeSpace` path. Only for renames whose codes carry over
    /// unchanged.
    pub retarget: IndexMap<String, String>,
    /// Codes the published lists no longer define, per file. The values are
    /// kept and reported rather than mapped.
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

/// The child i-UR requires on a data quality attribute, and the value to
/// supply when the source records none.
///
/// `classes` are the concrete classes carrying the requirement, `child` the
/// element to supply, `value` and `code_space` its content, and `after` the
/// children `child` must follow, since the type is a sequence.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct QualityPolicy {
    pub classes: Vec<String>,
    pub child: Option<String>,
    pub value: String,
    pub code_space: Option<String>,
    pub after: Vec<String>,
}

/// [`QualityPolicy`] with its names resolved.
#[derive(Debug, Clone, Default)]
pub struct QualityRules {
    pub classes: Vec<Name>,
    pub child: Option<Name>,
    pub value: String,
    pub code_space: Option<String>,
    pub after: Vec<Name>,
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
    quality: QualityRules,
}

impl Rules {
    pub fn from_toml(source: &str) -> Result<Self> {
        Self::compile(&Profile::load(source)?)
    }

    pub fn compile(profile: &Profile) -> Result<Self> {
        let input = &profile.input.namespaces;
        let output = &profile.output.namespaces;
        let policy = &profile.lod4;
        let codelists = profile.codelists.clone();

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

        let quality = QualityRules {
            classes: profile
                .quality
                .classes
                .iter()
                .map(|c| parse_name(c, output))
                .collect::<Result<_>>()?,
            child: profile
                .quality
                .child
                .as_deref()
                .map(|c| parse_name(c, output))
                .transpose()?,
            value: profile.quality.value.clone(),
            code_space: profile.quality.code_space.clone(),
            after: profile
                .quality
                .after
                .iter()
                .map(|a| parse_name(a, output))
                .collect::<Result<_>>()?,
        };
        if quality.child.is_some() && quality.value.is_empty() {
            return Err(Error::Profile(
                "[quality] names a child to supply but no value".into(),
            ));
        }

        let mut ade_hooks = HashMap::new();
        for (class, hook) in &profile.ade_hooks {
            ade_hooks.insert(parse_name(class, output)?, parse_name(hook, output)?);
        }

        let lod4 = Lod4Rules {
            attribute: policy
                .attribute
                .as_deref()
                .map(|a| parse_name(a, output))
                .transpose()?,
            lod2: policy.lod2.clone(),
            lod3: policy.lod3.clone(),
            fallback: policy.fallback,
        };
        if let Some(code) = lod4.lod2.iter().find(|c| lod4.lod3.contains(c)) {
            return Err(Error::Profile(format!(
                "[lod4] lists code `{code}` under both lod2 and lod3"
            )));
        }

        if let Some(pattern) = codelists.local.iter().find(|p| p.matches('*').count() > 1) {
            return Err(Error::Profile(format!(
                "[codelists] local pattern `{pattern}` has more than one `*`"
            )));
        }

        // These become the xmlns declarations on the output root, in this
        // order. Sorting keeps the bytes independent of how the profile was
        // split into fragments.
        let mut output_namespaces = output.clone();
        output_namespaces.sort_keys();
        let mut prefixes = PrefixMap::new();
        for (prefix, uri) in &output_namespaces {
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
            output_namespaces,
            prefixes,
            schema_location,
            height: profile.height.clone(),
            lod4,
            codelists,
            ade_hooks,
            quality,
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

    /// The output name for an attribute. Element rules do not apply, only the
    /// namespace bump.
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

    /// The CityGML property an i-UR class hangs off in 3.0, keyed by the
    /// class rather than by the 2.0 wrapper's own name.
    pub fn codelists(&self) -> &CodelistsPolicy {
        &self.codelists
    }

    pub fn ade_hook(&self, class: &Name) -> Option<&Name> {
        self.ade_hooks.get(class)
    }

    /// The child a data quality attribute must carry. See [`QualityPolicy`].
    pub fn quality(&self) -> &QualityRules {
        &self.quality
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

    /// One rule, written once, matches every i-UR minor version the prefix
    /// stands for.
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

    /// Every built-in profile declares what it accepts and what it produces,
    /// and covers all four i-UR modules.
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
    fn bumps_namespaces_by_default() {
        let r = rules();
        // No rule for CityModel, so only its namespace changes.
        let out = r
            .map_element(&Name::qualified(ns::CITYGML_2, "CityModel"))
            .unwrap();
        assert_eq!(out, Name::qualified(ns::CITYGML_3, "CityModel"));
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

    /// Each profile bumps its own i-UR minor and leaves the others unmapped,
    /// so running the wrong profile shows up as elements written
    /// unqualified.
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

    /// A profile written out in full still loads, with no `base` named.
    #[test]
    fn a_profile_that_names_no_base_is_complete_in_itself() {
        let rules = Rules::from_toml(
            r#"
            name = "standalone"
            [input.namespaces]
            core = "urn:old"
            [output.namespaces]
            core = "urn:new"
            "#,
        )
        .unwrap();
        assert_eq!(rules.name(), "standalone");
    }

    #[test]
    fn every_shipped_profile_is_assembled_from_fragments() {
        for (name, toml) in crate::PROFILES {
            let profile = Profile::load(toml).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(
                !profile.base.is_empty(),
                "{name} declares no base; the shared halves belong in a fragment"
            );
        }
    }

    /// A fragment is half a mapping and is refused where a profile is
    /// expected.
    #[test]
    fn a_fragment_is_not_a_profile_on_its_own() {
        for (name, toml) in crate::FRAGMENTS {
            let profile = Profile::load(toml).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(profile.source.iur.is_empty(), "{name} declares a [source]");
        }
    }

    #[test]
    fn an_unknown_base_lists_the_ones_there_are() {
        let err = Rules::from_toml(
            r#"
            name = "t"
            base = ["citygml-9.9"]
            "#,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("unknown base `citygml-9.9`"),
            "{err}"
        );
        assert!(err.to_string().contains("citygml-2.0-to-3.0"), "{err}");
    }

    /// A key declared in both a fragment and the profile is an error naming
    /// both files.
    #[test]
    fn anything_declared_in_two_files_is_refused() {
        let cases = [
            "[[element]]\nfrom = \"bldg:consistsOfBuildingPart\"\ndrop = true",
            "[namespace_map]\n\"http://www.opengis.net/citygml/2.0\" = \"urn:elsewhere\"",
            "[height]\nstatus = \"estimated\"",
        ];
        for case in cases {
            let err = Rules::from_toml(&format!(
                "name = \"clash\"\nbase = [\"citygml-2.0-to-3.0\"]\n{case}"
            ))
            .unwrap_err();
            let text = err.to_string();
            assert!(text.contains("citygml-2.0-to-3.0"), "{case}: {text}");
            assert!(text.contains("clash"), "{case}: {text}");
        }
    }

    /// The order the xmlns declarations are emitted in does not depend on how
    /// the profile was split into fragments.
    #[test]
    fn output_prefixes_are_ordered_independently_of_the_fragments() {
        let rules = rules();
        let prefixes: Vec<&str> = rules.output_namespaces.keys().map(|p| p.as_str()).collect();
        let mut sorted = prefixes.clone();
        sorted.sort_unstable();
        assert_eq!(prefixes, sorted);
    }
}
