resource "google_cloud_run_v2_service" "cms_worker" {
  project  = data.google_project.project.project_id
  name     = var.name
  location = var.region

  template {
    execution_environment            = "EXECUTION_ENVIRONMENT_GEN2"
    max_instance_request_concurrency = 1
    service_account                  = data.google_service_account.cms_worker.email
    timeout                          = "3600s"

    containers {
      image = var.image

      env {
        name  = "GCS_BUCKET_NAME"
        value = var.assets_bucket
      }

      env {
        name  = "GOOGLE_CLOUD_PROJECT"
        value = data.google_project.project.project_id
      }

      env {
        name  = "REEARTH_CMS_WORKER_PUBSUB_TOPIC"
        value = "decompress"
      }

      env {
        name  = "REEARTH_CMS_WORKER_GCP_PROJECT"
        value = data.google_project.project.name
      }

      env {
        name  = "REEARTH_CMS_WORKER_DECOMPRESSION_NUM_WORKERS"
        value = "500"
      }

      env {
        name  = "REEARTH_CMS_WORKER_DECOMPRESSION_WORKQUEUE_DEPTH"
        value = "https://${var.auth0_domain}"
      }

      env {
        name = "REEARTH_CMS_WORKER_DB"
        value_source {
          secret_key_ref {
            secret  = var.database_secret_id
            version = "latest"
          }
        }
      }

      resources {
        cpu_idle = true

        limits = {
          cpu    = "8000m"
          memory = "32Gi"
        }
      }

      ports {
        container_port = 8080
        name           = "h2c"
      }
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
      template[0].annotations["run.googleapis.com/client-name"],
      template[0].annotations["client.knative.dev/user-image"],
      template[0].annotations["run.googleapis.com/client-version"],

      # Re:Earth CMS is used in various project but we can't make this complicated.
      template[0].vpc_access,
    ]
  }
}
