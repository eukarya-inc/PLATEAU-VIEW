//! The address rewrite: xAL 2.0 content becomes an xAL 3.0 `Address`.
//!
//! CityGML 3.0 binds `core:xalAddress` to OASIS xAL 3.0, whose model differs
//! from the xAL 2.0 fragments CityGML 2.0 data carries in three ways:
//!
//! * the root is `Address`, not `AddressDetails`;
//! * containers no longer nest — `Country`, `AdministrativeArea`, `Locality`,
//!   `Thoroughfare`, `Premises` and the postal elements are laid out flat
//!   under `Address` in a fixed order, each holding its own sub container
//!   (`SubLocality`, `SubThoroughfare`, ...);
//! * every name becomes a `NameElement` (or an `Identifier` in the postal
//!   elements) whose `NameType`/`Type` attributes are closed enumerations,
//!   and free-form text lives in `FreeTextAddress/AddressLine`.
//!
//! The rewrite is structure-preserving where xAL 3.0 has a counterpart, and
//! lossless where it does not: an element with no counterpart keeps its text
//! as `FreeTextAddress/AddressLine` and is reported, so no address content is
//! silently dropped and the output always has somewhere valid to stand.
//! Content already in the xAL 3.0 namespace passes through untouched.
//!
//! Like the other passes this runs after the rename pass; `core` names are
//! CityGML 3.0. The xAL 2.0 namespace is deliberately absent from the
//! profile's `[namespace_map]` — a namespace bump cannot express this change,
//! so the whole subtree is rebuilt here instead.

use crate::error::Result;
use crate::profile::Rules;
use crate::report::Warnings;
use crate::xml::{Element, Name, Node};

/// The namespace of the address fragments CityGML 2.0 data carries.
pub const XAL_2_0: &str = "urn:oasis:names:tc:ciq:xsdschema:xAL:2.0";

/// xAL 3.0 closed `Type` lists, per container. A 2.0 `Type` value is carried
/// over case-insensitively when the list admits it and dropped (reported)
/// when it does not. Containers with an open list accept any value.
const LOCALITY_TYPES: &[&str] = &[
    "Municipality",
    "PostTown",
    "Place",
    "Suburb",
    "Town",
    "Village",
    "Area",
    "Zone",
];
const SUB_LOCALITY_TYPES: &[&str] = &["Municipality", "Village"];
const ADMIN_AREA_TYPES: &[&str] = &["City", "State", "Territory", "Province"];
const SUB_ADMIN_AREA_TYPES: &[&str] = &["County", "District", "Province", "Region"];
const PREMISES_TYPES: &[&str] = &[
    "Airport",
    "Area",
    "Building",
    "Farm",
    "Hospital",
    "House",
    "LandMark",
    "LargeMailUser",
    "Lot",
    "RailwayStation",
    "ShoppingComplex",
    "University",
    "Unit",
];

#[derive(Debug, Clone)]
pub struct XalRewrite {
    core: String,
    /// The output xAL namespace — xAL 3.0.
    xal: String,
}

/// One flat xAL 3.0 address under construction.
#[derive(Default)]
struct Builder {
    free: Vec<Element>,
    country: Vec<Element>,
    admin: Vec<Element>,
    admin_type: Option<String>,
    sub_admin: Vec<Element>,
    sub_admin_type: Option<String>,
    locality: Vec<Element>,
    locality_type: Option<String>,
    sub_locality: Vec<Element>,
    sub_locality_type: Option<String>,
    thoroughfare: Vec<Element>,
    thoroughfare_type: Option<String>,
    sub_thoroughfare: Vec<Element>,
    premises: Vec<Element>,
    premises_type: Option<String>,
    sub_premises: Vec<Element>,
    post_code: Vec<Element>,
    postal_delivery: Vec<Element>,
    post_office: Vec<Element>,
}

