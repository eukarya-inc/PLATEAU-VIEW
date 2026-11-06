//! Building floors, which CityGML 2.0 records as groups and CityGML 3.0 as
//! storeys.
//!
//! 2.0 has no floor feature, so a floor is a `grp:CityObjectGroup` that is its
//! own top-level member. It names the floor in `gml:name`, points at its
//! building through `grp:parent` and at its rooms through `grp:groupMember`.
//! 3.0 has `bldg:Storey`, which sits *inside* the building as a
//! `bldg:buildingSubdivision` and references its rooms through
//! `bldg:buildingRoom`. A subdivision is a composition, so the storey cannot
//! stay a separate member that the building links to.
//!
//! A group and its building are separate members in either order, so
//! [`StoreyRewrite::scan`] reads the document once before conversion. That
//! pass keeps only the groups, which carry no geometry, and the `gml:id`s of
//! buildings and rooms. Conversion then drops each group that became a storey
//! ([`StoreyRewrite::is_floor`]) and nests the storey in its building
//! ([`StoreyRewrite::attach`]).
//!
//! A group is a floor when `grp:parent` names a building, building part or
//! underground building in the same document and it carries no
//! `grp:function` other than the storey code. Any other group stays a group.
//!
//! Like [`crate::bldg`] this speaks CityGML 3.0 names. The scan sees 2.0 input
//! and maps every name through the profile before comparing it.

use std::collections::{HashMap, HashSet};

use crate::error::Result;
use crate::profile::Rules;
use crate::report::Warnings;
use crate::transform;
use crate::xml::{Element, Name, Node, Reader};

/// The `grp:function` code that marks a group as a storey. A group with any
/// other function, such as a zone, is not a floor.
const STOREY_FUNCTION: &str = "1030";

/// The storeys found ahead of conversion, waiting for their building.
#[derive(Debug, Default)]
pub struct Floors {
    /// Storeys by the `gml:id` of the building they belong in, in document
    /// order.
    by_host: HashMap<String, Vec<Element>>,
    /// `gml:id`s of the groups that became storeys.
    groups: HashSet<String>,
}

