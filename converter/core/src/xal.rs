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
//!
//! [`SLOTS`] is the whole mapping: one row per flat 3.0 slot, in the order
//! `AddressType` requires, saying which 2.0 container fills it, what its
//! `Type` attribute may say, and which name children it owns.

use crate::error::Result;
use crate::profile::Rules;
use crate::report::Warnings;
use crate::xml::{Element, Name, Node};

/// The namespace of the address fragments CityGML 2.0 data carries.
pub const XAL_2_0: &str = "urn:oasis:names:tc:ciq:xsdschema:xAL:2.0";

/// One flat xAL 3.0 slot under `Address`.
struct Slot {
    /// The 3.0 element, and the 2.0 container that fills it.
    to: &'static str,
    from: &'static str,
    /// The slot this one nests inside, as an index into [`SLOTS`]. A sub
    /// container is emitted after its parent's own names.
    parent: Option<usize>,
    /// The 3.0 `Type` list. Empty means open — any 2.0 `Type` is carried as
    /// written; `None` means the 3.0 element has no `Type` at all.
    types: Option<&'static [&'static str]>,
    /// A fixed `Type`, for a 2.0 container that became one case of a general
    /// 3.0 element.
    fixed_type: Option<&'static str>,
    /// The 2.0 name children this container owns, each with the `NameType`
    /// (or, in the postal slots, `Identifier` `Type`) it converts to. An empty
    /// value means the 3.0 lists cannot express it and the attribute is left off.
    names: &'static [(&'static str, &'static str)],
    /// Postal slots hold `Identifier`s with a `Type`; everything else
    /// `NameElement`s with a `NameType`.
    identifier: bool,
    /// How many names 3.0 allows here; the rest become free text.
    limit: usize,
}

impl Slot {
    const fn new(
        to: &'static str,
        from: &'static str,
        names: &'static [(&'static str, &'static str)],
    ) -> Slot {
        Slot {
            to,
            from,
            names,
            parent: None,
            types: None,
            fixed_type: None,
            identifier: false,
            limit: usize::MAX,
        }
    }

    const fn under(mut self, parent: usize) -> Slot {
        self.parent = Some(parent);
        self
    }

    const fn types(mut self, types: &'static [&'static str]) -> Slot {
        self.types = Some(types);
        self
    }

    const fn always(mut self, ty: &'static str) -> Slot {
        self.fixed_type = Some(ty);
        self
    }

    const fn identifiers(mut self) -> Slot {
        self.identifier = true;
        self
    }

    const fn at_most(mut self, limit: usize) -> Slot {
        self.limit = limit;
        self
    }
}

const ADMIN: usize = 1;
const LOCALITY: usize = 3;
const THOROUGHFARE: usize = 5;
const PREMISES: usize = 7;

/// The flat 3.0 slots, in the order `AddressType` requires. A sub container
/// follows its parent, which is what lets [`XalRewrite::assemble`] walk this
/// once and emit the address.
const SLOTS: &[Slot] = &[
    Slot::new(
        "Country",
        "Country",
        &[("CountryName", "Name"), ("CountryNameCode", "")],
    ),
    Slot::new(
        "AdministrativeArea",
        "AdministrativeArea",
        &[("AdministrativeAreaName", "Name")],
    )
    .types(&["City", "State", "Territory", "Province"]),
    Slot::new(
        "SubAdministrativeArea",
        "SubAdministrativeArea",
        &[("SubAdministrativeAreaName", "Name")],
    )
    .under(ADMIN)
    .types(&["County", "District", "Province", "Region"]),
    Slot::new("Locality", "Locality", &[("LocalityName", "Name")]).types(&[
        "Municipality",
        "PostTown",
        "Place",
        "Suburb",
        "Town",
        "Village",
        "Area",
        "Zone",
    ]),
    Slot::new(
        "SubLocality",
        "DependentLocality",
        &[
            ("DependentLocalityName", "Name"),
            ("DependentLocalityNumber", "Number"),
        ],
    )
    .under(LOCALITY)
    .types(&["Municipality", "Village"]),
    Slot::new("Thoroughfare", "Thoroughfare", THOROUGHFARE_NAMES).types(&[]),
    Slot::new(
        "SubThoroughfare",
        "DependentThoroughfare",
        THOROUGHFARE_NAMES,
    )
    .under(THOROUGHFARE)
    .at_most(5),
    Slot::new("Premises", "Premise", PREMISES_NAMES).types(PREMISES_TYPES),
    Slot::new("SubPremises", "SubPremise", PREMISES_NAMES).under(PREMISES),
    Slot::new(
        "PostCode",
        "PostalCode",
        &[
            ("PostalCodeNumber", ""),
            ("PostalCodeNumberExtension", "Extension"),
        ],
    )
    .identifiers(),
    Slot::new(
        "PostalDeliveryPoint",
        "PostBox",
        &[
            ("PostBoxNumber", "Number"),
            ("PostBoxNumberPrefix", "Prefix"),
            ("PostBoxNumberSuffix", "Suffix"),
            ("PostBoxNumberExtension", "Extension"),
        ],
    )
    .always("POBox")
    .identifiers(),
    Slot::new(
        "PostOffice",
        "PostOffice",
        &[("PostOfficeName", "Name"), ("PostOfficeNumber", "Number")],
    )
    .identifiers(),
];

