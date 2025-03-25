resource "google_cloud_run_v2_service" "editor_web" {
  project  = data.google_project.project.project_id
  name     = "editor-web"
  location = var.region
  ingress  = "INGRESS_TRAFFIC_ALL"

  template {
    containers {
      image = var.image

      resources {
        cpu_idle          = true
        startup_cpu_boost = true
      }

      dynamic "env" {
        for_each = var.api_url == "" ? [] : [""]
        content {
          name  = "REEARTH_API"
          value = var.api_url
        }
      }

      env {
        name  = "REEARTH_AUTH0_AUDIENCE"
        value = var.auth0_audience
      }

      env {
        name  = "REEARTH_AUTH0_CLIENT_ID"
        value = var.auth0_client_id
      }

      env {
        name  = "REEARTH_AUTH0_DOMAIN"
        value = var.auth0_domain
      }

      dynamic "env" {
        for_each = var.brand == null ? [] : [""]
        content {
          name  = "REEARTH_BRAND"
          value = jsonencode(var.brand)
        }
      }

      dynamic "env" {
        for_each = var.cesium_ion_token_secret_id == "" ? [] : [""]
        content {
          name = "REEARTH_CESIUM_ION_ACCESS_TOKEN"

          value_source {
            secret_key_ref {
              secret  = var.cesium_ion_token_secret_id
              version = "latest"
            }
          }
        }
      }

      dynamic "env" {
        for_each = var.cloud_api_url == "" ? [] : [""]
        content {
          name  = "REEARTH_CLOUD_API"
          value = var.cloud_api_url
        }
      }

      dynamic "env" {
        for_each = var.developer_mode == "" ? [] : [""]
        content {
          name  = "REEARTH_DEVELOPER_MODE"
          value = var.developer_mode
        }
      }

      dynamic "env" {
        for_each = var.disable_workspace_management == null ? [] : [""]
        content {
          name  = "REEARTH_DISABLE_WORKSPACE_MANAGEMENT"
          value = var.disable_workspace_management
        }
      }

      dynamic "env" {
        for_each = var.documentation_url == "" ? [] : [""]
        content {
          name  = "REEARTH_DOCUMENTATION_URL"
          value = var.documentation_url
        }
      }

      dynamic "env" {
        for_each = length(var.early_access_admins) > 0 ? [""] : []
        content {
          name  = "REEARTH_EARLY_ACCESS_ADMINS"
          value = jsonencode(var.early_access_admins)
        }
      }

      dynamic "env" {
        for_each = var.extension_urls == "" ? [] : [""]
        content {
          name  = "REEARTH_EXTENSION_URLS"
          value = jsonencode(var.extension_urls)
        }
      }

      dynamic "env" {
        for_each = var.favicon_url == "" ? [] : [""]
        content {
          name  = "REEARTH_FAVICON_URL"
          value = var.favicon_url
        }
      }

      dynamic "env" {
        for_each = var.marketplace_url == "" ? [] : [""]
        content {
          name  = "REEARTH_MARKETPLACE_URL"
          value = var.marketplace_url
        }
      }

      dynamic "env" {
        for_each = var.multi_tenant == null ? [] : [""]
        content {
          name  = "REEARTH_MULTI_TENANT"
          value = jsonencode(var.multi_tenant)
        }
      }

      dynamic "env" {
        for_each = var.ip == "" ? [] : [""]
        content {
          name  = "REEARTH_IP"
          value = var.ip
        }
      }

      dynamic "env" {
        for_each = var.password_policy == "" ? [] : [""]
        content {
          name  = "REEARTH_PASSWORD_POLICY"
          value = jsonencode(var.password_policy)
        }
      }

      dynamic "env" {
        for_each = var.plugins == "" ? [] : [""]
        content {
          name  = "REEARTH_PLUGINS"
          value = var.plugins
        }
      }

      dynamic "env" {
        for_each = var.policy == null ? [] : [""]
        content {
          name  = "REEARTH_POLICY"
          value = jsonencode(var.policy)
        }
      }

      dynamic "env" {
        for_each = var.published == "" ? [] : [""]
        content {
          name  = "REEARTH_PUBLISHED"
          value = var.published
        }
      }

      dynamic "env" {
        for_each = length(var.unsafe_plugin_urls) > 0 ? [""] : []
        content {
          name  = "REEARTH_UNSAFE_PLUGIN_URLS"
          value = jsonencode(var.unsafe_plugin_urls)
        }
      }
    }

    scaling {
      max_instance_count = 100
      min_instance_count = 0
    }

    service_account = var.service_account_email
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
      template[0].containers[0].name,
      template[0].revision,
      traffic[0].revision,
      traffic[0].type,
    ]
  }
}