impl Floors {
    /// True when no storey is still waiting for its building.
    pub fn is_settled(&self) -> bool {
        self.by_host.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct StoreyRewrite {
    rules: Rules,
    grp: String,
    bldg: String,
    gml: String,
    xlink: String,
    /// The 3.0 types a floor can belong to.
    hosts: Vec<Name>,
    room: Name,
    storey: Name,
}

impl StoreyRewrite {
    pub fn new(rules: &Rules) -> Result<Self> {
        let bldg = rules.output_ns("bldg")?.to_owned();
        let mut hosts = vec![
            Name::qualified(&bldg, "Building"),
            Name::qualified(&bldg, "BuildingPart"),
        ];
        if let Ok(uro) = rules.output_ns("uro") {
            hosts.push(Name::qualified(uro, "UndergroundBuilding"));
        }
        Ok(StoreyRewrite {
            rules: rules.clone(),
            grp: rules.output_ns("grp")?.to_owned(),
            room: Name::qualified(&bldg, "BuildingRoom"),
            storey: Name::qualified(&bldg, "Storey"),
            bldg,
            gml: rules.output_ns("gml")?.to_owned(),
            xlink: rules.output_ns("xlink")?.to_owned(),
            hosts,
        })
    }

    /// Reads `source` for floor groups and turns each into the storey that
    /// will be nested in its building.
    pub fn scan(&self, label: &str, source: &str, warnings: &mut Warnings) -> Result<Floors> {
        let mut floors = Floors::default();
        if !source.contains("CityObjectGroup") {
            return Ok(floors);
        }

        let group = Name::qualified(&self.grp, "CityObjectGroup");
        let mut hosts = HashSet::new();
        let mut rooms = HashSet::new();
        let groups = Reader::new(label, source).scan(
            |el| {
                if !el.name.local.starts_with(char::is_uppercase) {
                    return;
                }
                let Some(id) = self.raw_id(el) else {
                    return;
                };
                let Some(name) = self.rules.map_element(&el.name) else {
                    return;
                };
                if self.hosts.contains(&name) {
                    hosts.insert(id.to_owned());
                } else if name == self.room {
                    rooms.insert(id.to_owned());
                }
            },
            |el| self.rules.map_element(&el.name).as_ref() == Some(&group),
        )?;

        for raw in groups {
            let Some(group) = transform::rename(&self.rules, raw) else {
                continue;
            };
            if let Some((host, storey)) = self.storey(group, &hosts, &rooms, warnings) {
                if let Some(id) = storey.attr(Some(&self.gml), "id") {
                    floors.groups.insert(id.to_owned());
                }
                floors.by_host.entry(host).or_default().push(storey);
            }
        }
        Ok(floors)
    }

    /// True when `member` is a group [`Self::scan`] turned into a storey, and
    /// so must not be written where it stands.
    pub fn is_floor(&self, member: &Element, floors: &Floors) -> bool {
        if floors.groups.is_empty() {
            return false;
        }
        member
            .elements()
            .next()
            .filter(|f| f.is(&self.grp, "CityObjectGroup"))
            .and_then(|f| f.attr(Some(&self.gml), "id"))
            .is_some_and(|id| floors.groups.contains(id))
    }

    /// Reports an interior model under `member` that carries no `bldg:Storey`,
    /// which CityGML 3.0 requires in every interior LOD. `member` is a
    /// top-level member whose interior model was folded to an LOD.
    pub fn warn_missing(&self, member: &Element, warnings: &mut Warnings) {
        if !has_descendant(member, &self.room) || has_descendant(member, &self.storey) {
            return;
        }
        let feature = member
            .elements()
            .next()
            .filter(|_| member.name.local.starts_with(char::is_lowercase))
            .map(|f| &f.name)
            .unwrap_or(&member.name);
        warnings.add(format!(
            "{}'s interior model has no bldg:Storey, which CityGML 3.0 requires \
             in every interior LOD; the input has no floor (a grp:CityObjectGroup \
             whose grp:parent is the building) to make one from, so none were \
             created",
            self.rules.display_name(feature)
        ));
    }

    /// Nests every storey waiting for a building under `el` in that building,
    /// as a `bldg:buildingSubdivision`.
    pub fn attach(&self, el: &mut Element, floors: &mut Floors, warnings: &mut Warnings) {
        if floors.by_host.is_empty() {
            return;
        }
        for child in el.elements_mut() {
            self.attach(child, floors, warnings);
        }
        if !self.hosts.contains(&el.name) {
            return;
        }
        let Some(storeys) = el
            .attr(Some(&self.gml), "id")
            .and_then(|id| floors.by_host.remove(id))
        else {
            return;
        };
        for storey in storeys {
            warnings.add(format!(
                "a floor's grp:CityObjectGroup became a bldg:Storey inside its {}: \
                 CityGML 3.0 has a floor feature, and it is part of the building \
                 rather than a separate member",
                self.rules.display_name(&el.name)
            ));
            let mut subdivision = Element::new(Name::qualified(&self.bldg, "buildingSubdivision"));
            subdivision.push(storey);
            el.push(subdivision);
        }
    }

    /// The storey a renamed group becomes, with the `gml:id` of the building
    /// it belongs in, or `None` when the group is not a floor.
    fn storey(
        &self,
        group: Element,
        hosts: &HashSet<String>,
        rooms: &HashSet<String>,
        warnings: &mut Warnings,
    ) -> Option<(String, Element)> {
        if let Some(function) = group
            .elements()
            .filter(|c| c.is(&self.grp, "function"))
            .map(|c| c.text().trim().to_owned())
            .find(|code| code != STOREY_FUNCTION)
        {
            warnings.add(format!(
                "a grp:CityObjectGroup with grp:function {function} is not a floor, \
                 so it was not made a bldg:Storey and keeps its CityGML 2.0 \
                 grp:groupMember and grp:parent"
            ));
            return None;
        }

        let parent = group
            .child(&self.grp, "parent")
            .and_then(|p| p.attr(Some(&self.xlink), "href"))
            .and_then(|href| href.strip_prefix('#'))
            .filter(|id| hosts.contains(*id))
            .map(str::to_owned);
        let Some(parent) = parent else {
            warnings.add(
                "a grp:CityObjectGroup has no grp:parent naming a building in the same \
                 document, so it is not a floor; it was not made a bldg:Storey and \
                 keeps its CityGML 2.0 grp:groupMember and grp:parent",
            );
            return None;
        };

        let mut storey = Element::new(Name::qualified(&self.bldg, "Storey"));
        storey.attrs = group.attrs;
        for child in group.children {
            let Node::Element(child) = child else {
                storey.children.push(child);
                continue;
            };
            if !child.name.in_ns(&self.grp) {
                storey.push(child);
                continue;
            }
            match child.name.local.as_str() {
                "groupMember" => {
                    if let Some(room) = self.room_reference(&child, rooms) {
                        storey.push(room);
                    } else {
                        warnings.add(
                            "a floor's grp:groupMember that does not reference a room \
                             in the same document was dropped: bldg:Storey references \
                             its rooms only",
                        );
                    }
                }
                "parent" => {}
                local @ ("class" | "function" | "usage") => warnings.add(format!(
                    "grp:{local} {} was dropped from a floor: it classified the \
                     CityGML 2.0 group, and bldg:Storey records that it is a floor \
                     by its type",
                    child.text().trim()
                )),
                _ => storey.push(child),
            }
        }
        Some((parent, storey))
    }

    /// A `bldg:buildingRoom` reference for a `grp:groupMember` that links to
    /// a room by `xlink:href`. Only the xlink attributes carry over.
    fn room_reference(&self, member: &Element, rooms: &HashSet<String>) -> Option<Element> {
        if member.has_element_children() {
            return None;
        }
        member
            .attr(Some(&self.xlink), "href")
            .and_then(|href| href.strip_prefix('#'))
            .filter(|id| rooms.contains(*id))?;
        let mut room = Element::new(Name::qualified(&self.bldg, "buildingRoom"));
        room.attrs = member
            .attrs
            .iter()
            .filter(|a| a.name.in_ns(&self.xlink))
            .cloned()
            .collect();
        Some(room)
    }

    /// The `gml:id` of an element read straight from the 2.0 input.
    fn raw_id<'a>(&self, el: &'a Element) -> Option<&'a str> {
        el.attrs
            .iter()
            .find(|a| a.name.local == "id" && self.rules.map_attribute(&a.name).in_ns(&self.gml))
            .map(|a| a.value.as_str())
    }
}

