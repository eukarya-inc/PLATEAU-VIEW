resource "google_compute_target_https_proxy" "plateau_cms" {
  project = data.google_project.project.project_id

  certificate_map = "//certificatemanager.googleapis.com/${google_certificate_manager_certificate_map.plateau_cms.id}"
  name            = "plateau-cms"
  url_map         = google_compute_url_map.plateau_cms.id
}

resource "google_compute_target_https_proxy" "editor" {
  project = data.google_project.project.project_id

  name            = "plateau-editor"
  certificate_map = "//certificatemanager.googleapis.com/${google_certificate_manager_certificate_map.plateau_editor.id}"
  url_map         = google_compute_url_map.editor.id
}

resource "google_compute_target_https_proxy" "plateau_tiles" {
  project = data.google_project.project.project_id

  certificate_map = "//certificatemanager.googleapis.com/${google_certificate_manager_certificate_map.plateau_tiles.id}"
  name            = "plateau-tiles"
  url_map         = google_compute_url_map.plateau_tiles.id
}


resource "google_compute_target_https_proxy" "plateau_geo" {
  project = data.google_project.project.project_id

  certificate_map = "//certificatemanager.googleapis.com/${google_certificate_manager_certificate_map.plateau_geo.id}"
  name            = "plateau-geo"
  url_map         = google_compute_url_map.plateau_geo.id
}

resource "google_compute_target_https_proxy" "plateau_api" {
  project = data.google_project.project.project_id

  certificate_map = "//certificatemanager.googleapis.com/${google_certificate_manager_certificate_map.plateau_api.id}"
  name            = "plateau-api"
  url_map         = google_compute_url_map.plateau_api.id
}