impl XalRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        Ok(XalRewrite {
            core: rules.output_ns("core")?.to_owned(),
            xal: rules.output_ns("xAL")?.to_owned(),
        })
    }

    /// Rewrites every `core:xalAddress` under `el` in place.
    pub fn apply(&self, el: &mut Element, warnings: &mut Warnings) {
        if el.name.in_ns(&self.core) && el.name.local == "xalAddress" {
            self.convert(el, warnings);
            return;
        }
        for child in el.elements_mut() {
            self.apply(child, warnings);
        }
    }

    fn convert(&self, el: &mut Element, warnings: &mut Warnings) {
        let has_2_0 = el.elements().any(|c| c.name.in_ns(XAL_2_0));
        if !has_2_0 {
            // Already xAL 3.0, or empty: nothing to rebuild.
            return;
        }

        let mut b = Builder::default();
        let mut foreign = Vec::new();
        for node in std::mem::take(&mut el.children) {
            match node {
                Node::Element(child) if child.name.in_ns(XAL_2_0) => {
                    self.walk(child, &mut b, warnings);
                }
                Node::Element(child) => {
                    warnings.add(format!(
                        "core:xalAddress held {} next to xAL content; it was \
                         kept, but xAL 3.0 allows only an Address here",
                        child.name.local
                    ));
                    foreign.push(Node::Element(child));
                }
                other => drop(other),
            }
        }
        el.children = Vec::new();
        el.push(self.assemble(b, warnings));
        el.children.extend(foreign);
        warnings.add(
            "an xAL 2.0 address became an xAL 3.0 Address: CityGML 3.0 binds \
             core:xalAddress to OASIS xAL 3.0",
        );
    }

    /// Dispatches one xAL 2.0 element into the flat 3.0 buckets.
    fn walk(&self, el: Element, b: &mut Builder, warnings: &mut Warnings) {
        match el.name.local.as_str() {
            // Transparent wrappers: only their children matter.
            "AddressDetails" | "AddressLines" | "Address" => {
                self.recurse(el, b, warnings);
            }
            "Country" => self.container(el, b, warnings, Bucket::Country),
            "AdministrativeArea" => self.container(el, b, warnings, Bucket::Admin),
            "SubAdministrativeArea" => self.container(el, b, warnings, Bucket::SubAdmin),
            "Locality" => self.container(el, b, warnings, Bucket::Locality),
            "DependentLocality" => self.container(el, b, warnings, Bucket::SubLocality),
            "Thoroughfare" => self.container(el, b, warnings, Bucket::Thoroughfare),
            "DependentThoroughfare" => self.container(el, b, warnings, Bucket::SubThoroughfare),
            "Premise" => self.container(el, b, warnings, Bucket::Premises),
            "SubPremise" => self.container(el, b, warnings, Bucket::SubPremises),
            "PostalCode" => self.container(el, b, warnings, Bucket::PostCode),
            "PostBox" => self.container(el, b, warnings, Bucket::PostalDelivery),
            "PostOffice" => self.container(el, b, warnings, Bucket::PostOffice),
            "AddressLine" => {
                let line = self.element("AddressLine", el.text().trim());
                b.free.push(line);
            }
            other => {
                warnings.add(format!(
                    "xAL 2.0 {other} has no xAL 3.0 counterpart; its text was \
                     kept as FreeTextAddress"
                ));
                for line in text_lines(&el) {
                    b.free.push(self.element("AddressLine", &line));
                }
            }
        }
    }

    fn recurse(&self, mut el: Element, b: &mut Builder, warnings: &mut Warnings) {
        for node in std::mem::take(&mut el.children) {
            if let Node::Element(child) = node {
                if child.name.in_ns(XAL_2_0) {
                    self.walk(child, b, warnings);
                }
            }
        }
    }

    /// Converts one 2.0 container: its name children fill the bucket, its
    /// `Type` attribute is carried when the 3.0 list admits it, and nested
    /// containers recurse to their own flat buckets.
    fn container(&self, mut el: Element, b: &mut Builder, warnings: &mut Warnings, into: Bucket) {
        if let Some(value) = el.attr(None, "Type").map(str::to_owned) {
            match into.carry_type(&value) {
                Some(carried) => into.set_type(b, carried),
                None => warnings.add(format!(
                    "the Type=\"{value}\" attribute of xAL 2.0 {} has no xAL \
                     3.0 equivalent and was dropped",
                    el.name.local
                )),
            }
        }
        for node in std::mem::take(&mut el.children) {
            let Node::Element(child) = node else { continue };
            if !child.name.in_ns(XAL_2_0) {
                continue;
            }
            match into.name_of(child.name.local.as_str()) {
                Some(name_type) => {
                    let converted = self.name_like(&child, into, name_type, warnings);
                    into.bucket(b).push(converted);
                }
                None => self.walk(child, b, warnings),
            }
        }
    }

    /// A 2.0 name element as a 3.0 `NameElement` or `Identifier`.
    fn name_like(
        &self,
        src: &Element,
        into: Bucket,
        name_type: Option<&str>,
        warnings: &mut Warnings,
    ) -> Element {
        let local = if into.uses_identifier() {
            "Identifier"
        } else {
            "NameElement"
        };
        let mut out = self.element(local, src.text().trim());
        let attr = if into.uses_identifier() {
            "Type"
        } else {
            "NameType"
        };
        if let Some(name_type) = name_type {
            out.set_attr(Name::qualified(&self.xal, attr), name_type);
        }
        for attr in &src.attrs {
            warnings.add(format!(
                "the {}=\"{}\" attribute of xAL 2.0 {} has no xAL 3.0 \
                 equivalent and was dropped",
                attr.name.local, attr.value, src.name.local
            ));
        }
        out
    }

    fn element(&self, local: &str, text: &str) -> Element {
        if text.is_empty() {
            Element::new(Name::qualified(&self.xal, local))
        } else {
            Element::with_text(Name::qualified(&self.xal, local), text)
        }
    }

    /// Lays the buckets out in the order `AddressType` requires.
    fn assemble(&self, mut b: Builder, warnings: &mut Warnings) -> Element {
        // A container must hold at least one name, so a sub container whose
        // parent captured none donates its names to the parent.
        for (parent, sub, what) in [
            (&mut b.admin, &mut b.sub_admin, "SubAdministrativeArea"),
            (&mut b.locality, &mut b.sub_locality, "SubLocality"),
            (
                &mut b.thoroughfare,
                &mut b.sub_thoroughfare,
                "SubThoroughfare",
            ),
            (&mut b.premises, &mut b.sub_premises, "SubPremises"),
        ] {
            if parent.is_empty() && !sub.is_empty() {
                parent.append(sub);
                warnings.add(format!(
                    "an xAL 3.0 {what} cannot stand without its parent's own \
                     name, so the names moved up one level"
                ));
            }
        }
        if b.sub_thoroughfare.len() > 5 {
            warnings.add(
                "xAL 3.0 allows five SubThoroughfare names; the rest were \
                 kept as FreeTextAddress",
            );
            for extra in b.sub_thoroughfare.split_off(5) {
                b.free
                    .push(self.element("AddressLine", extra.text().trim()));
            }
        }

        let mut address = Element::new(Name::qualified(&self.xal, "Address"));
        if !b.free.is_empty() {
            let mut free = Element::new(Name::qualified(&self.xal, "FreeTextAddress"));
            free.children = b.free.into_iter().map(Node::Element).collect();
            address.push(free);
        }
        if !b.country.is_empty() {
            address.push(self.filled("Country", b.country, None, vec![]));
        }
        if !b.admin.is_empty() {
            let sub = self.sub("SubAdministrativeArea", b.sub_admin, b.sub_admin_type);
            address.push(self.filled("AdministrativeArea", b.admin, b.admin_type, sub));
        }
        if !b.locality.is_empty() {
            let sub = self.sub("SubLocality", b.sub_locality, b.sub_locality_type);
            address.push(self.filled("Locality", b.locality, b.locality_type, sub));
        }
        if !b.thoroughfare.is_empty() {
            let sub = self.sub("SubThoroughfare", b.sub_thoroughfare, None);
            address.push(self.filled("Thoroughfare", b.thoroughfare, b.thoroughfare_type, sub));
        }
        if !b.premises.is_empty() {
            let sub = self.sub("SubPremises", b.sub_premises, None);
            address.push(self.filled("Premises", b.premises, b.premises_type, sub));
        }
        if !b.post_code.is_empty() {
            address.push(self.filled("PostCode", b.post_code, None, vec![]));
        }
        if !b.postal_delivery.is_empty() {
            address.push(self.filled(
                "PostalDeliveryPoint",
                b.postal_delivery,
                Some("POBox".to_owned()),
                vec![],
            ));
        }
        if !b.post_office.is_empty() {
            address.push(self.filled("PostOffice", b.post_office, None, vec![]));
        }
        address
    }

    /// A wrapped sub container, or nothing when it captured no names.
    fn sub(&self, local: &str, names: Vec<Element>, type_attr: Option<String>) -> Vec<Element> {
        if names.is_empty() {
            vec![]
        } else {
            vec![self.filled(local, names, type_attr, vec![])]
        }
    }

    fn filled(
        &self,
        local: &str,
        names: Vec<Element>,
        type_attr: Option<String>,
        tail: Vec<Element>,
    ) -> Element {
        let mut out = Element::new(Name::qualified(&self.xal, local));
        if let Some(value) = type_attr {
            out.set_attr(Name::qualified(&self.xal, "Type"), value);
        }
        out.children = names.into_iter().map(Node::Element).collect();
        for extra in tail {
            out.push(extra);
        }
        out
    }
}