/// A 2.0 thoroughfare and its dependent one own the same name children, as do
/// a premise and its sub premise.
const THOROUGHFARE_NAMES: &[(&str, &str)] = &[
    ("ThoroughfareName", "NameOnly"),
    ("ThoroughfareNumber", ""),
    ("ThoroughfareNumberPrefix", ""),
    ("ThoroughfareNumberSuffix", ""),
];
const PREMISES_NAMES: &[(&str, &str)] = &[
    ("PremiseName", "Name"),
    ("SubPremiseName", "Name"),
    ("PremiseNumber", ""),
    ("PremiseNumberPrefix", ""),
    ("PremiseNumberSuffix", ""),
    ("SubPremiseNumber", ""),
    ("SubPremiseNumberPrefix", ""),
    ("SubPremiseNumberSuffix", ""),
    ("BuildingName", ""),
];
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

/// 2.0 elements whose children are what matter; the element itself has no 3.0
/// counterpart and needs none.
const TRANSPARENT: &[&str] = &["AddressDetails", "AddressLines", "Address"];

/// What one slot captured while walking an address.
#[derive(Default, Clone)]
struct Filled {
    names: Vec<Element>,
    ty: Option<String>,
}

#[derive(Debug, Clone)]
pub struct XalRewrite {
    core: String,
    /// The output xAL namespace — xAL 3.0.
    xal: String,
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
        if !el.elements().any(|c| c.name.in_ns(XAL_2_0)) {
            // Already xAL 3.0, or empty: nothing to rebuild.
            return;
        }

