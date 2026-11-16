output "assets_backend_service" {
  value = google_compute_backend_bucket.assets
}

output "network_endpoint_group" {
  value = google_compute_region_network_endpoint_group.cms_api
}

output "static_backend_service" {
  value = google_compute_backend_bucket.static
}

output "pubsub_push_service_account_email" {
  description = "Email of the service account that signs the OIDC tokens of Pub/Sub push requests to the CMS worker."
  value       = google_service_account.cms_worker_m2m.email
}
