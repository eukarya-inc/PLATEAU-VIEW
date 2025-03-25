resource "google_compute_global_address" "cms" {
  project = data.google_project.project.project_id
  name    = "cms"
}

resource "google_compute_global_address" "cms_cloudbuild" {
  provider = google-beta

  project = data.google_project.project.project_id
  name    = "cms-cloudbuild"

  address_type  = "INTERNAL"
  network       = data.google_compute_network.default.id
  prefix_length = 16
  purpose       = "VPC_PEERING"
}


resource "google_compute_global_address" "plateau_tiles" {
  project = data.google_project.project.project_id
  name    = "plateau-tiles"
}

resource "google_compute_global_address" "editor" {
  project = data.google_project.project.project_id
  name    = "editor"
}

resource "google_compute_global_address" "plateau_geo" {
  project = data.google_project.project.project_id
  name    = "plateau-geo"
}


resource "google_compute_global_address" "plateau_api" {
  project = data.google_project.project.project_id
  name    = "plateau-api"
}
