//! Per-pixel composite of DEM layers.

/// Paint `overlay` onto `base` per pixel. Where overlay is finite, it wins.
pub fn paint_over(base: &mut [f64], overlay: &[f64]) {
    let n = base.len().min(overlay.len());
    for i in 0..n {
        if overlay[i].is_finite() {
            base[i] = overlay[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::paint_over;

    #[test]
    fn finite_overlay_wins_and_lengths_may_differ() {
        let mut base = vec![1.0, 2.0, 3.0];
        paint_over(&mut base, &[f64::NAN, 20.0, f64::INFINITY, 40.0]);
        assert_eq!(base, vec![1.0, 20.0, 3.0]);
        paint_over(&mut base, &[]);
        assert_eq!(base, vec![1.0, 20.0, 3.0]);
    }
}