/// Which flat 3.0 bucket a 2.0 container fills, and how its names convert.
#[derive(Clone, Copy, PartialEq)]
enum Bucket {
    Country,
    Admin,
    SubAdmin,
    Locality,
    SubLocality,
    Thoroughfare,
    SubThoroughfare,
    Premises,
    SubPremises,
    PostCode,
    PostalDelivery,
    PostOffice,
}

impl Bucket {
    fn bucket(self, b: &mut Builder) -> &mut Vec<Element> {
        match self {
            Bucket::Country => &mut b.country,
            Bucket::Admin => &mut b.admin,
            Bucket::SubAdmin => &mut b.sub_admin,
            Bucket::Locality => &mut b.locality,
            Bucket::SubLocality => &mut b.sub_locality,
            Bucket::Thoroughfare => &mut b.thoroughfare,
            Bucket::SubThoroughfare => &mut b.sub_thoroughfare,
            Bucket::Premises => &mut b.premises,
            Bucket::SubPremises => &mut b.sub_premises,
            Bucket::PostCode => &mut b.post_code,
            Bucket::PostalDelivery => &mut b.postal_delivery,
            Bucket::PostOffice => &mut b.post_office,
        }
    }

    /// The postal elements hold `Identifier`s; everything else `NameElement`s.
    fn uses_identifier(self) -> bool {
        matches!(
            self,
            Bucket::PostCode | Bucket::PostalDelivery | Bucket::PostOffice
        )
    }

