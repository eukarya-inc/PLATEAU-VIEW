resource "google_pubsub_subscription" "cms_notify" {
  project                    = data.google_project.project.project_id
  message_retention_duration = "604800s"
  name                       = "reearth-cms-notify"
  topic                      = google_pubsub_topic.cms_decompress.name


  expiration_policy {
    ttl = ""
  }

  push_config {
    push_endpoint = "https://${local.api_domain}/api/notify"

    oidc_token {
      service_account_email = google_service_account.cms_worker_m2m.email
      audience              = "https://${local.api_domain}"
    }
  }

  retry_policy {
    maximum_backoff = "600s"
    minimum_backoff = "10s"
  }
}

resource "google_pubsub_subscription" "cms_webhook" {
  project = data.google_project.project.project_id
  name    = "reearth-cms-webhook"
  topic   = google_pubsub_topic.cms_webhook.name

  expiration_policy {
    ttl = ""
  }

  push_config {
    push_endpoint = "https://${local.worker_domain}/api/webhook"

    oidc_token {
      service_account_email = google_service_account.cms_worker_m2m.email
    }
  }

  retry_policy {
    maximum_backoff = "600s"
    minimum_backoff = "10s"
  }
}
