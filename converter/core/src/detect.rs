//! Choosing a conversion profile from the document itself.
//!
//! i-UR puts its minor version in the namespace URI, so a CityGML file states
//! which version it uses simply by declaring it. There is one profile per source
//! version, and picking between them is therefore a lookup rather than a
//! question for the user.
//!
//! The point of doing this in the converter rather than leaving it to the caller
//! is the failure mode it removes: running the 3.0 profile over 3.1 data
//! converts the CityGML half and silently leaves every `uro:` element behind,
//! which looks like a successful run.

use crate::error::{Error, Result};
use crate::profile::Rules;

/// Which profile fits a document, and why.
#[derive(Debug, Clone)]
pub struct Detection {
    /// Index into the candidate list.
    pub index: usize,
    /// The source namespaces that decided it. Empty when the document declares
    /// no i-UR at all.
    pub matched: Vec<String>,
}

/// Picks the profile whose `[source]` matches the namespaces a document declares.
///
/// A document that declares no i-UR namespace at all matches any profile — there
/// is nothing to tell them apart — so the first candidate is used and `matched`
/// comes back empty for the caller to report.
pub fn select(candidates: &[Rules], declared: &[String]) -> Result<Detection> {
    if candidates.is_empty() {
        return Err(Error::Profile("no profiles to choose from".into()));
    }

    // Refuse input that is already converted before blaming the i-UR version for
    // the mismatch: it is the likelier mistake and the confusing one to debug.
    for rules in candidates {
        if let Some(target) = &rules.target().citygml
            && declared.iter().any(|ns| ns == target)
        {
            return Err(Error::Unsupported(format!(
                "the input already declares {target}; it is not CityGML 2.0"
            )));
        }
    }

    let present: Vec<String> = declared
        .iter()
        .filter(|ns| {
            candidates
                .iter()
                .any(|r| r.source().iur.iter().any(|s| s == *ns))
        })
        .cloned()
        .collect();

    if present.is_empty() {
        return Ok(Detection {
            index: 0,
            matched: Vec::new(),
        });
    }

    let fits: Vec<usize> = candidates
        .iter()
        .enumerate()
        .filter(|(_, r)| present.iter().all(|ns| r.source().iur.contains(ns)))
        .map(|(i, _)| i)
        .collect();

    match fits.as_slice() {
        [index] => Ok(Detection {
            index: *index,
            matched: present,
        }),
        // Several i-UR versions in one document. The schemas import each other
        // by exact namespace, so there is no version pairing that could be
        // right; converting it would have to invent one.
        [] => Err(Error::Unsupported(format!(
            "the input mixes i-UR versions ({}), which no profile converts; \
             every module in a package must be the same version",
            present.join(", ")
        ))),
        // With more than one target version in play this is not a defect, it is
        // an unanswered question: the data says what it is, never what it
        // should become.
        many => Err(Error::Unsupported(format!(
            "{} profiles accept this input, producing i-UR {}; choose one",
            many.len(),
            many.iter()
                .map(|i| candidates[*i].target().iur_version.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// The i-UR versions the given profiles can produce, in profile order.
pub fn target_versions(candidates: &[Rules]) -> Vec<&str> {
    let mut seen: Vec<&str> = Vec::new();
    for rules in candidates {
        let version = rules.target().iur_version.as_str();
        if !version.is_empty() && !seen.contains(&version) {
            seen.push(version);
        }
    }
    seen
}

/// Keeps only the profiles producing i-UR `version`.
///
/// Selecting a target is the one part of the choice the data cannot make: an
/// input says which version it *is*, never which one it should become. Today
/// every profile targets 4.0, so this narrows nothing -- it exists so that
/// adding a target is a profile to generate rather than a CLI to redesign.
pub fn with_target(candidates: Vec<Rules>, version: &str) -> Result<Vec<Rules>> {
    let available = target_versions(&candidates).join(", ");
    let kept: Vec<Rules> = candidates
        .into_iter()
        .filter(|r| r.target().iur_version == version)
        .collect();
    if kept.is_empty() {
        return Err(Error::Unsupported(format!(
            "no profile produces i-UR {version}; available: {available}"
        )));
    }
    Ok(kept)
}

/// Checks that a profile the caller chose explicitly actually fits the document.
///
/// `--profile` is an override, so this reports rather than refuses when the
/// profile declares no `[source]` at all: a hand-written profile is allowed to
/// say nothing about what it accepts.
pub fn check(rules: &Rules, declared: &[String]) -> Option<String> {
    let source = rules.source();
    if source.iur.is_empty() {
        return None;
    }
    let foreign: Vec<&String> = declared
        .iter()
        .filter(|ns| ns.starts_with("https://www.geospatial.jp/iur/"))
        .filter(|ns| !source.iur.contains(ns))
        .collect();
    if foreign.is_empty() {
        return None;
    }
    Some(format!(
        "profile `{}` accepts {} but the input declares {}; those elements will \
         pass through unconverted",
        rules.name(),
        source.label,
        foreign
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PROFILES;

    fn candidates() -> Vec<Rules> {
        PROFILES
            .iter()
            .map(|(_, toml)| Rules::from_toml(toml).unwrap())
            .collect()
    }

    fn iur(module: &str, version: &str) -> String {
        format!("https://www.geospatial.jp/iur/{module}/{version}")
    }

    #[test]
    fn picks_the_profile_matching_the_declared_version() {
        let c = candidates();
        for version in ["3.0", "3.1", "3.2"] {
            let declared = vec![
                "http://www.opengis.net/citygml/2.0".to_string(),
                iur("uro", version),
            ];
            let found = select(&c, &declared).unwrap();
            assert_eq!(c[found.index].name(), format!("iur-{version}-to-4.0"));
        }
    }

    /// Several modules of the same version still resolve to one profile.
    #[test]
    fn several_modules_of_one_version_agree() {
        let c = candidates();
        let declared = vec![iur("uro", "3.1"), iur("urf", "3.1"), iur("urt", "3.1")];
        let found = select(&c, &declared).unwrap();
        assert_eq!(c[found.index].name(), "iur-3.1-to-4.0");
        assert_eq!(found.matched.len(), 3);
    }

    #[test]
    fn mixed_versions_are_refused() {
        let c = candidates();
        let declared = vec![iur("uro", "3.1"), iur("urf", "3.2")];
        let error = select(&c, &declared).unwrap_err().to_string();
        assert!(error.contains("mixes i-UR versions"), "{error}");
    }

    #[test]
    fn an_unknown_version_is_refused() {
        let c = candidates();
        // i-UR 2.0 is real but unsupported; it must not fall back to a near miss.
        let declared = vec![iur("uro", "2.0")];
        let found = select(&c, &declared).unwrap();
        assert!(
            found.matched.is_empty(),
            "an unrecognised i-UR version must not count as a match"
        );
    }

    #[test]
    fn already_converted_input_is_refused() {
        let c = candidates();
        let declared = vec!["http://www.opengis.net/citygml/3.0".to_string()];
        let error = select(&c, &declared).unwrap_err().to_string();
        assert!(error.contains("not CityGML 2.0"), "{error}");
    }

    #[test]
    fn a_document_without_iur_matches_anything() {
        let c = candidates();
        let declared = vec!["http://www.opengis.net/citygml/2.0".to_string()];
        let found = select(&c, &declared).unwrap();
        assert!(found.matched.is_empty());
    }

    #[test]
    fn the_targets_on_offer_are_read_from_the_profiles() {
        // One target today. The point of the flag is that adding another is a
        // profile to write, not a code change -- so this reads, never hardcodes.
        assert_eq!(target_versions(&candidates()), ["4.0"]);
    }

    #[test]
    fn narrowing_to_the_available_target_keeps_every_profile() {
        let kept = with_target(candidates(), "4.0").unwrap();
        assert_eq!(kept.len(), PROFILES.len());
    }

    #[test]
    fn narrowing_to_a_target_nothing_produces_is_an_error() {
        let error = with_target(candidates(), "4.1").unwrap_err().to_string();
        assert!(error.contains("no profile produces i-UR 4.1"), "{error}");
        assert!(error.contains("available: 4.0"), "{error}");
    }

    /// Narrowing by target must not disturb source detection.
    #[test]
    fn a_target_and_a_source_resolve_together() {
        let kept = with_target(candidates(), "4.0").unwrap();
        let declared = vec![iur("uro", "3.2")];
        let found = select(&kept, &declared).unwrap();
        assert_eq!(kept[found.index].name(), "iur-3.2-to-4.0");
    }

    #[test]
    fn check_reports_a_profile_used_on_the_wrong_version() {
        let c = candidates();
        let profile = c.iter().find(|r| r.name() == "iur-3.0-to-4.0").unwrap();
        let warning = check(profile, &[iur("uro", "3.2")]).expect("a mismatch must be reported");
        assert!(warning.contains("uro/3.2"), "{warning}");
        assert!(check(profile, &[iur("uro", "3.0")]).is_none());
    }
}
