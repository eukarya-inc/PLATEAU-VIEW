resource "google_cloud_run_v2_service" "tile" {
  project  = data.google_project.project.project_id
  name     = "plateau-tile"
  ingress  = "INGRESS_TRAFFIC_ALL"
  location = var.gcp_region

  template {
    execution_environment = "EXECUTION_ENVIRONMENT_GEN2"
    service_account       = var.service_account_email
    timeout               = "300s"

    containers {
      name  = "plateau-tile"
      image = "eukarya/plateauview-tiles:latest"

      resources {
        cpu_idle          = true
        startup_cpu_boost = true

        limits = {
          cpu    = var.resources.limits.cpu
          memory = var.resources.limits.memory
        }
      }

      ports {
        container_port = 8080
        name           = "h2c"
      }

      env {
        name  = "CONFIG_URL"
        value = var.config_url
      }

      env {
        name  = "CACHE_SIZE_MB"
        value = tostring(var.cache_size_mb)
      }

      env {
        name  = "CORS_ORIGINS"
        value = var.cors_origins
      }

      dynamic "env" {
        for_each = var.tile_cache_url != "" ? [1] : []
        content {
          name  = "TILE_CACHE_URL"
          value = var.tile_cache_url
        }
      }
    }

    scaling {
      max_instance_count = var.max_instance_count
      min_instance_count = 0
    }
  }

  traffic {
    percent = 100
    type    = "TRAFFIC_TARGET_ALLOCATION_TYPE_LATEST"
  }

  lifecycle {
    ignore_changes = [
      client,
      client_version,
      template[0].containers[0].image,
      template[0].revision,
      traffic[0].revision,
      traffic[0].type,
    ]
  }
}
