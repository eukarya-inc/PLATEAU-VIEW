resource "google_compute_backend_service" "editor_api" {
  project = data.google_project.project.project_id

  affinity_cookie_ttl_sec         = 0
  enable_cdn                      = false
  connection_draining_timeout_sec = 300
  load_balancing_scheme           = "EXTERNAL_MANAGED"
  name                            = "plateauview-reearth-visualizer-backend"
  port_name                       = "http"
  protocol                        = "HTTPS"
  session_affinity                = "NONE"
  timeout_sec                     = "30"

  backend {
    balancing_mode               = "UTILIZATION"
    capacity_scaler              = 1
    group                        = google_compute_region_network_endpoint_group.editor_api.self_link
    max_connections              = 0
    max_connections_per_endpoint = 0
    max_connections_per_instance = 0
    max_rate                     = 0
    max_rate_per_endpoint        = 0
    max_rate_per_instance        = 0
    max_utilization              = 0
  }

  log_config {
    enable      = "true"
    sample_rate = "1"
  }
}
