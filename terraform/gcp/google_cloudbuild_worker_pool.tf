resource "google_cloudbuild_worker_pool" "cms" {
  project  = data.google_project.project.project_id
  name     = "cms"
  location = "asia-northeast1"

  network_config {
    peered_network          = data.google_compute_network.default.id
    peered_network_ip_range = "/${google_compute_global_address.cms_cloudbuild.prefix_length}"
  }

  worker_config {
    disk_size_gb   = 100
    machine_type   = "e2-standard-4"
    no_external_ip = false # Allow to access the internet.
  }
}
