resource "google_compute_region_network_endpoint_group" "editor_web" {
  project               = data.google_project.project.project_id
  name                  = var.name
  network_endpoint_type = "SERVERLESS"
  region                = var.region

  cloud_run {
    service = google_cloud_run_v2_service.editor_web.name
  }
}
