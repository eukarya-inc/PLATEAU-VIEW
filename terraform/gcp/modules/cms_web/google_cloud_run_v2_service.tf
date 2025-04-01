resource "google_cloud_run_v2_service" "cms_web" {
  project  = data.google_project.project.project_id
  name     = "cms-web"
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
          name  = "REEARTH_CMS_API"
          value = var.api_url
        }
      }

      env {
        name  = "REEARTH_CMS_AUTH0_AUDIENCE"
        value = var.auth0_audience
      }

      env {
        name  = "REEARTH_CMS_AUTH0_CLIENT_ID"
        value = var.auth0_client_id
      }

      env {
        name  = "REEARTH_CMS_AUTH0_DOMAIN"
        value = var.auth0_domain
      }

      dynamic "env" {
        for_each = var.cesium_ion_token_secret_id == "" ? [] : [""]
        content {
          name = "REEARTH_CMS_CESIUM_ION_ACCESS_TOKEN"

          value_source {
            secret_key_ref {
              secret  = var.cesium_ion_token_secret_id
              version = "latest"
            }
          }
        }
      }

      dynamic "env" {
        for_each = var.cover_image_url == "" ? [] : [""]
        content {
          name  = "REEARTH_CMS_COVER_IMAGE_URL"
          value = var.cover_image_url
        }
      }

      dynamic "env" {
        for_each = var.editor_url == "" ? [] : [""]
        content {
          name  = "REEARTH_CMS_EDITOR_URL"
          value = var.editor_url
        }
      }

      dynamic "env" {
        for_each = var.logo_url == "" ? [] : [""]
        content {
          name  = "REEARTH_CMS_LOGO_URL"
          value = var.logo_url
        }
      }

      dynamic "env" {
        for_each = var.favicon_url == "" ? [] : [""]
        content {
          name  = "REEARTH_CMS_FAVICON_URL"
          value = var.favicon_url
        }
      }

      dynamic "env" {
        for_each = var.title == "" ? [] : [""]
        content {
          name  = "REEARTH_CMS_TITLE"
          value = var.title
        }
      }

      dynamic "env" {
        for_each = var.multi_tenant == "" ? [] : [""]
        content {
          name  = "REEARTH_CMS_MULTI_TENANT"
          value = var.multi_tenant
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

      # Currently, we restrict production environment to use all ingress.
      # It will be released in 2025 so I'll just ignore changes as for now.
      ingress,

      template[0].containers[0].image,
      template[0].containers[0].name,
      template[0].revision,
      traffic[0].revision,
      traffic[0].type,
    ]
  }
}
