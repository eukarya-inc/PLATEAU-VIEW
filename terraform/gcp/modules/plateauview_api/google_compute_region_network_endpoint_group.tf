resource "google_compute_region_network_endpoint_group" "plateauview_api" {
  project               = data.google_project.project.project_id
  name                  = "plateau-api-neg"
  network_endpoint_type = "SERVERLESS"
  region                = var.gcp_region
  cloud_run {
    service = google_cloud_run_v2_service.plateauview_api.name
  }
}
