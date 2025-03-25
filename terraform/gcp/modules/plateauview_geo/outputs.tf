output "cloud_run_service_url" {
  value = google_cloud_run_v2_service.plateauview_geo.uri
}

output "network_endpoint_group" {
  value = google_compute_region_network_endpoint_group.plateauview_geo
}
