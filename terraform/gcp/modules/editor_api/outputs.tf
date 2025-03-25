output "network_endpoint_group" {
  value = google_compute_region_network_endpoint_group.editor_api
}

output "static_backend_service" {
  value = google_compute_backend_bucket.editor_static
}
