resource "google_certificate_manager_certificate_map" "plateau_reearth" {
  project = data.google_project.project.project_id
  name    = "plateau-reearth"
}

resource "google_certificate_manager_certificate_map" "plateau_cms" {
  project = data.google_project.project.project_id
  name    = "plateau-cms"
}

resource "google_certificate_manager_certificate_map" "plateau_editor" {
  project = data.google_project.project.project_id
  name    = "plateau-editor"
}

resource "google_certificate_manager_certificate_map" "plateau_tiles" {
  project = data.google_project.project.project_id
  name    = "plateau-tiles"
}

resource "google_certificate_manager_certificate_map" "plateau_geo" {
  project = data.google_project.project.project_id
  name    = "plateau-geo"
}

resource "google_certificate_manager_certificate_map" "plateau_api" {
  project = data.google_project.project.project_id
  name    = "plateau-api"
}
