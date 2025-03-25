output "assets_backend_service" {
  value = google_compute_backend_bucket.assets
}

output "network_endpoint_group" {
  value = google_compute_region_network_endpoint_group.cms_api
}

output "static_backend_service" {
  value = google_compute_backend_bucket.static
}
