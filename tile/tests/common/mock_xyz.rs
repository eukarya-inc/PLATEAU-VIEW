//! Mock XYZ tile server using wiremock.

use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures;

/// A mock XYZ tile server for testing.
pub struct MockXyzServer {
    server: MockServer,
}

#[allow(dead_code)]
impl MockXyzServer {
    /// Start a new mock XYZ server.
    pub async fn start() -> Self {
        let server = MockServer::start().await;
        Self { server }
    }

    /// Get the base URL of the mock server.
    pub fn url(&self) -> String {
        self.server.uri()
    }

    /// Get a URL template for XYZ tiles.
    pub fn xyz_url_template(&self) -> String {
        format!("{}/{{z}}/{{x}}/{{y}}.png", self.url())
    }

    /// Mount a mock that returns a solid color tile for all requests.
    pub async fn mock_all_tiles_with_color(&self, color: [u8; 4]) {
        let png = fixtures::create_solid_png(256, 256, color);
        Mock::given(method("GET"))
            .and(path_regex(r"^/\d+/\d+/\d+\.png$"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(png)
                    .insert_header("content-type", "image/png"),
            )
            .mount(&self.server)
            .await;
    }

    /// Mount a mock that returns red tiles for all requests.
    pub async fn mock_all_red_tiles(&self) {
        self.mock_all_tiles_with_color([255, 0, 0, 255]).await;
    }

    /// Mount a mock that returns green tiles for all requests.
    pub async fn mock_all_green_tiles(&self) {
        self.mock_all_tiles_with_color([0, 255, 0, 255]).await;
    }

    /// Mount a mock that returns 404 for all tile requests.
    pub async fn mock_all_tiles_not_found(&self) {
        Mock::given(method("GET"))
            .and(path_regex(r"^/\d+/\d+/\d+\.png$"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for a specific tile coordinate.
    pub async fn mock_tile(&self, z: u32, x: u32, y: u32, png_data: Vec<u8>) {
        let path = format!("/{z}/{x}/{y}.png");
        Mock::given(method("GET"))
            .and(wiremock::matchers::path(path))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(png_data)
                    .insert_header("content-type", "image/png"),
            )
            .mount(&self.server)
            .await;
    }

    /// Mount a mock for a specific tile coordinate that returns 404.
    pub async fn mock_tile_not_found(&self, z: u32, x: u32, y: u32) {
        let path = format!("/{z}/{x}/{y}.png");
        Mock::given(method("GET"))
            .and(wiremock::matchers::path(path))
            .respond_with(ResponseTemplate::new(404))
            .mount(&self.server)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_xyz_server() {
        let server = MockXyzServer::start().await;
        server.mock_all_red_tiles().await;

        let client = reqwest::Client::new();
        let url = format!("{}/10/909/403.png", server.url());
        let response = client.get(&url).send().await.unwrap();

        assert_eq!(response.status(), 200);
        let bytes = response.bytes().await.unwrap();
        assert!(!bytes.is_empty());
    }
}
