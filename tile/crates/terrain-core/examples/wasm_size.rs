//! Size probe, not an API: a `cdylib` whose exports keep every module of
//! `terrain-core` reachable, so CI can print what the core costs in a
//! wasm32 release build. The real WebAssembly bindings are a separate crate.
//!
//! `cargo build -p terrain-core --example wasm_size --release --target wasm32-unknown-unknown`

use terrain_core::hexfloat;
use terrain_core::paint::paint_over;
use terrain_core::positions::{AxisPositions, PixelPositions, TileSpace, Upsampled, mercator_lat};
use terrain_core::resample::{extract_and_upsample, fit_to_tile_size, resample_bilinear};
use terrain_core::sanitize::sanitize_height;
use terrain_core::vertical::{DhMemo, DhSelection, SparseGrid, mesh6_of};

fn axis(src: u32, dst: u32) -> AxisPositions {
    if src == dst {
        AxisPositions::Centres { n: dst }
    } else {
        AxisPositions::Resampled { src, dst }
    }
}

/// Latitude of element `(i, j)` of tile `z/x/y` for a tile resampled from
/// `src` px to `n` px, optionally upsampled by `factor`.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn tc_lonlat_lat(
    linear: u32,
    src: u32,
    n: u32,
    factor: u32,
    z: u32,
    x: u32,
    y: u32,
    i: u32,
    j: u32,
) -> f64 {
    let p = PixelPositions {
        space: if linear != 0 {
            TileSpace::LinearLatLon
        } else {
            TileSpace::Mercator
        },
        x: axis(src, n),
        y: axis(src, n),
        upsampled: (factor > 1).then_some(Upsampled {
            factor,
            sub_x: x % factor.max(1),
            sub_y: y % factor.max(1),
            tile_size: n,
        }),
    };
    p.lonlat(z as u8, x, y, i, j).1 + mercator_lat(y as f64, (1u64 << (z % 32)) as f64)
}

/// Resample / upsample / paint / sanitise a synthetic tile and return a
/// checksum.
#[unsafe(no_mangle)]
pub extern "C" fn tc_resample(src: u32, n: u32, zoom_diff: u32) -> f64 {
    let grid: Vec<f64> = (0..src * src).map(|k| (k % 97) as f64).collect();
    let (mut tile, _) = fit_to_tile_size(grid, src, src, n, None);
    let up = extract_and_upsample(&tile, n, zoom_diff as u8, 0, 0);
    paint_over(&mut tile, &up);
    let again = resample_bilinear(&tile, n, n, src, src);
    again.iter().map(|v| sanitize_height(*v)).sum()
}

/// ΔH over a synthetic two-grid product, through the per-request memo.
#[unsafe(no_mangle)]
pub extern "C" fn tc_dh(z: u32, x: u32, y: u32, n: u32, i: u32, j: u32) -> f64 {
    let mk = |v: f64| SparseGrid {
        width: 64,
        height: 64,
        x0: 122.0,
        y0: 46.0,
        dx: 1.0 / 80.0,
        dy: 1.0 / 120.0,
        nodes: (0..64)
            .flat_map(|r| (0..64).map(move |c| ((r, c), v + (r * c) as f64 * 1e-4)))
            .collect(),
    };
    let (bm, tr) = (mk(0.1), mk(-0.2));
    let sel = DhSelection::ListedMeshes {
        primary: &bm,
        listed: &tr,
        fallback: &tr,
        meshes: [473121i64].into_iter().collect(),
    };
    let mut memo = DhMemo::default();
    let dh = memo
        .grid(PixelPositions::centres(n), z as u8, x, y, n)
        .get(&sel, i as usize, j as usize);
    dh + mesh6_of(dh, dh) as f64
}

/// Hex-float round trip.
#[unsafe(no_mangle)]
pub extern "C" fn tc_hex(v: f64) -> f64 {
    hexfloat::parse(&hexfloat::format(v)).unwrap_or(f64::NAN)
}
