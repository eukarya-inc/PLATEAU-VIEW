//! Adding an i-UR `lodType` code to a feature.
//!
//! A new code goes next to the feature's first `lodType`, taking its name
//! and `codeSpace`. A feature without one gets it in the first element
//! holding a data quality descriptor, named in that descriptor's namespace,
//! with a `codeSpace` naming the type's code list in the directory the
//! descriptor's own `codeSpace` points to.

use crate::iur::IurModules;
use crate::xml::{Element, Name};

/// What [`add`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Added {
    Added,
    /// The feature already carries the code.
    Present,
    /// The feature has neither a `lodType` nor a quality descriptor.
    NoHolder,
}

/// Adds a `lodType` holding `code` from the code list `list` to `feature`.
pub fn add(iur: &IurModules, feature: &mut Element, code: &str, list: &str) -> Added {
    let Some(path) = holder(iur, feature) else {
        return Added::NoHolder;
    };
    let holder = path.iter().fold(feature, |el, &i| {
        el.elements_mut().nth(i).expect("a path just found")
    });
    if holder
        .elements()
        .any(|c| is_lod_type(iur, c) && c.text().trim() == code)
    {
        return Added::Present;
    }
    let element = new_lod_type(iur, holder, code, list);
    holder.push(element);
    Added::Added
}

/// Adds a `lodType` holding `code` from the code list `list` to `holder`,
/// named and with a `codeSpace` as [`add`] would place it in `feature`.
/// `holder` lies outside `feature`.
pub fn add_to(
    iur: &IurModules,
    feature: &Element,
    holder: &mut Element,
    code: &str,
    list: &str,
) -> Added {
    let Some(path) = self::holder(iur, feature) else {
        return Added::NoHolder;
    };
    if holder
        .elements()
        .any(|c| is_lod_type(iur, c) && c.text().trim() == code)
    {
        return Added::Present;
    }
    let template = path.iter().fold(feature, |el, &i| {
        el.elements().nth(i).expect("a path just found")
    });
    holder.push(new_lod_type(iur, template, code, list));
    Added::Added
}

/// An i-UR `lodType`, whatever code it holds.
pub fn is_lod_type(iur: &IurModules, el: &Element) -> bool {
    el.name.local == "lodType" && iur.contains(&el.name)
}

/// The child indices from `el` down to the first element holding a
/// `lodType`, or else the first holding a quality descriptor.
fn holder(iur: &IurModules, el: &Element) -> Option<Vec<usize>> {
    let lod_type = |e: &Element| e.elements().any(|c| is_lod_type(iur, c));
    let descriptor = |e: &Element| {
        e.elements()
            .any(|c| iur.quality_descriptor(&c.name).is_some())
    };
    path_to(el, &lod_type).or_else(|| path_to(el, &descriptor))
}

fn new_lod_type(iur: &IurModules, holder: &Element, code: &str, list: &str) -> Element {
    let (name, code_space) = match holder.elements().find(|c| is_lod_type(iur, c)) {
        Some(sibling) => (
            sibling.name.clone(),
            sibling.attr(None, "codeSpace").map(str::to_owned),
        ),
        None => {
            let descriptor = holder
                .elements()
                .find(|c| iur.quality_descriptor(&c.name).is_some())
                .expect("the holder has a descriptor");
            let dir = descriptor
                .attr(None, "codeSpace")
                .and_then(|c| c.rsplit_once('/'))
                .map(|(dir, _)| format!("{dir}/{list}"));
            (Name::new(descriptor.name.ns.clone(), "lodType"), dir)
        }
    };
    let mut el = Element::with_text(name, code);
    if let Some(code_space) = code_space {
        el.set_attr(Name::unqualified("codeSpace"), code_space);
    }
    el
}

/// The child indices from `el` down to the first descendant, `el` included,
/// for which `found` holds.
fn path_to(el: &Element, found: &dyn Fn(&Element) -> bool) -> Option<Vec<usize>> {
    if found(el) {
        return Some(Vec::new());
    }
    el.elements().enumerate().find_map(|(i, child)| {
        path_to(child, found).map(|mut path| {
            path.insert(0, i);
            path
        })
    })
}
