resource "google_compute_region_network_endpoint_group" "tile" {
  project               = data.google_project.project.project_id
  name                  = "tile-neg"
  network_endpoint_type = "SERVERLESS"
  region                = var.gcp_region
  cloud_run {
    service = google_cloud_run_v2_service.tile.name
  }
}
