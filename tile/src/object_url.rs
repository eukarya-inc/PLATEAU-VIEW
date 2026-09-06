//! Shared URL → object-store key conversion.
//!
//! Used by every module that turns a source/cache URL into an
//! `object_store` key (`cache::store`, `cog`/`pmtiles` tile sources and the
//! terrain DEM providers), so the percent-encoding rule lives in one place.

use object_store::path::{Error as PathError, Path as ObjectPath};
use url::Url;

/// Build the object-store key for a URL.
///
/// `Url::path()` is **percent-encoded**, and `ObjectPath::from` percent-encodes
/// again (its encode set includes `%`), so a key with non-ASCII characters
/// (e.g. `（）` or `・`) ends up double-encoded and every GET/HEAD 404s.
/// `ObjectPath::from_url_path` decodes first, and `object_store` then encodes
/// exactly once when it builds the request URL.
///
/// A leading `/` is stripped, so this is also usable for bucket prefixes.
pub fn object_path_from_url(url: &Url) -> Result<ObjectPath, PathError> {
    ObjectPath::from_url_path(url.path())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_ascii_url_path_is_single_encoded() {
        let url = Url::parse("https://example.com/patch/静岡市（葵区）・DEM.tif").unwrap();
        assert!(url.path().contains('%'));

        let path = object_path_from_url(&url).unwrap();
        assert_eq!(path.to_string(), "patch/静岡市（葵区）・DEM.tif");
        assert!(!path.to_string().contains('%'));
        // The old `ObjectPath::from(url.path())` form kept the escapes and
        // re-encoded the `%`, which is what produced the 404s.
        assert_ne!(path, ObjectPath::from(url.path()));
    }

    #[test]
    fn ascii_url_path_unchanged() {
        let url = Url::parse("gs://bucket/base/dem5/5238.tif").unwrap();
        let path = object_path_from_url(&url).unwrap();
        assert_eq!(path.to_string(), "base/dem5/5238.tif");
    }

    #[test]
    fn empty_path_is_root() {
        let url = Url::parse("gs://bucket").unwrap();
        assert_eq!(object_path_from_url(&url).unwrap().to_string(), "");
    }
}
