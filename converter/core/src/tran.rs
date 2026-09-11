//! Structural rewrites for the Transportation module that a rename table
//! cannot express.
//!
//! Like [`crate::bldg`] this runs *after* [`crate::transform::rename`], so
//! every name here is CityGML 3.0. It runs before [`crate::iur`], so the data
//! quality descriptors it renumbers are in place when that pass supplies the
//! one i-UR 4.0 requires.
//!
//! CityGML 2.0 gave a transportation object its own geometry and hung
//! surfaces off it directly. CityGML 3.0 gives the object no geometry and
//! reaches every surface through a space:
//!
//! ```text
//! 2.0   <tran:Road>
//!         <tran:lod1MultiSurface>…</…>              the full road width
//!         <tran:trafficArea><tran:TrafficArea>
//!           <tran:lod2MultiSurface>…</…>
//!         </…></…>
//!
//! 3.0   <tran:Road>
//!         <tran:trafficSpace><tran:TrafficSpace>
//!           <tran:granularity>way</…>
//!           <core:boundary><tran:TrafficArea>
//!             <core:lod1MultiSurface>…</…>
//!           </…></…>
//!         </…></…>
//! ```
//!
//! One pass covers Road, Track, Square, Railway and Waterway, since the module
//! gives them one shape. Per feature it does the following:
//!
//! * wraps every inline traffic and auxiliary traffic area in a space, whose
//!   `gml:id` is the area's own id with `_space` appended, so a document in
//!   another file can name the space from the area it already references;
//! * rewrites an area held by reference into a reference to that space;
//! * moves each area's geometry down one LOD, since 3.0's LOD1 has no height
//!   and its LOD2 does, which is where 2.0 drew the line between LOD2 and
//!   LOD3;
//! * keeps the feature's own full-width surface as a minted space and area at
//!   LOD1 when no area carries an LOD2 surface, and drops it otherwise;
//! * drops the feature's own LOD2 and LOD3 surfaces, which 2.0 defined as the
//!   aggregate of the areas' surfaces;
//! * renumbers the LOD-indexed data quality descriptors the same way the
//!   geometry moved, and rewrites `lodType` through the profile's map;
//! * when the run asks for it, extrudes each space's LOD1 area into a
//!   `core:lod1Solid` by the clearance height that [`Clearance`] resolves.
//!
//! Granularity, the full-width function code, the `lodType` map and the
//! per-type clearance heights come from the profile's `[tran]` table.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::error::Result;
use crate::extrude::extrude;
use crate::profile::{Rules, TranRules};
use crate::report::Warnings;
use crate::transform::{IdGen, id_seed};
use crate::xml::{Element, Name, Node, ns};

/// The transportation feature types this pass rewrites.
const FEATURES: &[&str] = &["Road", "Track", "Square", "Railway", "Waterway"];

/// The 2.0 properties that held an area, and the 3.0 property and space each
/// becomes.
const AREA_PROPERTIES: &[(&str, &str, &str)] = &[
    ("trafficArea", "trafficSpace", "TrafficSpace"),
    (
        "auxiliaryTrafficArea",
        "auxiliaryTrafficSpace",
        "AuxiliaryTrafficSpace",
    ),
];

/// Data quality descriptors recorded per LOD, by their stem.
const QUALITY_STEMS: &[&str] = &[
    "geometrySrcDesc",
    "appearanceSrcDesc",
    "publicSurveySrcDesc",
    "srcScale",
];

/// The clearance heights one run extrudes LOD1 spaces by, in metres.
///
/// A space takes the first of these that applies, `overrides` by the
/// `gml:id` of its area, `overrides` by the `gml:id` of its feature, `height`,
/// then the profile's default for the feature type. A space none applies to
/// gets no solid.
#[derive(Debug, Clone, Default)]
pub struct Clearance {
    pub height: Option<f64>,
    pub overrides: HashMap<String, f64>,
}

impl Clearance {
    /// Reads `overrides` from CSV text with a `gml_id,height` header row. The
    /// error names the offending line.
    pub fn parse_csv(text: &str) -> std::result::Result<HashMap<String, f64>, String> {
        let mut lines = text.lines().enumerate().map(|(n, l)| (n + 1, l.trim()));
        let header = lines
            .by_ref()
            .find(|(_, l)| !l.is_empty())
            .ok_or("the clearance CSV is empty")?;
        let columns: Vec<&str> = header.1.split(',').map(str::trim).collect();
        if columns != ["gml_id", "height"] {
            return Err(format!(
                "line {}: the clearance CSV header must be `gml_id,height`",
                header.0
            ));
        }
        let mut out = HashMap::new();
        for (n, line) in lines {
            if line.is_empty() {
                continue;
            }
            let (id, height) = line
                .split_once(',')
                .map(|(a, b)| (a.trim(), b.trim()))
                .ok_or(format!("line {n}: expected `gml_id,height`"))?;
            let height: f64 = height
                .parse()
                .map_err(|_| format!("line {n}: `{height}` is not a number"))?;
            if id.is_empty() || height <= 0.0 {
                return Err(format!(
                    "line {n}: the id must be non-empty and the height positive"
                ));
            }
            if out.insert(id.to_owned(), height).is_some() {
                return Err(format!("line {n}: `{id}` appears twice"));
            }
        }
        Ok(out)
    }
}

/// Where a resolved clearance height came from.
#[derive(Debug, Clone, Copy)]
enum ClearanceSource {
    Area,
    Feature,
    Run,
    Default,
}

/// A clearance height and its source.
type Height = (f64, ClearanceSource);

/// Which kinds of space a feature had extruded.
#[derive(Debug, Default)]
struct Extruded {
    traffic: bool,
    auxiliary: bool,
}

#[derive(Debug, Clone)]
pub struct TranRewrite {
    core: String,
    tran: String,
    gml: String,
    /// The i-UR namespaces a quality descriptor may sit in after rename.
    iur: Vec<String>,
    rules: TranRules,
    display: Rules,
    clearance: Option<Clearance>,
    /// The override ids some feature or area has matched.
    used_overrides: Arc<Mutex<HashSet<String>>>,
}