    /// The 2.0 name children this container owns, with the `NameType` (or
    /// `Identifier` `Type`) each maps to — `None` marking values the closed
    /// 3.0 lists cannot express, which convert without the attribute.
    fn name_of(self, local: &str) -> Option<Option<&'static str>> {
        let name = |t| Some(Some(t));
        let bare = Some(None);
        match (self, local) {
            (Bucket::Country, "CountryName") => name("Name"),
            (Bucket::Country, "CountryNameCode") => bare,
            (Bucket::Admin, "AdministrativeAreaName") => name("Name"),
            (Bucket::SubAdmin, "SubAdministrativeAreaName") => name("Name"),
            (Bucket::Locality, "LocalityName") => name("Name"),
            (Bucket::SubLocality, "DependentLocalityName") => name("Name"),
            (Bucket::SubLocality, "DependentLocalityNumber") => name("Number"),
            (Bucket::Thoroughfare | Bucket::SubThoroughfare, "ThoroughfareName") => {
                name("NameOnly")
            }
            (
                Bucket::Thoroughfare | Bucket::SubThoroughfare,
                "ThoroughfareNumber" | "ThoroughfareNumberPrefix" | "ThoroughfareNumberSuffix",
            ) => bare,
            (Bucket::Premises | Bucket::SubPremises, "PremiseName" | "SubPremiseName") => {
                name("Name")
            }
            (
                Bucket::Premises | Bucket::SubPremises,
                "PremiseNumber"
                | "PremiseNumberPrefix"
                | "PremiseNumberSuffix"
                | "SubPremiseNumber"
                | "SubPremiseNumberPrefix"
                | "SubPremiseNumberSuffix"
                | "BuildingName",
            ) => bare,
            (Bucket::PostCode, "PostalCodeNumber") => bare,
            (Bucket::PostCode, "PostalCodeNumberExtension") => name("Extension"),
            (Bucket::PostalDelivery, "PostBoxNumber") => name("Number"),
            (Bucket::PostalDelivery, "PostBoxNumberPrefix") => name("Prefix"),
            (Bucket::PostalDelivery, "PostBoxNumberSuffix") => name("Suffix"),
            (Bucket::PostalDelivery, "PostBoxNumberExtension") => name("Extension"),
            (Bucket::PostOffice, "PostOfficeName") => name("Name"),
            (Bucket::PostOffice, "PostOfficeNumber") => name("Number"),
            _ => None,
        }
    }

    /// Carries a 2.0 container `Type` into the 3.0 `Type` list, matching
    /// case-insensitively; open lists accept the value as written.
    fn carry_type(self, value: &str) -> Option<String> {
        let closed: Option<&[&str]> = match self {
            Bucket::Locality => Some(LOCALITY_TYPES),
            Bucket::SubLocality => Some(SUB_LOCALITY_TYPES),
            Bucket::Admin => Some(ADMIN_AREA_TYPES),
            Bucket::SubAdmin => Some(SUB_ADMIN_AREA_TYPES),
            Bucket::Premises | Bucket::SubPremises => Some(PREMISES_TYPES),
            // ThoroughfareTypeList and the postal lists are open.
            Bucket::Thoroughfare | Bucket::SubThoroughfare | Bucket::PostOffice => None,
            Bucket::Country | Bucket::PostCode | Bucket::PostalDelivery => {
                return None;
            }
        };
        match closed {
            None => Some(value.to_owned()),
            Some(list) => list
                .iter()
                .find(|allowed| allowed.eq_ignore_ascii_case(value))
                .map(|allowed| (*allowed).to_owned()),
        }
    }

    fn set_type(self, b: &mut Builder, value: String) {
        let slot = match self {
            Bucket::Locality => &mut b.locality_type,
            Bucket::SubLocality => &mut b.sub_locality_type,
            Bucket::Admin => &mut b.admin_type,
            Bucket::SubAdmin => &mut b.sub_admin_type,
            Bucket::Thoroughfare => &mut b.thoroughfare_type,
            Bucket::Premises => &mut b.premises_type,
            _ => return,
        };
        slot.get_or_insert(value);
    }
}

/// Every non-empty text run under `el`, in document order.
fn text_lines(el: &Element) -> Vec<String> {
    let mut out = Vec::new();
    collect_text(el, &mut out);
    out
}

fn collect_text(el: &Element, out: &mut Vec<String>) {
    for node in &el.children {
        match node {
            Node::Text(t) | Node::CData(t) => {
                let t = t.trim();
                if !t.is_empty() {
                    out.push(t.to_owned());
                }
            }
            Node::Element(child) => collect_text(child, out),
            _ => {}
        }
    }
}
