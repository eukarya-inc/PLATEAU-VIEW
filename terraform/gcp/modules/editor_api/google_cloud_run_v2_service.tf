resource "google_cloud_run_v2_service" "editor_api" {
  project  = data.google_project.project.project_id
  name     = "editor-api"
  location = var.region
  ingress  = "INGRESS_TRAFFIC_ALL"

  template {
    containers {
      image = var.image

      resources {
        cpu_idle          = true
        startup_cpu_boost = true

        limits = {
          cpu    = var.resources.limits.cpu
          memory = var.resources.limits.memory
        }
      }

      env {
        name  = "REEARTH_AUTH0_CLIENTID"
        value = var.auth0_client_id
      }

      env {
        name = "REEARTH_AUTH0_CLIENTSECRET"
        value_source {
          secret_key_ref {
            secret  = var.auth0_client_secret_secret_id
            version = "latest"
          }
        }
      }

      env {
        name  = "REEARTH_AUTH0_DOMAIN"
        value = "https://${var.auth0_domain}/"
      }

      env {
        name  = "REEARTH_GCS_BUCKETNAME"
        value = google_storage_bucket.editor_static.id
      }

      dynamic "env" {
        for_each = contains(["prod", "dev"], var.env) ? [""] : []
        content {
          name  = "REEARTH_CERT_IP"
          value = var.ip_address
        }
      }
      dynamic "env" {
        for_each = contains(["prod", "dev"], var.env) ? [""] : []
        content {
          name  = "REEARTH_CERT_PUBSUBTOPICISSUE"
          value = "reearth-${var.env}-customdomain-issue-cert"
        }
      }

      dynamic "env" {
        for_each = contains(["prod", "dev"], var.env) ? [""] : []
        content {
          name  = "REEARTH_CERT_PUBSUBTOPICREVOKE"
          value = "reearth-${var.env}-customdomain-revoke-cert"
        }
      }
      env {
        name  = "GOOGLE_CLOUD_PROJECT"
        value = data.google_project.project.project_id
      }

      # Currently, only used in production environment.
      dynamic "env" {
        for_each = var.env == "prod" ? [""] : []
        content {
          name  = "REEARTH_ENV"
          value = local.env[var.env]
        }
      }

      env {
        name  = "REEARTH_ASSETBASEURL"
        value = "https://api.${var.visualizer_domain}"
      }

      env {
        name  = "REEARTH_GCS_PUBLICATIONCACHECONTROL"
        value = "no-store"
      }

      env {
        name  = "REEARTH_ORIGINS"
        value = join(",", var.origins)
      }

      env {
        name  = "REEARTH_AUTHSRV_DISABLED"
        value = "true"
      }

      env {
        name  = "REEARTH_HOST"
        value = var.visualizer_domain
      }

      env {
        name  = "REEARTH_HOST_WEB"
        value = var.visualizer_domain
      }

      env {
        name  = "REEARTH_AUTH0_AUD"
        value = "https://api.${var.domain}"
      }

      # Currently, only used in production environment.
      dynamic "env" {
        for_each = var.env == "prod" ? [""] : []
        content {
          name  = "REEARTH_POLICY_DEFAULT"
          value = "free"
        }
      }

      env {
        name  = "REEARTH_DB_ACCOUNT"
        value = var.account_database
      }

      env {
        name  = "REEARTH_AUTH0_AUDIENCE"
        value = "https://api.${var.domain}"
      }

      env {
        name = "REEARTH_DB"
        value_source {
          secret_key_ref {
            secret  = var.database_secret_id
            version = "latest"
          }
        }
      }

      dynamic "env" {
        for_each = var.database != null ? [""] : []
        content {
          name  = "REEARTH_DB_VIS"
          value = var.database
        }
      }

      # Currently, only used in development environment.
      dynamic "env" {
        for_each = var.env == "dev" ? [""] : []
        content {
          name  = "REEARTH_TRACERSAMPLE"
          value = ".0"
        }
      }

      # Currently, only used in development environment.
      dynamic "env" {
        for_each = var.env == "dev" ? [""] : []
        content {
          name  = "REEARTH_DEV"
          value = "true"
        }
      }

      env {
        name = "REEARTH_SIGNUPSECRET"
        value_source {
          secret_key_ref {
            secret  = var.signup_secret_secret_id
            version = "latest"
          }
        }
      }

      dynamic "env" {
        for_each = var.ext_plugin_url != "" ? [""] : []
        content {
          name  = "REEARTH_EXT_PLUGIN"
          value = var.ext_plugin_url
        }
      }

      dynamic "env" {
        for_each = var.redis_url_secret_id != "" ? [""] : []
        content {
          name = "REDIS_URL"
          value_source {
            secret_key_ref {
              secret  = var.redis_url_secret_id
              version = "latest"
            }
          }
        }
      }
    }

    scaling {
      max_instance_count = 20
      min_instance_count = 1
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
      template[0].revision,
      traffic[0].revision,
      traffic[0].type,
    ]
  }

  depends_on = [
    google_project_iam_custom_role.editor
  ]
}