/// What the pass learned about one feature before rewriting it.
struct Shape {
    /// Some inline area carries an LOD2 surface, or the feature holds only
    /// references and carries an LOD2 aggregate of its own.
    divided: bool,
    /// Some inline area carries an LOD3 surface, or the feature holds only
    /// references and carries an LOD3 aggregate of its own.
    has_lod3: bool,
    /// The feature's `lodType` puts every traffic space at `lane`.
    lane: bool,
}

impl TranRewrite {
    /// `clearance` enables the LOD1 extrusion.
    pub fn new(rules: &Rules, clearance: Option<Clearance>) -> Result<Self> {
        let mut iur = Vec::new();
        for prefix in ["uro", "urc", "urf", "urg", "urt"] {
            if let Ok(uri) = rules.output_ns(prefix) {
                iur.push(uri.to_owned());
            }
        }
        Ok(TranRewrite {
            core: rules.output_ns("core")?.to_owned(),
            tran: rules.output_ns("tran")?.to_owned(),
            gml: rules.output_ns("gml")?.to_owned(),
            iur,
            rules: rules.tran().clone(),
            display: rules.clone(),
            clearance,
            used_overrides: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    /// Raises the upper corner of a document's `gml:boundedBy` by the largest
    /// height this run can extrude by, so the envelope still bounds the
    /// solids. Any other member is left alone.
    pub fn raise_envelope(&self, member: &mut Element, warnings: &mut Warnings) {
        let Some(raise) = self.max_height() else {
            return;
        };
        if !member.is(&self.gml, "boundedBy") {
            return;
        }
        let corner = member
            .elements_mut()
            .find(|e| e.is(&self.gml, "Envelope"))
            .and_then(|env| env.elements_mut().find(|e| e.is(&self.gml, "upperCorner")));
        let Some(corner) = corner else {
            return;
        };
        let text = corner.text();
        let mut tokens: Vec<String> = text.split_whitespace().map(str::to_owned).collect();
        let raised = tokens
            .get(2)
            .and_then(|z| z.parse::<f64>().ok())
            .map(|z| (z + raise).to_string());
        let Some(raised) = raised else {
            warnings.add(
                "gml:boundedBy's upper corner has no height to raise, so the envelope \
                 no longer bounds the extruded spaces",
            );
            return;
        };
        tokens[2] = raised;
        corner.children = vec![Node::Text(tokens.join(" "))];
        warnings.add(format!(
            "gml:boundedBy's upper corner was raised by {raise} m, the largest clearance \
             height this run extrudes by, so the envelope still bounds the solids"
        ));
    }

    /// The largest height any space in this run can be extruded by.
    fn max_height(&self) -> Option<f64> {
        let clearance = self.clearance.as_ref()?;
        clearance
            .height
            .into_iter()
            .chain(clearance.overrides.values().copied())
            .chain(self.rules.clearance.values().copied())
            .reduce(f64::max)
    }

    /// Reports the clearance override ids that no feature or area matched.
    pub fn report_unused_clearance(&self, warnings: &mut Warnings) {
        let Some(clearance) = &self.clearance else {
            return;
        };
        let used = self
            .used_overrides
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut unused: Vec<&str> = clearance
            .overrides
            .keys()
            .filter(|id| !used.contains(*id))
            .map(String::as_str)
            .collect();
        if unused.is_empty() {
            return;
        }
        unused.sort_unstable();
        let shown = unused
            .iter()
            .take(20)
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        let more = if unused.len() > 20 { ", ..." } else { "" };
        warnings.add(format!(
            "{} id(s) in the clearance CSV matched no feature or area in the dataset: \
             {shown}{more}",
            unused.len()
        ));
    }

    /// The height a feature's spaces take unless their area says otherwise.
    fn feature_clearance(&self, feature: &Element) -> Option<Height> {
        let clearance = self.clearance.as_ref()?;
        if let Some(height) = feature
            .attr(Some(&self.gml), "id")
            .and_then(|id| self.take_override(clearance, id))
        {
            return Some((height, ClearanceSource::Feature));
        }
        if let Some(height) = clearance.height {
            return Some((height, ClearanceSource::Run));
        }
        self.rules
            .clearance
            .get(&feature.name)
            .map(|h| (*h, ClearanceSource::Default))
    }

    /// The height one area's space takes.
    fn area_clearance(&self, area: &Element, feature: Option<Height>) -> Option<Height> {
        let clearance = self.clearance.as_ref()?;
        area.attr(Some(&self.gml), "id")
            .and_then(|id| self.take_override(clearance, id))
            .map(|h| (h, ClearanceSource::Area))
            .or(feature)
    }

    fn take_override(&self, clearance: &Clearance, id: &str) -> Option<f64> {
        let height = *clearance.overrides.get(id)?;
        self.used_overrides
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.to_owned());
        Some(height)
    }

    /// The `core:lod1Solid` for a space bounded by `area`, or nothing when
    /// the area's LOD1 surface cannot be extruded.
    fn solid_for(
        &self,
        area: &Element,
        space_name: &str,
        space_id: &str,
        (height, source): Height,
        warnings: &mut Warnings,
    ) -> Option<Element> {
        let area_label = self.display.display_name(&area.name);
        let Some(surface) = area
            .child(&self.core, "lod1MultiSurface")
            .and_then(|p| p.elements().next())
        else {
            warnings.add(format!(
                "a {area_label} carries no LOD1 surface, so its tran:{space_name} got no \
                 core:lod1Solid; only LOD1 is extruded"
            ));
            return None;
        };
        let id = format!("{space_id}_solid");
        match extrude(surface, height, &self.gml, &id) {
            Ok(solid) => {
                let source = match source {
                    ClearanceSource::Area => "the clearance CSV, by area id",
                    ClearanceSource::Feature => "the clearance CSV, by feature id",
                    ClearanceSource::Run => "the run's clearance height",
                    ClearanceSource::Default => "the profile's default for the feature type",
                };
                warnings.add(format!(
                    "tran:{space_name} was written with a core:lod1Solid, its LOD1 area \
                     extruded upward by {height} m, from {source}"
                ));
                let mut property = Element::new(Name::qualified(&self.core, "lod1Solid"));
                property.push(solid);
                Some(property)
            }
            Err(reason) => {
                warnings.add(format!(
                    "a {area_label} could not be extruded ({reason}), so its \
                     tran:{space_name} got no core:lod1Solid"
                ));
                None
            }
        }
    }

    /// Adds the `lodType` the profile names for an extruded feature, beside
    /// the last `lodType` the feature already carries.
    fn stamp_lod_type(
        &self,
        feature: &mut Element,
        label: &str,
        extruded: &Extruded,
        warnings: &mut Warnings,
    ) {
        if !extruded.traffic && !extruded.auxiliary {
            return;
        }
        let Some(codes) = self.rules.clearance_lod_type.get(&feature.name) else {
            return;
        };
        let code = if extruded.auxiliary {
            &codes.auxiliary
        } else {
            &codes.traffic
        };
        if self.insert_lod_type(feature, code) {
            warnings.add(format!(
                "the {label} gained lodType {code}: the i-UR 4.0 code list names the \
                 extruded form it now carries"
            ));
        } else {
            warnings.add(format!(
                "the {label} was extruded but carries no lodType to place {code} beside, \
                 so none was added"
            ));
        }
    }

    fn insert_lod_type(&self, el: &mut Element, code: &str) -> bool {
        let last = el
            .children
            .iter()
            .rposition(|c| matches!(c, Node::Element(e) if self.is_lod_type(e)));
        if let Some(i) = last {
            let Node::Element(existing) = &el.children[i] else {
                unreachable!()
            };
            let mut added = existing.clone();
            added.children = vec![Node::Text(code.to_owned())];
            el.children.insert(i + 1, Node::Element(added));
            return true;
        }
        for child in el.elements_mut() {
            if !self.is_space_property(child) && self.insert_lod_type(child, code) {
                return true;
            }
        }
        false
    }

    /// Rewrites one top-level member in place. A member that is a property
    /// wrapping the feature is looked through.
    pub fn apply(&self, member: &mut Element, ids: &mut IdGen, warnings: &mut Warnings) {
        if self.is_feature(member) {
            self.feature(member, ids, warnings);
            return;
        }
        for child in member.elements_mut() {
            if self.is_feature(child) {
                self.feature(child, ids, warnings);
            }
        }
    }

    fn is_feature(&self, el: &Element) -> bool {
        el.name.in_ns(&self.tran) && FEATURES.contains(&el.name.local.as_str())
    }

    fn feature(&self, feature: &mut Element, ids: &mut IdGen, warnings: &mut Warnings) {
        let label = self.display.display_name(&feature.name);
        let shape = self.shape(feature);
        let clearance = self.feature_clearance(feature);
        let mut extruded = Extruded::default();

        let children = std::mem::take(&mut feature.children);
        let mut out = Vec::with_capacity(children.len());
        let mut full_width: Vec<Element> = Vec::new();
        for child in children {
            let Node::Element(child) = child else {
                out.push(child);
                continue;
            };
            if !child.name.in_ns(&self.tran) {
                out.push(Node::Element(child));
                continue;
            }
            match child.name.local.as_str() {
                "trafficArea" | "auxiliaryTrafficArea" => {
                    let space = self.space(child, &shape, clearance, &mut extruded, ids, warnings);
                    out.push(Node::Element(space));
                }
                "lod1MultiSurface" if shape.divided => warnings.add(format!(
                    "tran:lod1MultiSurface on the {label} was dropped: its areas divide \
                     the same surface at LOD2, which becomes CityGML 3.0's LOD1, and \
                     one surface may not be written twice"
                )),
                "lod1MultiSurface" => {
                    full_width.push(retag(
                        child,
                        Name::qualified(&self.core, "lod1MultiSurface"),
                    ));
                }
                "lod0Network" => full_width.push(self.network(child, warnings)),
                "lod2MultiSurface" | "lod3MultiSurface" => warnings.add(format!(
                    "tran:{} on the {label} was dropped: CityGML 2.0 defined it as the \
                     aggregate of its areas' surfaces, which the output still carries",
                    child.name.local
                )),
                _ => out.push(Node::Element(child)),
            }
        }
        if !full_width.is_empty() {
            out.push(Node::Element(self.full_width_space(
                &feature.name,
                full_width,
                clearance,
                &mut extruded,
                ids,
                warnings,
            )));
        }
        feature.children = out;

        self.renumber_quality(feature, &label, shape.divided, warnings);
        self.map_lod_types(feature, &label, shape.has_lod3, warnings);
        self.stamp_lod_type(feature, &label, &extruded, warnings);
        self.check_provenance(feature, &label, warnings);
    }

    fn shape(&self, feature: &Element) -> Shape {
        let mut inline_areas = 0;
        let mut divided = false;
        let mut has_lod3 = false;
        let mut own_lod2 = false;
        let mut own_lod3 = false;
        for child in feature.elements().filter(|c| c.name.in_ns(&self.tran)) {
            match child.name.local.as_str() {
                "trafficArea" | "auxiliaryTrafficArea" => {
                    for area in child.elements() {
                        inline_areas += 1;
                        divided |= area.child(&self.tran, "lod2MultiSurface").is_some();
                        has_lod3 |= area.child(&self.tran, "lod3MultiSurface").is_some();
                    }
                }
                "lod2MultiSurface" => own_lod2 = true,
                "lod3MultiSurface" => own_lod3 = true,
                _ => {}
            }
        }
        Shape {
            divided: divided || (inline_areas == 0 && own_lod2),
            has_lod3: has_lod3 || (inline_areas == 0 && own_lod3),
            lane: self.lod_type_lane(feature),
        }
    }

    /// True when a `lodType` under `feature` is one the profile lists as
    /// lane-level for the code list it cites.
    fn lod_type_lane(&self, feature: &Element) -> bool {
        let mut lane = false;
        self.for_each_lod_type(feature, &mut |el| {
            let file = code_list_file(el);
            if let Some(codes) = file.and_then(|f| self.rules.lane_lod_types.get(f)) {
                lane |= codes.iter().any(|c| c == el.text().trim());
            }
        });
        lane
    }

    /// Rewrites an area property into the space property that holds it.
    fn space(
        &self,
        mut property: Element,
        shape: &Shape,
        clearance: Option<Height>,
        extruded: &mut Extruded,
        ids: &mut IdGen,
        warnings: &mut Warnings,
    ) -> Element {
        let (_, prop_name, space_name) = AREA_PROPERTIES
            .iter()
            .find(|(from, _, _)| *from == property.name.local)
            .copied()
            .expect("only area properties reach here");
        let auxiliary = space_name == "AuxiliaryTrafficSpace";
        let from = self.display.display_name(&property.name);
        let to = Name::qualified(&self.tran, prop_name);

        if !property.has_element_children() {
            if let Some(href) = property.attr(Some(ns::XLINK), "href").map(str::to_owned) {
                let new = match href.rsplit_once('#') {
                    Some((document, fragment)) => format!("{document}#{}", space_id(fragment)),
                    None => {
                        warnings.add(format!(
                            "{from} references {href}, which names no element, so the \
                             reference was kept as it is; check the result"
                        ));
                        href.clone()
                    }
                };
                property.set_attr(Name::qualified(ns::XLINK, "href"), new);
                warnings.add(format!(
                    "{from} by reference became {} by reference: CityGML 3.0 reaches an \
                     area through the space it bounds, and that space's id is derived \
                     from the area's",
                    self.display.display_name(&to)
                ));
            }
            return retag(property, to);
        }

        let mut out = Element::new(to.clone());
        out.attrs = std::mem::take(&mut property.attrs);
        for child in std::mem::take(&mut property.children) {
            match child {
                Node::Element(area) => {
                    let space = self.wrap(
                        area, space_name, auxiliary, shape, clearance, extruded, ids, warnings,
                    );
                    out.push(space);
                }
                other => out.children.push(other),
            }
        }
        warnings.add(format!(
            "{from} became {} holding a tran:{space_name}, with the area under \
             core:boundary: CityGML 3.0 reaches every surface through a space",
            self.display.display_name(&to)
        ));
        out
    }

    /// Wraps one area in its space, moving the area's geometry down one LOD.
    #[allow(clippy::too_many_arguments)]
    fn wrap(
        &self,
        mut area: Element,
        space_name: &str,
        auxiliary: bool,
        shape: &Shape,
        clearance: Option<Height>,
        extruded: &mut Extruded,
        ids: &mut IdGen,
        warnings: &mut Warnings,
    ) -> Element {
        let lane = !auxiliary
            && (shape.lane
                || self.rules.lane_function.as_deref().is_some_and(|code| {
                    area.elements()
                        .any(|e| e.is(&self.tran, "function") && e.text().trim() == code)
                }));
        self.shift_geometry(&mut area, warnings);

        let id = match area.attr(Some(&self.gml), "id") {
            Some(id) => space_id(id),
            None => {
                warnings.add(format!(
                    "a {} carries no gml:id, so its space took a minted one that no \
                     other file can derive",
                    self.display.display_name(&area.name)
                ));
                ids.mint()
            }
        };
        let granularity = if lane { "lane" } else { "way" };
        warnings.add(format!(
            "tran:{space_name} was written with granularity={granularity}, which \
             CityGML 3.0 requires and CityGML 2.0 does not record"
        ));

        let solid = self
            .area_clearance(&area, clearance)
            .and_then(|height| self.solid_for(&area, space_name, &id, height, warnings));

        let mut space = Element::new(Name::qualified(&self.tran, space_name));
        space.set_attr(Name::qualified(&self.gml, "id"), id);
        space.push(Element::with_text(
            Name::qualified(&self.tran, "granularity"),
            granularity,
        ));
        let mut boundary = Element::new(Name::qualified(&self.core, "boundary"));
        boundary.push(area);
        space.push(boundary);
        if let Some(solid) = solid {
            space.push(solid);
            if auxiliary {
                extruded.auxiliary = true;
            } else {
                extruded.traffic = true;
            }
        }
        space
    }

    /// Moves an area's `lodNMultiSurface` to `core:lod(N-1)MultiSurface`.
    fn shift_geometry(&self, area: &mut Element, warnings: &mut Warnings) {
        let area_label = self.display.display_name(&area.name);
        for child in area.elements_mut() {
            if !child.name.in_ns(&self.tran) {
                continue;
            }
            let to = match child.name.local.as_str() {
                "lod2MultiSurface" => "lod1MultiSurface",
                "lod3MultiSurface" => "lod2MultiSurface",
                "lod4MultiSurface" => "lod3MultiSurface",
                _ => continue,
            };
            warnings.add(format!(
                "tran:{} on a {} became core:{to}: CityGML 3.0's LOD1 has no height and \
                 its LOD2 does, which is where CityGML 2.0 divided LOD2 from LOD3",
                child.name.local, area_label
            ));
            child.name = Name::qualified(&self.core, to);
        }
    }

    /// Rewrites `tran:lod0Network` into `core:lod0MultiCurve`, whose content
    /// is a `gml:MultiCurve`.
    fn network(&self, mut property: Element, warnings: &mut Warnings) -> Element {
        for curve in property.elements_mut() {
            if curve.is(&self.gml, "CompositeCurve") {
                curve.name = Name::qualified(&self.gml, "MultiCurve");
            } else if !curve.is(&self.gml, "MultiCurve") {
                warnings.add(format!(
                    "tran:lod0Network held gml:{}, and core:lod0MultiCurve expects a \
                     gml:MultiCurve; check the result",
                    curve.name.local
                ));
            }
        }
        warnings.add(
            "tran:lod0Network became core:lod0MultiCurve on the full-width area, its \
             gml:CompositeCurve written as a gml:MultiCurve with the same members",
        );
        retag(property, Name::qualified(&self.core, "lod0MultiCurve"))
    }

    /// Builds the space and area that carry a feature's own geometry.
    fn full_width_space(
        &self,
        feature: &Name,
        geometry: Vec<Element>,
        clearance: Option<Height>,
        extruded: &mut Extruded,
        ids: &mut IdGen,
        warnings: &mut Warnings,
    ) -> Element {
        let label = self.display.display_name(feature);
        let mut area = Element::new(Name::qualified(&self.tran, "TrafficArea"));
        area.set_attr(Name::qualified(&self.gml, "id"), ids.mint());
        match self.rules.full_width_function.get(feature) {
            Some(code) => {
                let mut function =
                    Element::with_text(Name::qualified(&self.tran, "function"), code.clone());
                if let Some(code_space) = &self.rules.function_code_space {
                    function.set_attr(Name::unqualified("codeSpace"), code_space.clone());
                }
                area.push(function);
                warnings.add(format!(
                    "the {label}'s own geometry became a minted tran:TrafficArea with \
                     function {code} under a tran:TrafficSpace with granularity=way: \
                     CityGML 3.0 gives a transportation object no geometry of its own"
                ));
            }
            None => warnings.add(format!(
                "the {label}'s own geometry became a minted tran:TrafficArea under a \
                 tran:TrafficSpace with granularity=way, with no tran:function since the \
                 profile's [tran] names no full-width code for {label}"
            )),
        }
        for g in geometry {
            area.push(g);
        }

        let space_id = ids.mint();
        let solid = clearance
            .and_then(|height| self.solid_for(&area, "TrafficSpace", &space_id, height, warnings));
        let mut space = Element::new(Name::qualified(&self.tran, "TrafficSpace"));
        space.set_attr(Name::qualified(&self.gml, "id"), space_id);
        space.push(Element::with_text(
            Name::qualified(&self.tran, "granularity"),
            "way",
        ));
        let mut boundary = Element::new(Name::qualified(&self.core, "boundary"));
        boundary.push(area);
        space.push(boundary);
        if let Some(solid) = solid {
            space.push(solid);
            extruded.traffic = true;
        }

        let mut property = Element::new(Name::qualified(&self.tran, "trafficSpace"));
        property.push(space);
        property
    }

    /// Renumbers the LOD-indexed quality descriptors under `el` to follow the
    /// geometry. Spaces are not entered, since they hold no descriptors of the
    /// feature's.
    fn renumber_quality(
        &self,
        el: &mut Element,
        label: &str,
        divided: bool,
        warnings: &mut Warnings,
    ) {
        let children = std::mem::take(&mut el.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            let Node::Element(mut child) = child else {
                out.push(child);
                continue;
            };
            let Some((stem, lod)) = self.quality_descriptor(&child) else {
                if !self.is_space_property(&child) {
                    self.renumber_quality(&mut child, label, divided, warnings);
                }
                out.push(Node::Element(child));
                continue;
            };
            let from = self.display.display_name(&child.name);
            let to = match (lod, divided) {
                ('1', true) => {
                    warnings.add(format!(
                        "{from} on the {label} was dropped: the surface it described \
                         gave way to the divided areas, whose own descriptors take LOD1"
                    ));
                    continue;
                }
                ('2', false) => {
                    warnings.add(format!(
                        "{from} on the {label} was dropped: no area carries the LOD2 \
                         surface it described, so the band it moves to is empty"
                    ));
                    continue;
                }
                ('2', true) => '1',
                ('3', _) => '2',
                _ => {
                    out.push(Node::Element(child));
                    continue;
                }
            };
            child.name = Name::qualified(
                child.name.ns.clone().unwrap_or_default(),
                format!("{stem}Lod{to}"),
            );
            warnings.add(format!(
                "{from} became {}: the geometry it describes moved down one LOD",
                self.display.display_name(&child.name)
            ));
            out.push(Node::Element(child));
        }
        el.children = out;
    }

    /// Rewrites every `lodType` under `feature` through the profile's map for
    /// the code list it cites.
    fn map_lod_types(
        &self,
        feature: &mut Element,
        label: &str,
        has_lod3: bool,
        warnings: &mut Warnings,
    ) {
        let children = std::mem::take(&mut feature.children);
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            let Node::Element(mut child) = child else {
                out.push(child);
                continue;
            };
            if self.is_space_property(&child) {
                out.push(Node::Element(child));
                continue;
            }
            if !self.is_lod_type(&child) {
                self.map_lod_types(&mut child, label, has_lod3, warnings);
                out.push(Node::Element(child));
                continue;
            }
            let old = child.text().trim().to_owned();
            let Some(file) = code_list_file(&child).map(str::to_owned) else {
                warnings.add(format!(
                    "lodType {old} on the {label} cites no code list and was kept"
                ));
                out.push(Node::Element(child));
                continue;
            };
            let Some(map) = self.rules.lod_type_map.get(&file) else {
                warnings.add(format!(
                    "lodType {old} from {file} has no map in the profile's [tran] and \
                     was kept"
                ));
                out.push(Node::Element(child));
                continue;
            };
            match map.get(&old) {
                Some(new) => {
                    warnings.add(format!(
                        "lodType {old} became {new}: the i-UR 4.0 {file} numbers the \
                         detailed LODs the way CityGML 3.0 does"
                    ));
                    if !has_lod3 {
                        warnings.add(format!(
                            "the {label} declares lodType {old} but no area carries the \
                             detailed surface it classifies; the value was mapped as \
                             declared"
                        ));
                    }
                    child.children = vec![Node::Text(new.clone())];
                    out.push(Node::Element(child));
                }
                None => warnings.add(format!(
                    "lodType {old} has no entry in the i-UR 4.0 {file} and was dropped"
                )),
            }
        }
        feature.children = out;
    }

    /// Reports a geometry descriptor that says a band was not created while
    /// the feature carries a surface in it.
    fn check_provenance(&self, feature: &Element, label: &str, warnings: &mut Warnings) {
        for lod in ['1', '2'] {
            let geometry = format!("lod{lod}MultiSurface");
            if !has_descendant(feature, &self.core, &geometry) {
                continue;
            }
            let descriptor = format!("geometrySrcDescLod{lod}");
            let mut says_absent = false;
            self.for_each_descriptor(feature, &mut |el| {
                says_absent |= el.name.local == descriptor && el.text().trim() == "999";
            });
            if says_absent {
                warnings.add(format!(
                    "the {label} says its LOD{lod} geometry was not created (999) but \
                     carries LOD{lod} surfaces; the source's provenance was kept as it is"
                ));
            }
        }
    }

    /// Calls `f` on every quality descriptor under `el`, spaces excluded.
    fn for_each_descriptor(&self, el: &Element, f: &mut dyn FnMut(&Element)) {
        for child in el.elements() {
            if self.is_space_property(child) {
                continue;
            }
            if self.quality_descriptor(child).is_some() {
                f(child);
            } else {
                self.for_each_descriptor(child, f);
            }
        }
    }

    /// Calls `f` on every `lodType` under `el`, spaces excluded.
    fn for_each_lod_type(&self, el: &Element, f: &mut dyn FnMut(&Element)) {
        for child in el.elements() {
            if self.is_space_property(child) {
                continue;
            }
            if self.is_lod_type(child) {
                f(child);
            } else {
                self.for_each_lod_type(child, f);
            }
        }
    }

    /// The stem and LOD digit of an i-UR quality descriptor, such as
    /// `("geometrySrcDesc", '2')` for `urc:geometrySrcDescLod2`.
    fn quality_descriptor(&self, el: &Element) -> Option<(&'static str, char)> {
        if !self.iur.iter().any(|uri| el.name.in_ns(uri)) {
            return None;
        }
        let stem = QUALITY_STEMS
            .iter()
            .copied()
            .find(|stem| el.name.local.starts_with(stem))?;
        let rest = el.name.local[stem.len()..].strip_prefix("Lod")?;
        let mut chars = rest.chars();
        let digit = chars.next().filter(char::is_ascii_digit)?;
        chars.as_str().is_empty().then_some((stem, digit))
    }

    fn is_lod_type(&self, el: &Element) -> bool {
        el.name.local == "lodType" && self.iur.iter().any(|uri| el.name.in_ns(uri))
    }

    /// The properties that hold spaces, before and after this pass.
    fn is_space_property(&self, el: &Element) -> bool {
        el.name.in_ns(&self.tran)
            && matches!(
                el.name.local.as_str(),
                "trafficArea" | "auxiliaryTrafficArea" | "trafficSpace" | "auxiliaryTrafficSpace"
            )
    }
}

/// The `gml:id` of the space wrapping the area whose id is `area`.
pub fn space_id(area: &str) -> String {
    format!("{}_space", id_seed(area))
}

/// The file name a `codeSpace` cites.
fn code_list_file(el: &Element) -> Option<&str> {
    let value = el.attr(None, "codeSpace")?;
    Some(value.rsplit_once('/').map_or(value, |(_, file)| file))
}

/// Replaces an element's name, keeping its attributes and children.
fn retag(mut el: Element, name: Name) -> Element {
    el.name = name;
    el
}

/// True when an element named `(ns, local)` sits anywhere under `el`.
fn has_descendant(el: &Element, ns: &str, local: &str) -> bool {
    el.elements()
        .any(|child| child.is(ns, local) || has_descendant(child, ns, local))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_PROFILE;

    const TRAN: &str = "http://www.opengis.net/citygml/transportation/3.0";
    const URC: &str = "https://www.geospatial.jp/iur/urc/4.0";
    const URO: &str = "https://www.geospatial.jp/iur/uro/4.0";
    const ROAD_LOD_TYPE: &str = "../../codelists/Road_lodType.xml";

    fn rewrite() -> TranRewrite {
        TranRewrite::new(&Rules::from_toml(DEFAULT_PROFILE).unwrap(), None).unwrap()
    }

    fn multi_surface(local: &str) -> Element {
        let mut prop = Element::new(Name::qualified(TRAN, local));
        prop.push(Element::new(Name::qualified(ns::GML_32, "MultiSurface")));
        prop
    }

    fn area(class: &str, id: &str, function: &str, geometry: &[&str]) -> Element {
        let mut a = Element::new(Name::qualified(TRAN, class));
        a.set_attr(Name::qualified(ns::GML_32, "id"), id);
        a.push(Element::with_text(
            Name::qualified(TRAN, "function"),
            function,
        ));
        for g in geometry {
            a.push(multi_surface(g));
        }
        let property = if class == "TrafficArea" {
            "trafficArea"
        } else {
            "auxiliaryTrafficArea"
        };
        let mut p = Element::new(Name::qualified(TRAN, property));
        p.push(a);
        p
    }

    fn quality(fields: &[(&str, &str)]) -> Element {
        let mut class = Element::new(Name::qualified(URC, "ExteriorDataQualityAttribute"));
        for (local, value) in fields {
            let mut el = Element::with_text(Name::qualified(URC, *local), *value);
            if *local == "lodType" {
                el.set_attr(Name::unqualified("codeSpace"), ROAD_LOD_TYPE);
            }
            class.push(el);
        }
        let mut prop = Element::new(Name::qualified(URO, "tranDataQualityAttribute"));
        prop.push(class);
        prop
    }

    fn road(children: Vec<Element>) -> Element {
        let mut r = Element::new(Name::qualified(TRAN, "Road"));
        r.set_attr(Name::qualified(ns::GML_32, "id"), "road_1");
        for c in children {
            r.push(c);
        }
        r
    }

    fn convert(mut feature: Element) -> (Element, Warnings) {
        let mut warnings = Warnings::new();
        let mut ids = IdGen::new("road_1");
        rewrite().apply(&mut feature, &mut ids, &mut warnings);
        (feature, warnings)
    }

    fn feature_with(local: &str, id: &str, children: Vec<Element>) -> Element {
        let mut f = Element::new(Name::qualified(TRAN, local));
        f.set_attr(Name::qualified(ns::GML_32, "id"), id);
        for c in children {
            f.push(c);
        }
        f
    }

    fn square(local: &str) -> Element {
        let mut ring = Element::new(Name::qualified(ns::GML_32, "LinearRing"));
        ring.push(Element::with_text(
            Name::qualified(ns::GML_32, "posList"),
            "0 0 0 1 0 0 1 1 0 0 1 0 0 0 0",
        ));
        let mut exterior = Element::new(Name::qualified(ns::GML_32, "exterior"));
        exterior.push(ring);
        let mut polygon = Element::new(Name::qualified(ns::GML_32, "Polygon"));
        polygon.push(exterior);
        let mut member = Element::new(Name::qualified(ns::GML_32, "surfaceMember"));
        member.push(polygon);
        let mut ms = Element::new(Name::qualified(ns::GML_32, "MultiSurface"));
        ms.push(member);
        let mut prop = Element::new(Name::qualified(TRAN, local));
        prop.push(ms);
        prop
    }

    fn square_area(id: &str) -> Element {
        let mut a = Element::new(Name::qualified(TRAN, "TrafficArea"));
        a.set_attr(Name::qualified(ns::GML_32, "id"), id);
        a.push(square("lod2MultiSurface"));
        let mut p = Element::new(Name::qualified(TRAN, "trafficArea"));
        p.push(a);
        p
    }

    /// The height of the solid's top face, or nothing when the space has no
    /// solid.
    fn top(space: &Element) -> Option<String> {
        let shell = space
            .child(ns::CITYGML_3, "lod1Solid")?
            .child(ns::GML_32, "Solid")?
            .child(ns::GML_32, "exterior")?
            .child(ns::GML_32, "Shell")?;
        let top = shell.elements().nth(1)?.elements().next()?;
        let text = top
            .child(ns::GML_32, "exterior")?
            .child(ns::GML_32, "LinearRing")?
            .child(ns::GML_32, "posList")?
            .text();
        text.split_whitespace().last().map(str::to_owned)
    }

    fn spaces(feature: &Element) -> Vec<&Element> {
        feature
            .elements()
            .filter(|e| e.name.local == "trafficSpace" || e.name.local == "auxiliaryTrafficSpace")
            .filter_map(|p| p.elements().next())
            .collect()
    }

    fn granularity(space: &Element) -> String {
        space.child(TRAN, "granularity").unwrap().text()
    }

    fn bounded_area(space: &Element) -> &Element {
        space
            .child(ns::CITYGML_3, "boundary")
            .unwrap()
            .elements()
            .next()
            .unwrap()
    }

    fn quality_locals(feature: &Element) -> Vec<String> {
        feature
            .child(URO, "tranDataQualityAttribute")
            .unwrap()
            .elements()
            .next()
            .unwrap()
            .elements()
            .map(|e| format!("{}={}", e.name.local, e.text()))
            .collect()
    }

    /// A divided road keeps its areas at LOD1 under spaces named after them,
    /// and loses its own full-width surface and aggregates.
    #[test]
    fn a_divided_road_moves_its_areas_down_one_lod_under_derived_spaces() {
        let (r, w) = convert(road(vec![
            multi_surface("lod1MultiSurface"),
            multi_surface("lod2MultiSurface"),
            multi_surface("lod3MultiSurface"),
            area(
                "TrafficArea",
                "tra_a",
                "1000",
                &["lod2MultiSurface", "lod3MultiSurface"],
            ),
            area(
                "AuxiliaryTrafficArea",
                "ata_b",
                "3000",
                &["lod2MultiSurface"],
            ),
            quality(&[
                ("geometrySrcDescLod1", "000"),
                ("geometrySrcDescLod2", "101"),
                ("geometrySrcDescLod3", "000"),
                ("srcScaleLod3", "2"),
            ]),
        ]));

        assert!(r.elements().all(|e| !e.name.local.starts_with("lod")));
        let spaces = spaces(&r);
        assert_eq!(spaces.len(), 2, "no full-width space was minted");
        assert_eq!(spaces[0].attr(Some(ns::GML_32), "id"), Some("tra_a_space"));
        assert_eq!(spaces[1].name.local, "AuxiliaryTrafficSpace");
        assert_eq!(granularity(spaces[0]), "way");
        assert_eq!(granularity(spaces[1]), "way");

        let area = bounded_area(spaces[0]);
        assert!(area.child(ns::CITYGML_3, "lod1MultiSurface").is_some());
        assert!(area.child(ns::CITYGML_3, "lod2MultiSurface").is_some());
        assert!(area.child(TRAN, "lod2MultiSurface").is_none());

        assert_eq!(
            quality_locals(&r),
            [
                "geometrySrcDescLod1=101",
                "geometrySrcDescLod2=000",
                "srcScaleLod2=2"
            ],
            "old LOD1 provenance goes, old LOD2 and LOD3 move down"
        );
        assert!(
            w.iter()
                .any(|(m, _)| m.contains("tran:lod1MultiSurface on the tran:Road was dropped"))
        );
    }

    /// A road with no LOD2 area keeps its full-width surface as a minted
    /// space and area at LOD1, and LOD3 areas go to LOD2.
    #[test]
    fn an_undivided_road_keeps_its_full_width_surface_on_a_minted_area() {
        let (r, _) = convert(road(vec![
            multi_surface("lod1MultiSurface"),
            area("TrafficArea", "tfa_a", "1000", &["lod3MultiSurface"]),
            quality(&[
                ("geometrySrcDescLod1", "000"),
                ("geometrySrcDescLod2", "999"),
                ("geometrySrcDescLod3", "000"),
            ]),
        ]));

        let spaces = spaces(&r);
        assert_eq!(spaces.len(), 2);
        let full = bounded_area(spaces[1]);
        assert_eq!(full.child(TRAN, "function").unwrap().text(), "1140");
        assert_eq!(
            full.child(TRAN, "function")
                .unwrap()
                .attr(None, "codeSpace"),
            Some("../../codelists/TrafficArea_function.xml")
        );
        assert!(full.child(ns::CITYGML_3, "lod1MultiSurface").is_some());
        assert_eq!(granularity(spaces[1]), "way");
        assert_eq!(
            spaces[1].attr(Some(ns::GML_32), "id"),
            Some("road_1_2"),
            "the minted pair counts from the feature's own generator"
        );
        assert!(
            bounded_area(spaces[0])
                .child(ns::CITYGML_3, "lod2MultiSurface")
                .is_some()
        );
        assert_eq!(
            quality_locals(&r),
            ["geometrySrcDescLod1=000", "geometrySrcDescLod2=000"],
            "the empty LOD2 band's descriptor goes, LOD3 moves down"
        );
    }

    /// Granularity comes from the area's function, or from the feature's
    /// lodType for every traffic space at once. Auxiliary spaces stay way.
    #[test]
    fn granularity_follows_the_lane_function_and_the_lod_type() {
        let (r, _) = convert(road(vec![
            area("TrafficArea", "a", "1010", &["lod3MultiSurface"]),
            area("TrafficArea", "b", "2000", &["lod3MultiSurface"]),
            area("AuxiliaryTrafficArea", "c", "3000", &["lod3MultiSurface"]),
        ]));
        let g: Vec<String> = spaces(&r).iter().map(|s| granularity(s)).collect();
        assert_eq!(g, ["lane", "way", "way"]);

        let (r, _) = convert(road(vec![
            area("TrafficArea", "a", "1000", &["lod3MultiSurface"]),
            area("TrafficArea", "b", "2000", &["lod3MultiSurface"]),
            area("AuxiliaryTrafficArea", "c", "3000", &["lod3MultiSurface"]),
            quality(&[("lodType", "3.3")]),
        ]));
        let g: Vec<String> = spaces(&r).iter().map(|s| granularity(s)).collect();
        assert_eq!(g, ["lane", "lane", "way"]);
        assert_eq!(quality_locals(&r), ["lodType=2.2"]);
    }

    /// An area held by reference becomes a reference to the space whose id
    /// the other file derived from the same area id.
    #[test]
    fn a_referenced_area_becomes_a_reference_to_its_derived_space() {
        let mut by_ref = Element::new(Name::qualified(TRAN, "trafficArea"));
        by_ref.set_attr(
            Name::qualified(ns::XLINK, "href"),
            "../tran/53396570_tran_6697_op.gml#tra_x",
        );
        let mut square = Element::new(Name::qualified(TRAN, "Square"));
        square.set_attr(Name::qualified(ns::GML_32, "id"), "sq_1");
        square.push(by_ref);
        square.push(multi_surface("lod2MultiSurface"));
        square.push(multi_surface("lod3MultiSurface"));
        square.push(quality(&[
            ("geometrySrcDescLod1", "999"),
            ("geometrySrcDescLod2", "000"),
            ("lodType", "3.0"),
        ]));

        let (s, w) = convert(square);
        assert!(
            !w.iter().any(|(m, _)| m.contains("declares lodType")),
            "the aggregate stands in for the areas' LOD3: {w:?}"
        );

        let prop = s.child(TRAN, "trafficSpace").expect("trafficSpace");
        assert_eq!(
            prop.attr(Some(ns::XLINK), "href"),
            Some("../tran/53396570_tran_6697_op.gml#tra_x_space")
        );
        assert!(s.child(TRAN, "lod2MultiSurface").is_none());
        assert_eq!(
            quality_locals(&s),
            ["geometrySrcDescLod1=000", "lodType=2.0"],
            "the aggregate stands in for the areas the square cannot see"
        );
    }

    /// A track's network becomes the LOD0 curve of its full-width area, with
    /// the track's own function code and the curve container 3.0 expects.
    #[test]
    fn a_network_becomes_the_full_width_areas_lod0_curve() {
        let mut network = Element::new(Name::qualified(TRAN, "lod0Network"));
        let mut composite = Element::new(Name::qualified(ns::GML_32, "CompositeCurve"));
        composite.push(Element::new(Name::qualified(ns::GML_32, "curveMember")));
        network.push(composite);
        let mut track = Element::new(Name::qualified(TRAN, "Track"));
        track.set_attr(Name::qualified(ns::GML_32, "id"), "trk_1");
        track.push(network);

        let (t, _) = convert(track);

        let full = bounded_area(spaces(&t)[0]);
        assert_eq!(full.child(TRAN, "function").unwrap().text(), "2040");
        let curve = full
            .child(ns::CITYGML_3, "lod0MultiCurve")
            .unwrap()
            .child(ns::GML_32, "MultiCurve")
            .expect("a MultiCurve with the same members");
        assert!(curve.child(ns::GML_32, "curveMember").is_some());
    }

    #[test]
    fn a_lod_type_with_no_4_0_entry_is_dropped() {
        let (r, w) = convert(road(vec![quality(&[("lodType", "8")])]));
        assert!(quality_locals(&r).is_empty());
        assert!(w.iter().any(|(m, _)| m.contains("lodType 8 has no entry")));
    }
    /// The height a space is extruded by follows the run's inputs in order,
    /// area override, feature override, run height, then the profile's
    /// per-type default, and a type without a default gets no solid. An
    /// extruded Railway gains the code list's lodType for that form.
    #[test]
    fn clearance_resolves_in_order_and_stamps_the_railway_lod_type() {
        let rules = Rules::from_toml(DEFAULT_PROFILE).unwrap();
        let with = |clearance: Clearance| TranRewrite::new(&rules, Some(clearance)).unwrap();
        let run = |rewrite: &TranRewrite, mut feature: Element| {
            let mut warnings = Warnings::new();
            let mut ids = IdGen::new("f");
            rewrite.apply(&mut feature, &mut ids, &mut warnings);
            feature
        };

        let overrides = HashMap::from([("a1".to_owned(), 1.0), ("road_1".to_owned(), 2.0)]);
        let rewrite = with(Clearance {
            height: Some(3.0),
            overrides,
        });
        let divided = run(&rewrite, road(vec![square_area("a1"), square_area("a2")]));
        let by_id = spaces(&divided);
        assert_eq!(top(by_id[0]).as_deref(), Some("1"));
        assert_eq!(top(by_id[1]).as_deref(), Some("2"));

        let rewrite = with(Clearance {
            height: Some(3.0),
            overrides: HashMap::new(),
        });
        let by_run = run(&rewrite, road(vec![square_area("a1")]));
        assert_eq!(top(spaces(&by_run)[0]).as_deref(), Some("3"));

        let rewrite = with(Clearance::default());
        let by_default = run(&rewrite, road(vec![square_area("a1")]));
        assert_eq!(top(spaces(&by_default)[0]).as_deref(), Some("4.5"));

        let railway = run(
            &rewrite,
            feature_with("Railway", "rwy_1", vec![square_area("r1")]),
        );
        assert_eq!(top(spaces(&railway)[0]), None);

        let rewrite = with(Clearance {
            height: Some(5.7),
            overrides: HashMap::new(),
        });
        let railway = run(
            &rewrite,
            feature_with(
                "Railway",
                "rwy_1",
                vec![square_area("r1"), quality(&[("lodType", "3.0")])],
            ),
        );
        assert_eq!(top(spaces(&railway)[0]).as_deref(), Some("5.7"));
        assert_eq!(quality_locals(&railway), ["lodType=2.0", "lodType=1.0"]);

        let mut warnings = Warnings::new();
        rewrite.report_unused_clearance(&mut warnings);
        assert!(warnings.is_empty());
    }
}
