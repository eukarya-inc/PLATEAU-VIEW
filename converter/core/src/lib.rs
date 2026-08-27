//! CityGML 2.0 -> 3.0 conversion for PLATEAU city models.
//!
//! The crate is split into four layers:
//!
//! * [`dataset`] resolves whatever the user handed us (a PLATEAU folder, a zip of
//!   one, or a loose set of per-part zips) into a single on-disk PLATEAU tree.
//! * [`xml`] is a small namespace-aware XML tree: it streams a CityGML document
//!   and hands out one fully-materialised subtree per top-level member, so a
//!   feature can be restructured freely without holding the whole file in memory.
//! * [`profile`] holds the declarative part of the mapping (namespace bumps,
//!   element renames, child ordering) loaded from a TOML profile.
//! * [`transform`] and [`bldg`] apply the profile and the structural rewrites
//!   that a rename table cannot express.
//!
//! [`convert`] ties them together and [`report`] carries the diagnostics back out.

pub mod bldg;
pub mod common;
pub mod convert;
pub mod dataset;
pub mod error;
pub mod profile;
pub mod report;
pub mod transform;
pub mod xml;

pub use error::{Error, Result};

/// The default conversion profile, compiled into the binary.
///
/// Override it with [`profile::Profile::from_toml`] to retarget namespaces or
/// add rules without rebuilding.
pub const DEFAULT_PROFILE: &str = include_str!("../../profiles/citygml-2.0-to-3.0.toml");
