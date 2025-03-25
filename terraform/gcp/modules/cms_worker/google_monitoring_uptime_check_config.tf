resource "google_monitoring_uptime_check_config" "cms_worker" {
  project = data.google_project.project.project_id

  display_name = "Re:Earth CMS Worker Cloud Run Uptime Check (${var.region})"
  period       = "900s" # 15 mins, maximum. Default is 300s (5 mins).
  timeout      = "60s"

  http_check {
    # Ref: https://github.com/reearth/reearth-cms/blob/ef04229a7f12a61fe3d64a5b2890e5ba2a633c53/worker/internal/app/app.go#L41
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
      location : google_cloud_run_v2_service.cms_worker.location
      project_id : google_cloud_run_v2_service.cms_worker.project
      service_name : google_cloud_run_v2_service.cms_worker.name
    }
  }

  lifecycle {
    ignore_changes = [
      monitored_resource[0].labels
    ]
  }
}
