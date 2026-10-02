//! Sizes that cannot be allocated must give empty / NaN results, not a
//! trap: a trap poisons the WebAssembly instance for every later request.
//! Runs natively and in wasm32, where the sizes below exceed what a 32-bit
//! address space can hold.

use terrain_core::positions::PixelPositions;
use terrain_core::resample::{resample_bilinear, upsample_subregion};
use terrain_core::vertical::{DhMemo, DhSelection, SparseGrid};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn unallocatable_sizes_do_not_trap() {
    // Only on 32-bit targets: a 64-bit host may well reserve these.
    if !cfg!(target_pointer_width = "32") {
        return;
    }
    let g = vec![1.0; 16];
    // u32::MAX elements of f64 = 32 GiB.
    assert!(resample_bilinear(&[], 1, 1, u32::MAX, 1).is_empty());
    assert!(resample_bilinear(&g, 4, 4, 65535, 65535).is_empty());
    assert!(upsample_subregion(&g, 65535, 2, 0, 0).is_empty());

    let grid = SparseGrid {
        width: 1,
        height: 1,
        x0: 0.0,
        y0: 0.0,
        dx: 1.0,
        dy: 1.0,
        nodes: Default::default(),
    };
    let sel = DhSelection::Single(&grid);
    let mut memo = DhMemo::default();
    let huge = memo.grid(PixelPositions::centres(u32::MAX), 0, 0, 0, u32::MAX);
    assert!(huge.get(&sel, 0, 0).is_nan());
}
