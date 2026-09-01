//! CityGML 2.0 -> 3.0 conversion for PLATEAU city models.
//!
//! The crate is split into four layers:
//!
//! * [`dataset`] resolves whatever the user handed us (a PLATEAU folder, a zip of
//!   one, or a loose set of per-part zips) into a single on-disk PLATEAU tree.
//! * [`xml`] is a small namespace-aware XML tree. It streams a CityGML document
//!   and hands out one fully-materialised subtree per top-level member, so a
//!   feature can be restructured without holding the whole file in memory.
//! * [`profile`] holds the declarative part of the mapping (namespace bumps,
//!   element renames, child ordering) loaded from a TOML profile.
//! * [`transform`], [`common`], [`xal`], [`app`], [`lod4`], [`bldg`] and [`iur`]
//!   apply the profile and the structural rewrites a rename table cannot
//!   express.
//!
//! [`convert`] ties them together and [`report`] carries the diagnostics back out.

pub mod app;
pub mod bldg;
pub mod common;
pub mod convert;
pub mod dataset;
pub mod detect;
pub mod error;
pub mod iur;
pub mod lod4;
pub mod profile;
pub mod report;
pub mod transform;
pub mod xal;
pub mod xml;

pub use error::{Error, Result};

/// The profile fragments compiled into the binary, named as a profile's `base`
/// names them.
///
/// One fragment carries the CityGML 2.0 -> 3.0 mapping and another the i-UR 4.0
/// target. What is left in each profile of [`PROFILES`] is the i-UR source
/// half.
pub const FRAGMENTS: &[(&str, &str)] = &[
    (
        "citygml-2.0-to-3.0",
        include_str!("../../profiles/citygml-2.0-to-3.0.toml"),
    ),
    (
        "iur-4.0-target",
        include_str!("../../profiles/iur-4.0-target.toml"),
    ),
];

/// The conversion profiles compiled into the binary, newest source version
/// last, one per source i-UR version.
///
/// [`detect::select`] picks between them by reading the namespaces a document
/// declares. `--profile` overrides the choice with a file.
pub const PROFILES: &[(&str, &str)] = &[
    (
        "iur-3.0-to-4.0",
        include_str!("../../profiles/iur-3.0-to-4.0.toml"),
    ),
    (
        "iur-3.1-to-4.0",
        include_str!("../../profiles/iur-3.1-to-4.0.toml"),
    ),
    (
        "iur-3.2-to-4.0",
        include_str!("../../profiles/iur-3.2-to-4.0.toml"),
    ),
];

/// The i-UR 4.0 schemas written into a converted package's `schemas/`.
///
/// A PLATEAU package resolves i-UR through a relative path into its own
/// `schemas/` folder, so a converted package carries the i-UR side itself.
pub const IUR_4_0_SCHEMAS: &[(&str, &str)] = &[
    (
        "iur/uro/4.0/urbanObject.xsd",
        include_str!("../../fixtures/schemas/iur/uro/4.0/urbanObject.xsd"),
    ),
    (
        "iur/urc/4.0/urbanCore.xsd",
        include_str!("../../fixtures/schemas/iur/urc/4.0/urbanCore.xsd"),
    ),
    (
        "iur/urf/4.0/urbanFunction.xsd",
        include_str!("../../fixtures/schemas/iur/urf/4.0/urbanFunction.xsd"),
    ),
    (
        "iur/urg/4.0/statisticalGrid.xsd",
        include_str!("../../fixtures/schemas/iur/urg/4.0/statisticalGrid.xsd"),
    ),
    (
        "iur/urt/4.0/publicTransit.xsd",
        include_str!("../../fixtures/schemas/iur/urt/4.0/publicTransit.xsd"),
    ),
];

// The published i-UR 4.0 code lists (`CODELISTS_4_0`), vendored in
// `fixtures/codelists/` and embedded by `build.rs`. They replace the input's
// copies of the same files. Lists the input authors itself, matched by the
// profile's `[codelists] local` patterns, are kept as shipped.
include!(concat!(env!("OUT_DIR"), "/codelists_gen.rs"));

/// The profile used when a document says nothing about which one it needs.
pub const DEFAULT_PROFILE: &str = PROFILES[1].1;
