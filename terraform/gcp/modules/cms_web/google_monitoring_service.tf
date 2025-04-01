resource "google_monitoring_service" "cms_web" {
  project      = data.google_project.project.project_id
  service_id   = "reearth-cms-web-${var.region}" # This has to be unique across the project so we add the region suffix.
  display_name = "reearth-cms-web"

  basic_service {
    service_type = "CLOUD_RUN"
    service_labels = {
      location     = google_cloud_run_v2_service.cms_web.location
      service_name = google_cloud_run_v2_service.cms_web.name
    }
  }
}
