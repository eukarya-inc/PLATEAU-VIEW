resource "google_compute_firewall" "allow_health_check" {
  project  = data.google_project.project.project_id
  name     = "default-allow-health-check"
  network  = "default"
  priority = 1000

  allow {
    protocol = "tcp"
  }

  # See: https://cloud.google.com/load-balancing/docs/https
  source_ranges = [
    "35.191.0.0/16",
    "130.211.0.0/22",
  ]

  target_tags = ["lb-health-check"]
}
