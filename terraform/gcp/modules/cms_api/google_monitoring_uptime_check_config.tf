resource "google_monitoring_uptime_check_config" "cms_api" {
  project = data.google_project.project.project_id

  display_name = "Re:Earth CMS API Cloud Run Uptime Check (${var.region})"
  period       = "900s" # 15 mins, maximum. Default is 300s (5 mins).
  timeout      = "60s"

  http_check {
    # Ref: https://github.com/reearth/reearth-cms/blob/61175e6a7c99eeb3e1cd5536f2ba4f4e6ad14d88/server/internal/app/app.go#L68-L69
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
      location : google_cloud_run_v2_service.cms_api.location
      project_id : google_cloud_run_v2_service.cms_api.project
      service_name : google_cloud_run_v2_service.cms_api.name
    }
  }

  lifecycle {
    ignore_changes = [
      monitored_resource[0].labels
    ]
  }
}
