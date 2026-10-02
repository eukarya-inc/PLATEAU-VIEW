//! Trig determinism probe (informational): how often do
//! `PixelPositions::lonlat` and `mercator_lat` — the only transcendental
//! steps in this crate (`sinh`, `atan`) — differ between a native build and
//! wasm32, and by how many ulps?
//!
//! 1. Native: `TRIG_PROBE_NATIVE=/path/native.bin cargo test -p terrain-core
//!    --release --test trig_probe -- --ignored --nocapture` writes the
//!    native results.
//! 2. wasm32: build the tests with the same `TRIG_PROBE_NATIVE` set
//!    (compile-time) and run them under Node; the wasm test reads the file
//!    and reports the count of non-identical values and the max ulp
//!    distance. Without the variable it only reports a digest.
//!
//! Nothing here fails on a difference: it measures, it does not gate.

use terrain_core::positions::{AxisPositions, PixelPositions, TileSpace, Upsampled, mercator_lat};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[cfg(target_arch = "wasm32")]
macro_rules! report { ($($t:tt)*) => { wasm_bindgen_test::console_log!($($t)*) } }
#[cfg(not(target_arch = "wasm32"))]
macro_rules! report { ($($t:tt)*) => { eprintln!($($t)*) } }

const CASES: usize = 1_000_000;
/// Per case: lonlat().0, lonlat().1, mercator_lat() as little-endian f64.
const RECORD: usize = 24;

struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// Uniform in `0..n` (`n > 0`).
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    /// Uniform in `[0, 1)`.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

const SIZES: [u32; 6] = [256, 512, 65, 100, 7, 1024];

fn pick(r: &mut SplitMix) -> u32 {
    SIZES[r.below(SIZES.len() as u64) as usize]
}

/// An axis of an `n`-pixel tile: native centres, or resampled from any
/// decoded size (down or up).
fn axis(r: &mut SplitMix, n: u32) -> AxisPositions {
    if r.below(2) == 0 {
        AxisPositions::Centres { n }
    } else {
        AxisPositions::Resampled {
            src: pick(r),
            dst: n,
        }
    }
}

/// Deterministic inputs covering z 0–20, both tile spaces, every
/// `AxisPositions` variant (centres, down- and up-resampled) and upsampled
/// tiles, plus a `mercator_lat` argument per case.
fn compute() -> Vec<u8> {
    let mut r = SplitMix(0x7e44_a1c0_2026_1002);
    let mut out = Vec::with_capacity(CASES * RECORD);
    for _ in 0..CASES {
        let z = r.below(21) as u8;
        let tiles = 1u64 << z;
        let x = r.below(tiles) as u32;
        let y = r.below(tiles) as u32;
        let space = if r.below(4) == 0 {
            TileSpace::LinearLatLon
        } else {
            TileSpace::Mercator
        };
        // Served tile size; an upsampled tile's parent grid has the same.
        let n = pick(&mut r);
        let (xa, ya) = (axis(&mut r, n), axis(&mut r, n));
        let upsampled = if z > 0 && r.below(3) == 0 {
            let dz = 1 + r.below(z.min(8) as u64) as u32;
            let factor = 1u32 << dz;
            Some(Upsampled {
                factor,
                sub_x: r.below(factor as u64) as u32,
                sub_y: r.below(factor as u64) as u32,
                tile_size: n,
            })
        } else {
            None
        };
        let p = PixelPositions {
            space,
            x: xa,
            y: ya,
            upsampled,
        };
        let i = r.below(n as u64) as u32;
        let j = r.below(n as u64) as u32;
        let (lon, lat) = p.lonlat(z, x, y, i, j);
        let ty = r.unit() * tiles as f64;
        let m = mercator_lat(ty, tiles as f64);
        for v in [lon, lat, m] {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    out
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Distance in ulps between two finite doubles (sign-aware).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn ulps(a: f64, b: f64) -> u64 {
    let ord = |v: f64| {
        let b = v.to_bits() as i64;
        if b < 0 { i64::MIN - b } else { b }
    };
    (ord(a) as i128 - ord(b) as i128).unsigned_abs() as u64
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn compare(native: &[u8], here: &[u8]) {
    assert_eq!(native.len(), here.len(), "probe size mismatch");
    let names = ["lonlat.lon", "lonlat.lat", "mercator_lat"];
    let mut diff = [0usize; 3];
    let mut max_ulp = [0u64; 3];
    let mut max_abs = [0f64; 3];
    let mut any_lonlat = 0usize;
    for (a, b) in native.chunks_exact(RECORD).zip(here.chunks_exact(RECORD)) {
        let mut ll = false;
        for k in 0..3 {
            let get = |s: &[u8]| {
                let mut w = [0u8; 8];
                w.copy_from_slice(&s[k * 8..k * 8 + 8]);
                f64::from_le_bytes(w)
            };
            let (x, y) = (get(a), get(b));
            if x.to_bits() != y.to_bits() {
                diff[k] += 1;
                max_ulp[k] = max_ulp[k].max(ulps(x, y));
                max_abs[k] = max_abs[k].max((x - y).abs());
                ll |= k < 2;
            }
        }
        any_lonlat += ll as usize;
    }
    for k in 0..3 {
        report!(
            "trig-probe {}: {}/{CASES} non-identical, max {} ulp, max |Δ| {:e}°",
            names[k],
            diff[k],
            max_ulp[k],
            max_abs[k]
        );
    }
    report!(
        "trig-probe PixelPositions::lonlat: {any_lonlat}/{CASES} cases with any component non-identical"
    );
}

/// Native half: write the results to `$TRIG_PROBE_NATIVE`.
#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "informational; writes $TRIG_PROBE_NATIVE (see module docs)"]
fn trig_probe_native() {
    let out = compute();
    report!(
        "trig-probe native [{}-{}]: {CASES} cases, fnv1a {:016x}",
        std::env::consts::ARCH,
        std::env::consts::OS,
        fnv1a(&out)
    );
    if let Ok(path) = std::env::var("TRIG_PROBE_NATIVE") {
        std::fs::write(&path, &out).unwrap();
        report!("trig-probe native: wrote {path}");
    }
}

#[cfg(target_arch = "wasm32")]
mod node_fs {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(module = "fs")]
    extern "C" {
        #[wasm_bindgen(js_name = readFileSync)]
        pub fn read_file_sync(path: &str) -> Vec<u8>;
    }
}

/// wasm32 half: compare against the native results baked in at build time.
#[cfg(target_arch = "wasm32")]
#[test]
fn trig_probe_wasm() {
    let out = compute();
    report!(
        "trig-probe wasm32: {CASES} cases, fnv1a {:016x}",
        fnv1a(&out)
    );
    match option_env!("TRIG_PROBE_NATIVE") {
        Some(path) => compare(&node_fs::read_file_sync(path), &out),
        None => {
            report!("trig-probe wasm32: TRIG_PROBE_NATIVE not set at build time; no comparison")
        }
    }
}
