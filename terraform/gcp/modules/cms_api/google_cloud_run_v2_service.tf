resource "google_cloud_run_v2_service" "cms_api" {
  project  = data.google_project.project.project_id
  name     = var.name
  location = var.region
  ingress  = "INGRESS_TRAFFIC_ALL"

  template {
    execution_environment            = "EXECUTION_ENVIRONMENT_GEN2"
    max_instance_request_concurrency = 80
    service_account                  = data.google_service_account.cms_api.email
    timeout                          = "3600s"

    containers {
      image = var.image

      env {
        name  = "GOOGLE_CLOUD_PROJECT"
        value = data.google_project.project.project_id
      }

      env {
        name  = "REEARTH_CMS_ASSETBASEURL"
        value = "https://${local.assets_domain}"
      }

      env {
        name  = "REEARTH_CMS_AUTH0_AUDIENCE"
        value = "https://${local.api_domain}"
      }

      env {
        name  = "REEARTH_CMS_AUTH0_DOMAIN"
        value = "https://${var.auth0_domain}"
      }

      env {
        name = "REEARTH_CMS_AUTH0_CLIENTSECRET"
        value_source {
          secret_key_ref {
            secret  = var.auth0_client_client_secret_id
            version = "latest"
          }
        }
      }

      env {
        name  = "REEARTH_CMS_AUTHM2M_AUD"
        value = "https://${local.api_domain}"
      }

      env {
        name  = "REEARTH_CMS_AUTH0_WEBCLIENTID"
        value = var.auth_client_spa.client_id
      }

      env {
        name  = "REEARTH_CMS_AUTHM2M_EMAIL"
        value = google_service_account.cms_worker_m2m.email
      }

      env {
        name  = "REEARTH_CMS_AUTHM2M_ISS"
        value = "https://accounts.google.com"
      }

      env {
        name = "REEARTH_CMS_DB"
        value_source {
          secret_key_ref {
            secret  = var.database_secret_id
            version = "latest"
          }
        }
      }

      dynamic "env" {
        for_each = var.account_database != null ? [""] : []
        content {
          name  = "REEARTH_CMS_DB_ACCOUNT"
          value = var.account_database
        }
      }

      env {
        name  = "REEARTH_CMS_SECRET_DB"
        value = "reearth-cms-db"
      }

      dynamic "env" {
        for_each = var.database != null ? [""] : []
        content {
          name  = "REEARTH_CMS_DB_CMS"
          value = var.database
        }
      }

      env {
        name  = "REEARTH_CMS_GCS_BUCKETNAME"
        value = google_storage_bucket.assets.name
      }

      env {
        name  = "REEARTH_CMS_GRAPHQL_COMPLEXITYLIMIT"
        value = "6000"
      }

      env {
        name  = "REEARTH_CMS_HOST"
        value = "https://${local.api_domain}"
      }

      env {
        name  = "REEARTH_CMS_ORIGINS"
        value = "https://${local.cms_domain}"
      }

      env {
        name  = "REEARTH_GCS_PUBLICATIONCACHECONTROL"
        value = "no-store"
      }

      env {
        name  = "REEARTH_CMS_TASK_DECOMPRESSORIMAGE"
        value = "reearth/reearth-cms-decompressor:rc"
      }

      env {
        name  = "REEARTH_CMS_TASK_COPIERIMAGE"
        value = "reearth/reearth-cms-copier:rc"
      }

      env {
        name  = "REEARTH_CMS_TASK_DBSECRETNAME"
        value = var.task_database_secret_id
      }

      env {
        name  = "REEARTH_CMS_TASK_CMSIMAGE"
        value = "reearth/reearth-cms:rc"
      }

      env {
        name  = "REEARTH_CMS_TASK_GCPPROJECT"
        value = data.google_project.project.project_id
      }

      env {
        name  = "REEARTH_CMS_TASK_BUILDSERVICEACCOUNT"
        value = var.cloudbuild_service_account_email
      }

      dynamic "env" {
        for_each = var.task_region != null ? [""] : []
        content {
          name  = "REEARTH_CMS_TASK_GCPREGION"
          value = var.task_region
        }
      }

      env {
        name  = "REEARTH_CMS_TASK_SUBSCRIBERURL"
        value = "https://${local.worker_domain}/api/decompress"
      }

      env {
        name  = "REEARTH_CMS_TASK_TOPIC"
        value = google_pubsub_topic.cms_webhook.name
      }

      env {
        name  = "REEARTH_CMS_TASK_QUEUENAME"
        value = google_pubsub_topic.cms_decompress.name
      }

      dynamic "env" {
        for_each = var.worker_pool_id != null ? [""] : []
        content {
          name  = "REEARTH_CMS_TASK_WORKERPOOL"
          value = var.worker_pool_id
        }
      }

      env {
        name  = "REEARTH_HOST_WEB"
        value = "https://${local.cms_domain}"
      }

      env {
        name  = "REEARTH_CMS_ASSET_PUBLIC"
        value = "true"
      }

      resources {
        cpu_idle = true

        limits = {
          cpu    = "1000m"
          memory = "512Mi"
        }
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
      client,
      client_version,

      # Currently, we restrict production environment to use all ingress.
      # It will be released in 2025 so I'll just ignore changes as for now.
      ingress,

      template[0].containers[0].image,
      template[0].annotations["run.googleapis.com/client-name"],
      template[0].annotations["client.knative.dev/user-image"],
      template[0].annotations["run.googleapis.com/client-version"],

      # Re:Earth CMS is used in various project but we can't make this complicated.
      template[0].vpc_access,
    ]
  }
}
