resource "google_monitoring_uptime_check_config" "editor_api" {
  project = data.google_project.project.project_id

  display_name = "Re:Earth Visualizer API Cloud Run Uptime Check (${var.region})"
  period       = "900s" # 15 mins, maximum. Default is 300s (5 mins).
  timeout      = "60s"

  http_check {
    # For health checks
    # Ref: https://github.com/reearth/reearth-visualizer/blob/86e12934d9744eb8957aa89972a54a69f396eb95/server/internal/app/app.go#L122-L123
    path    = "/api/ping"
    port    = "443"
    use_ssl = true

    # Not supported for Cloud Run uptime check
    validate_ssl = false

    accepted_response_status_codes {
      status_class = "STATUS_CLASS_2XX"
    }
  }

  monitored_resource {
    type = "cloud_run_revision"
    labels = {
      location : google_cloud_run_v2_service.editor_api.location
      project_id : google_cloud_run_v2_service.editor_api.project
      service_name : google_cloud_run_v2_service.editor_api.name
    }
  }

  lifecycle {
    ignore_changes = [
      monitored_resource[0].labels
    ]
  }
}
