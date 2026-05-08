//! Filesystem-backed static asset serving for viewer HTML.
//!
//! Viewer pages used to be embedded with `include_str!` so the binary was
//! self-contained, but that meant every HTML tweak invalidated the entire
//! Rust compile + Docker image build. By reading from disk at request time
//! and copying the `static/` directory in a separate Docker layer (after
//! the much larger binary copy), an HTML-only change is a 30-second image
//! push instead of a multi-minute full rebuild.
//!
//! Reading on every request — rather than caching at startup — also gives
//! free hot-reload during local development: edit the HTML, refresh the
//! browser, no `cargo run` needed.

use std::path::{Path, PathBuf};

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

/// Directory holding viewer HTML files, resolved at every read so that
/// `STATIC_DIR` can be overridden without restarting in tests/dev.
pub fn static_dir() -> PathBuf {
    std::env::var("STATIC_DIR")
        .unwrap_or_else(|_| "static".to_string())
        .into()
}

/// Serve an HTML file from `static_dir()`. Logs and 500s on read failure
/// so a missing file shows up clearly in logs instead of panicking the
/// process.
pub fn serve_html(name: &str) -> Response {
    let path = static_dir().join(name);
    if !is_safe_name(name) {
        return (StatusCode::BAD_REQUEST, "invalid asset name").into_response();
    }
    match std::fs::read_to_string(&path) {
        Ok(s) => Html(s).into_response(),
        Err(e) => {
            tracing::error!(
                path = %path.display(),
                error = %e,
                "failed to read static HTML",
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("missing static asset: {name}"),
            )
                .into_response()
        }
    }
}

/// Defence in depth: callers pass fixed names, but reject anything with
/// path separators or parent-directory components just in case.
fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
        && Path::new(name)
            .file_name()
            .map(|n| n == name)
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal() {
        assert!(!is_safe_name(""));
        assert!(!is_safe_name("../etc/passwd"));
        assert!(!is_safe_name("foo/bar.html"));
        assert!(!is_safe_name("foo\\bar.html"));
        assert!(is_safe_name("viewer.html"));
        assert!(is_safe_name("terrain_viewer.html"));
    }
}
