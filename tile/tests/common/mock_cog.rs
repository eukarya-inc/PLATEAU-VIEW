//! Mock COG server for serving local files via HTTP with range request support.

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use super::fixtures;

/// A mock COG file server that supports HTTP range requests.
#[allow(dead_code)]
pub struct MockCogServer {
    port: u16,
    base_url: String,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

#[derive(Clone)]
#[allow(dead_code)]
struct ServerState {
    fixtures_dir: PathBuf,
}

#[allow(dead_code)]
impl MockCogServer {
    /// Start a new mock COG server.
    pub async fn start() -> Self {
        let port = portpicker::pick_unused_port().expect("No available port");
        let addr = format!("127.0.0.1:{port}");
        let base_url = format!("http://{addr}");

        let state = ServerState {
            fixtures_dir: fixtures::fixtures_dir(),
        };

        let app = Router::new()
            .route("/{filename}", get(serve_file))
            .with_state(Arc::new(state));

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let listener = TcpListener::bind(&addr)
            .await
            .expect("Failed to bind listener");

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .ok();
        });

        // Wait for server to be ready
        let client = reqwest::Client::new();
        for _ in 0..50 {
            if client.head(&base_url).send().await.is_ok() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        Self {
            port,
            base_url,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// Get the URL for a COG file.
    pub fn cog_url(&self, filename: &str) -> String {
        format!("{}/{filename}", self.base_url)
    }

    /// Get the port the server is running on.
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for MockCogServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Handler for serving files with range request support.
#[allow(dead_code)]
async fn serve_file(
    State(state): State<Arc<ServerState>>,
    Path(filename): Path<String>,
    headers: HeaderMap,
) -> Response {
    let file_path = state.fixtures_dir.join(&filename);

    if !file_path.exists() {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    }

    let mut file = match File::open(&file_path).await {
        Ok(f) => f,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to open file").into_response();
        }
    };

    let metadata = match file.metadata().await {
        Ok(m) => m,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get metadata").into_response();
        }
    };

    let file_size = metadata.len();

    // Check for Range header
    if let Some(range_header) = headers.get(header::RANGE)
        && let Ok(range_str) = range_header.to_str()
        && let Some(range) = parse_range(range_str, file_size)
    {
        let (start, end) = range;
        let length = end - start + 1;

        if file.seek(std::io::SeekFrom::Start(start)).await.is_err() {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to seek").into_response();
        }

        let mut buffer = vec![0u8; length as usize];
        if file.read_exact(&mut buffer).await.is_err() {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read").into_response();
        }

        return (
            StatusCode::PARTIAL_CONTENT,
            [
                (header::CONTENT_TYPE, "image/tiff"),
                (header::ACCEPT_RANGES, "bytes"),
                (
                    header::CONTENT_RANGE,
                    &format!("bytes {start}-{end}/{file_size}"),
                ),
            ],
            buffer,
        )
            .into_response();
    }

    // No range header - return full file
    let mut buffer = Vec::new();
    if file.read_to_end(&mut buffer).await.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response();
    }

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/tiff"),
            (header::ACCEPT_RANGES, "bytes"),
            (header::CONTENT_LENGTH, &file_size.to_string()),
        ],
        buffer,
    )
        .into_response()
}

/// Parse HTTP Range header.
/// Supports: bytes=0-499, bytes=500-, bytes=-500
fn parse_range(range: &str, file_size: u64) -> Option<(u64, u64)> {
    let range = range.strip_prefix("bytes=")?;

    if let Some(suffix) = range.strip_prefix('-') {
        // bytes=-500 (last 500 bytes)
        let suffix_len: u64 = suffix.parse().ok()?;
        let start = file_size.saturating_sub(suffix_len);
        Some((start, file_size - 1))
    } else if let Some((start_str, end_str)) = range.split_once('-') {
        let start: u64 = start_str.parse().ok()?;
        let end = if end_str.is_empty() {
            // bytes=500- (from 500 to end)
            file_size - 1
        } else {
            // bytes=0-499
            end_str.parse().ok()?
        };
        if start <= end && end < file_size {
            Some((start, end))
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range() {
        // bytes=0-499
        assert_eq!(parse_range("bytes=0-499", 1000), Some((0, 499)));

        // bytes=500-
        assert_eq!(parse_range("bytes=500-", 1000), Some((500, 999)));

        // bytes=-500
        assert_eq!(parse_range("bytes=-500", 1000), Some((500, 999)));

        // Invalid ranges
        assert_eq!(parse_range("bytes=500-400", 1000), None); // start > end
        assert_eq!(parse_range("bytes=0-1000", 1000), None); // end >= file_size
    }
}
