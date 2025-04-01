resource "google_cloud_run_v2_service" "plateauview_geo" {
  project  = data.google_project.project.project_id
  name     = "plateauview-geo"
  ingress  = "INGRESS_TRAFFIC_ALL"
  location = var.gcp_region

  template {
    execution_environment = "EXECUTION_ENVIRONMENT_GEN2"
    service_account       = var.service_account_email
    timeout               = "300s"

    containers {
      image = "eukarya/plateauview-geo:latest"
      resources {
        limits = {
          cpu    = "1000m"
          memory = "512Mi"
        }
      }

      ports {
        container_port = 8080
        name           = "h2c"
      }

      env {
        name  = "GOOGLE_PROJECT_ID"
        value = var.gcp_project_id
      }

      env {
        name  = "ALLOW_ORIGIN"
        value = "[\"https://${local.reearth_domain}\", \"https://*.${var.domain}\"]"
      }

      env {
        name  = "TILE_CACHE_ROOT"
        value = "gs://${var.tile_cache_bucket_name}/tiles"
      }
    }

    scaling {
      max_instance_count = 10
    }
  }

  traffic {
    percent = 100
    type    = "TRAFFIC_TARGET_ALLOCATION_TYPE_LATEST"
  }

  lifecycle {
    ignore_changes = [
      template[0].containers[0].image,
    ]
  }
}