/// True when an element named `name` sits anywhere under `el`.
fn has_descendant(el: &Element, name: &Name) -> bool {
    el.elements()
        .any(|child| child.name == *name || has_descendant(child, name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_PROFILE;
    use crate::xml::ns;

    const GRP3: &str = "http://www.opengis.net/citygml/cityobjectgroup/3.0";

    fn rewrite() -> StoreyRewrite {
        StoreyRewrite::new(&Rules::from_toml(DEFAULT_PROFILE).unwrap()).unwrap()
    }

    /// A 2.0 document holding `members`, each wrapped in a cityObjectMember.
    fn document(members: &[&str]) -> String {
        let body: String = members
            .iter()
            .map(|m| format!("<core:cityObjectMember>{m}</core:cityObjectMember>"))
            .collect();
        format!(
            r#"<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
                xmlns:bldg="http://www.opengis.net/citygml/building/2.0"
                xmlns:grp="http://www.opengis.net/citygml/cityobjectgroup/2.0"
                xmlns:gml="http://www.opengis.net/gml"
                xmlns:xlink="http://www.w3.org/1999/xlink">{body}</core:CityModel>"#
        )
    }

    const BUILDING: &str = r##"<bldg:Building gml:id="b1">
        <bldg:interiorRoom><bldg:Room gml:id="r1"/></bldg:interiorRoom>
        <bldg:interiorRoom><bldg:Room gml:id="r2"/></bldg:interiorRoom>
        <bldg:interiorBuildingInstallation>
            <bldg:IntBuildingInstallation gml:id="i1"/>
        </bldg:interiorBuildingInstallation>
    </bldg:Building>"##;

    fn floor(id: &str, extra: &str) -> String {
        format!(
            r##"<grp:CityObjectGroup gml:id="{id}">
                <gml:name>1F</gml:name>
                {extra}
                <grp:groupMember xlink:href="#r1"/>
                <grp:groupMember xlink:href="#r2"/>
                <grp:parent xlink:href="#b1"/>
            </grp:CityObjectGroup>"##
        )
    }

    fn scan(members: &[&str]) -> (Floors, Warnings) {
        let mut warnings = Warnings::new();
        let floors = rewrite()
            .scan("test", &document(members), &mut warnings)
            .unwrap();
        (floors, warnings)
    }

    fn building() -> Element {
        let mut b = Element::new(Name::qualified(ns::BUILDING_3, "Building"));
        b.set_attr(Name::qualified(ns::GML_32, "id"), "b1");
        b
    }

    fn mentions(warnings: &Warnings, text: &str) -> bool {
        warnings.iter().any(|(m, _)| m.contains(text))
    }

    #[test]
    fn a_floor_group_becomes_a_storey_inside_its_building_whichever_comes_first() {
        let group = floor("g1", "");
        for members in [[group.as_str(), BUILDING], [BUILDING, group.as_str()]] {
            let (mut floors, mut warnings) = scan(&members);
            let mut b = building();
            rewrite().attach(&mut b, &mut floors, &mut warnings);
            assert!(floors.is_settled());

            let storey = b
                .child(ns::BUILDING_3, "buildingSubdivision")
                .and_then(|s| s.child(ns::BUILDING_3, "Storey"))
                .expect("a storey nested in the building");
            assert_eq!(storey.attr(Some(ns::GML_32), "id"), Some("g1"));
            assert_eq!(storey.child(ns::GML_32, "name").unwrap().text(), "1F");
            let rooms: Vec<_> = storey
                .elements()
                .filter(|e| e.is(ns::BUILDING_3, "buildingRoom"))
                .map(|e| e.attr(Some(ns::XLINK), "href").unwrap())
                .collect();
            assert_eq!(rooms, ["#r1", "#r2"]);
            assert!(storey.elements().all(|e| !e.name.in_ns(GRP3)));
            assert!(mentions(
                &warnings,
                "became a bldg:Storey inside its bldg:Building"
            ));
        }
    }

    #[test]
    fn the_group_that_became_a_storey_is_not_written_where_it_stood() {
        let (floors, _) = scan(&[&floor("g1", ""), BUILDING]);
        let mut group = Element::new(Name::qualified(GRP3, "CityObjectGroup"));
        group.set_attr(Name::qualified(ns::GML_32, "id"), "g1");
        let mut member = Element::new(Name::qualified(ns::CITYGML_3, "cityObjectMember"));
        member.push(group);
        assert!(rewrite().is_floor(&member, &floors));
    }

    #[test]
    fn the_storey_function_code_is_dropped_and_reported() {
        let extra = r#"<grp:function codeSpace="../../codelists/CityObjectGroup_function.xml">1030</grp:function>"#;
        let (mut floors, mut warnings) = scan(&[&floor("g1", extra), BUILDING]);
        let mut b = building();
        rewrite().attach(&mut b, &mut floors, &mut warnings);
        assert!(b.child(ns::BUILDING_3, "buildingSubdivision").is_some());
        assert!(mentions(&warnings, "grp:function 1030 was dropped"));
    }

    #[test]
    fn a_zone_is_not_a_floor() {
        let extra = "<grp:function>1040</grp:function>";
        let (floors, warnings) = scan(&[&floor("g1", extra), BUILDING]);
        assert!(floors.is_settled() && floors.groups.is_empty());
        assert!(mentions(&warnings, "grp:function 1040 is not a floor"));
    }

    #[test]
    fn a_group_whose_parent_is_not_a_building_in_the_document_is_not_a_floor() {
        let (floors, warnings) = scan(&[&floor("g1", "")]);
        assert!(floors.groups.is_empty());
        assert!(mentions(&warnings, "no grp:parent naming a building"));
    }

    #[test]
    fn a_member_that_is_not_a_room_is_dropped_and_reported() {
        let extra = r##"<grp:groupMember xlink:href="#i1"/>"##;
        let (mut floors, mut warnings) = scan(&[&floor("g1", extra), BUILDING]);
        let mut b = building();
        rewrite().attach(&mut b, &mut floors, &mut warnings);
        let storey = b
            .child(ns::BUILDING_3, "buildingSubdivision")
            .and_then(|s| s.child(ns::BUILDING_3, "Storey"))
            .unwrap();
        assert_eq!(
            storey
                .elements()
                .filter(|e| e.is(ns::BUILDING_3, "buildingRoom"))
                .count(),
            2
        );
        assert!(mentions(&warnings, "does not reference a room"));
    }

    #[test]
    fn a_document_without_groups_is_not_scanned() {
        let (floors, warnings) = scan(&[BUILDING]);
        assert!(floors.is_settled() && warnings.is_empty());
    }
}
