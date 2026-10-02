//! # terrain-core
//!
//! The pure, synchronous part of the PLATEAU terrain pipeline, shared by the
//! tile server (`tile`, native) and the Cloudflare Worker (WebAssembly).
//!
//! Everything here is plain `std` computation on values already in memory:
//! no I/O, no async runtime, no clocks, no global state. Functions never
//! panic on their input — malformed sizes or coordinates give NaN / empty
//! results instead — because a panic in a WebAssembly instance poisons it
//! for every later request.
//!
//! The server keeps its historical paths (`tile::terrain::PixelPositions`,
//! `tile::terrain::resample_bilinear`, `tile::terrain::vertical::sample`, …)
//! as re-exports of the items below, so the code moved here is the code the
//! server runs: one implementation, two targets.
//!
//! - [`positions`]: where a DEM tile's elevations were evaluated.
//! - [`resample`]: DEM tile resizing and upsampling above the DEM max zoom.
//! - [`vertical`]: the ΔH (vertical datum correction) sampler.
//! - [`sanitize`]: the invalid-elevation policy of every terrain encoding.
//! - [`paint`]: the per-pixel overlay composite.
//! - [`hexfloat`]: exact text form of `f64` values for fixtures.

#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

/// An empty `Vec` with room for `len` elements, or `None` when that cannot
/// be allocated (a capacity overflow, or memory the instance cannot grow
/// to). The infallible `Vec::with_capacity` / `vec![…; len]` would trap
/// instead, which poisons a WebAssembly instance for later requests.
pub(crate) fn try_vec<T>(len: usize) -> Option<Vec<T>> {
    let mut v = Vec::new();
    v.try_reserve_exact(len).ok()?;
    Some(v)
}

pub mod hexfloat;
pub mod paint;
pub mod positions;
pub mod resample;
pub mod sanitize;
pub mod vertical;
