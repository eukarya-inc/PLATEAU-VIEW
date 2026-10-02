//! Per-source credit line for the terrain metadata documents.
//!
//! `/terrain/{source}/layer.json`, `/terrarium/{source}/tilejson.json` and
//! `/mapbox/{source}/tilejson.json` carry an `attribution`. It is derived from
//! what the DEM source is actually built from, so it cannot drift from the
//! config:
//!
//! | part | credited when |
//! |------|---------------|
//! | PLATEAU | always (the publisher of every source) |
//! | base | the source sits on the shared `DEM_URL` base and that base has a credit: `DEM_ATTRIBUTION`, else Mapterhorn for a Mapterhorn template / PMTiles base; the sea-level base has none |
//! | layers | each overlay layer's own `attribution`, bottom → top, if it declares one |
//! | 国土地理院 | always: every source applies a GSI geoid model. With the qualifier [`GSI_CORRECTED_CREDIT`] when the source applies a GSI height-correction product |
//!
//! Parts are joined with ` | ` and exact duplicates are dropped. A source's
//! own `attribution` in the config JSON replaces the derived line entirely.
//!
//! The credit is metadata only: it never enters a tile body, an ETag or a
//! cache key.

/// PLATEAU (the publisher) — also the `/tiles` TileJSON default.
pub const PLATEAU_CREDIT: &str =
    r#"<a href="https://www.mlit.go.jp/plateau/" target="_blank">PLATEAU</a>"#;

/// Mapterhorn, the default shared base (Japan built on GSI elevation data).
pub const MAPTERHORN_CREDIT: &str =
    r#"<a href="https://mapterhorn.com/" target="_blank">Mapterhorn</a>"#;

/// 国土地理院 — the geoid model every source applies, and (by default) the
/// elevation data under the PLATEAU COG overlays.
pub const GSI_CREDIT: &str = r#"<a href="https://www.gsi.go.jp/" target="_blank">国土地理院</a>"#;

/// 国土地理院, for a source that applies a GSI height-correction product
/// (標高補正パラメータ) to the elevation data. GSI's terms ask processed data
/// to say it was processed; this wording is a placeholder pending review.
pub const GSI_CORRECTED_CREDIT: &str = r#"<a href="https://www.gsi.go.jp/" target="_blank">国土地理院</a>（基盤地図情報 数値標高モデル、標高補正パラメータを適用して加工）"#;

/// Separator between credits, as in the historical line.
const SEPARATOR: &str = " | ";

/// What a DEM source is built from, as far as credit is concerned.
#[derive(Debug, Default, Clone, Copy)]
pub struct SourceCredits<'a> {
    /// The source's own `attribution` from the config JSON. Non-blank wins
    /// over everything below.
    pub override_line: Option<&'a str>,
    /// Credit of the base under the layers: the shared base's credit, or
    /// `None` for the sea-level base (or a shared base without one).
    pub base: Option<&'a str>,
    /// Each overlay layer's own `attribution`, bottom → top.
    pub layers: &'a [Option<&'a str>],
    /// Whether a height-correction product is applied to any member.
    pub height_corrected: bool,
}

impl SourceCredits<'_> {
    /// The credit line for this source.
    pub fn line(&self) -> String {
        if let Some(line) = non_blank(self.override_line) {
            return line.to_string();
        }
        let gsi = if self.height_corrected {
            GSI_CORRECTED_CREDIT
        } else {
            GSI_CREDIT
        };
        let mut parts: Vec<&str> = vec![PLATEAU_CREDIT];
        parts.extend(non_blank(self.base));
        parts.extend(self.layers.iter().filter_map(|l| non_blank(*l)));
        parts.push(gsi);
        let mut seen = Vec::with_capacity(parts.len());
        for p in parts {
            if !seen.contains(&p) {
                seen.push(p);
            }
        }
        seen.join(SEPARATOR)
    }
}

/// Credit of the shared `DEM_URL` base: `DEM_ATTRIBUTION` when set, nothing for
/// the sea-level base, else Mapterhorn — the only upstream the server reads as
/// a `{z}/{x}/{y}` template, and the one `scripts/japan-pmtiles` mirrors into
/// PMTiles.
pub fn shared_base_credit(dem_url_is_sea_level: bool, configured: Option<&str>) -> Option<String> {
    if dem_url_is_sea_level {
        return None;
    }
    Some(
        non_blank(configured)
            .unwrap_or(MAPTERHORN_CREDIT)
            .to_string(),
    )
}

fn non_blank(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The line every source served before attribution became per source.
    const LEGACY: &str = r#"<a href="https://www.mlit.go.jp/plateau/" target="_blank">PLATEAU</a> | <a href="https://mapterhorn.com/" target="_blank">Mapterhorn</a> | <a href="https://www.gsi.go.jp/" target="_blank">国土地理院</a>"#;

    #[test]
    fn shared_mapterhorn_base_keeps_the_legacy_line() {
        let base = shared_base_credit(false, None);
        let line = SourceCredits {
            base: base.as_deref(),
            layers: &[None, None],
            ..Default::default()
        }
        .line();
        assert_eq!(line, LEGACY);
    }

    #[test]
    fn sea_level_base_with_correction_drops_mapterhorn() {
        let line = SourceCredits {
            base: None,
            layers: &[None],
            height_corrected: true,
            ..Default::default()
        }
        .line();
        assert_eq!(line, format!("{PLATEAU_CREDIT} | {GSI_CORRECTED_CREDIT}"));
        assert!(!line.contains("Mapterhorn"));
    }

    #[test]
    fn layer_credits_are_ordered_and_deduplicated() {
        let line = SourceCredits {
            base: Some(MAPTERHORN_CREDIT),
            layers: &[
                Some("A"),
                None,
                Some(" "),
                Some("B"),
                Some("A"),
                Some(GSI_CREDIT),
            ],
            ..Default::default()
        }
        .line();
        assert_eq!(
            line,
            format!("{PLATEAU_CREDIT} | {MAPTERHORN_CREDIT} | A | B | {GSI_CREDIT}")
        );
    }

    #[test]
    fn override_replaces_the_derived_line() {
        let c = SourceCredits {
            override_line: Some("  custom  "),
            base: Some(MAPTERHORN_CREDIT),
            height_corrected: true,
            ..Default::default()
        };
        assert_eq!(c.line(), "custom");
        // A blank override is ignored.
        let c = SourceCredits {
            override_line: Some(" "),
            ..c
        };
        assert!(c.line().starts_with(PLATEAU_CREDIT));
    }

    #[test]
    fn shared_base_credit_resolution() {
        assert_eq!(shared_base_credit(true, Some("X")), None);
        assert_eq!(
            shared_base_credit(false, None).as_deref(),
            Some(MAPTERHORN_CREDIT)
        );
        assert_eq!(
            shared_base_credit(false, Some("")).as_deref(),
            Some(MAPTERHORN_CREDIT)
        );
        assert_eq!(shared_base_credit(false, Some("X")).as_deref(), Some("X"));
    }
}