        let mut slots = vec![Filled::default(); SLOTS.len()];
        let mut free = Vec::new();
        let mut foreign = Vec::new();
        for node in std::mem::take(&mut el.children) {
            match node {
                Node::Element(child) if child.name.in_ns(XAL_2_0) => {
                    self.walk(child, &mut slots, &mut free, warnings);
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
        el.push(self.assemble(slots, free, warnings));
        el.children.extend(foreign);
        warnings.add(
            "an xAL 2.0 address became an xAL 3.0 Address: CityGML 3.0 binds \
             core:xalAddress to OASIS xAL 3.0",
        );
    }

    /// Dispatches one xAL 2.0 element into the flat 3.0 slots.
    fn walk(
        &self,
        el: Element,
        slots: &mut [Filled],
        free: &mut Vec<Element>,
        warnings: &mut Warnings,
    ) {
        let local = el.name.local.as_str();
        if let Some(at) = SLOTS.iter().position(|s| s.from == local) {
            return self.container(el, at, slots, free, warnings);
        }
        if TRANSPARENT.contains(&local) {
            for child in into_elements(el) {
                self.walk(child, slots, free, warnings);
            }
        } else if local == "AddressLine" {
            free.push(self.element("AddressLine", el.text().trim()));
        } else {
            warnings.add(format!(
                "xAL 2.0 {local} has no xAL 3.0 counterpart; its text was kept \
                 as FreeTextAddress"
            ));
            for line in text_lines(&el) {
                free.push(self.element("AddressLine", &line));
            }
        }
    }

    /// Converts one 2.0 container: its name children fill the slot, its `Type`
    /// attribute is carried when the 3.0 list admits it, and anything else it
    /// holds is dispatched to its own slot.
    fn container(
        &self,
        el: Element,
        at: usize,
        slots: &mut [Filled],
        free: &mut Vec<Element>,
        warnings: &mut Warnings,
    ) {
        let slot = &SLOTS[at];
        if let Some(value) = el.attr(None, "Type").map(str::to_owned) {
            match carry_type(slot, &value) {
                Some(carried) => {
                    slots[at].ty.get_or_insert(carried);
                }
                None => warnings.add(format!(
                    "the Type=\"{value}\" attribute of xAL 2.0 {} has no xAL \
                     3.0 equivalent and was dropped",
                    el.name.local
                )),
            }
        }
        for child in into_elements(el) {
            match slot
                .names
                .iter()
                .find(|(from, _)| *from == child.name.local)
            {
                Some((_, name_type)) => {
                    let name = self.name_like(&child, slot, name_type, warnings);
                    slots[at].names.push(name);
                }
                None => self.walk(child, slots, free, warnings),
            }
        }
    }

    /// A 2.0 name element as a 3.0 `NameElement` or `Identifier`.
    fn name_like(
        &self,
        src: &Element,
        slot: &Slot,
        name_type: &str,
        warnings: &mut Warnings,
    ) -> Element {
        let (local, attr) = if slot.identifier {
            ("Identifier", "Type")
        } else {
            ("NameElement", "NameType")
        };
        let mut out = self.element(local, src.text().trim());
        if !name_type.is_empty() {
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

    /// Lays the slots out in the order `AddressType` requires.
    fn assemble(
        &self,
        mut slots: Vec<Filled>,
        mut free: Vec<Element>,
        warnings: &mut Warnings,
    ) -> Element {
        for (at, slot) in SLOTS.iter().enumerate() {
            let Some(parent) = slot.parent else { continue };
            // A container must hold at least one name, so a sub container
            // whose parent captured none donates its names to the parent.
            if slots[parent].names.is_empty() && !slots[at].names.is_empty() {
                let donated = std::mem::take(&mut slots[at]);
                slots[parent].names = donated.names;
                warnings.add(format!(
                    "an xAL 3.0 {} cannot stand without its parent's own name, \
                     so the names moved up one level",
                    slot.to
                ));
            }
            if slots[at].names.len() > slot.limit {
                warnings.add(format!(
                    "xAL 3.0 allows {} {} names; the rest were kept as \
                     FreeTextAddress",
                    slot.limit, slot.to
                ));
                for extra in slots[at].names.split_off(slot.limit) {
                    free.push(self.element("AddressLine", extra.text().trim()));
                }
            }
        }

        let mut address = Element::new(Name::qualified(&self.xal, "Address"));
        if !free.is_empty() {
            let mut text = Element::new(Name::qualified(&self.xal, "FreeTextAddress"));
            text.children = free.into_iter().map(Node::Element).collect();
            address.push(text);
        }
        for (at, slot) in SLOTS.iter().enumerate() {
            if slot.parent.is_some() || slots[at].names.is_empty() {
                continue;
            }
            let mut out = self.build(at, &mut slots);
            // Its sub container, if it captured anything, goes after its names.
            for (sub, _) in SLOTS
                .iter()
                .enumerate()
                .filter(|(_, s)| s.parent == Some(at))
            {
                if !slots[sub].names.is_empty() {
                    let nested = self.build(sub, &mut slots);
                    out.push(nested);
                }
            }
            address.push(out);
        }
        address
    }

    /// One slot as its 3.0 element, names and all.
    fn build(&self, at: usize, slots: &mut [Filled]) -> Element {
        let slot = &SLOTS[at];
        let filled = std::mem::take(&mut slots[at]);
        let mut out = Element::new(Name::qualified(&self.xal, slot.to));
        if let Some(value) = filled.ty.or_else(|| slot.fixed_type.map(str::to_owned)) {
            out.set_attr(Name::qualified(&self.xal, "Type"), value);
        }
        out.children = filled.names.into_iter().map(Node::Element).collect();
        out
    }
}

/// Carries a 2.0 container `Type` into the 3.0 `Type` list, matching
/// case-insensitively; an open list accepts the value as written.
fn carry_type(slot: &Slot, value: &str) -> Option<String> {
    match slot.types? {
        [] => Some(value.to_owned()),
        closed => closed
            .iter()
            .find(|allowed| allowed.eq_ignore_ascii_case(value))
            .map(|allowed| (*allowed).to_owned()),
    }
}

/// The xAL 2.0 element children of `el`, taking ownership.
fn into_elements(el: Element) -> impl Iterator<Item = Element> {
    el.children.into_iter().filter_map(|node| match node {
        Node::Element(child) if child.name.in_ns(XAL_2_0) => Some(child),
        _ => None,
    })
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
