resource "google_certificate_manager_certificate_map_entry" "plateau_cms" {
  project = data.google_project.project.project_id

  certificates = [
    google_certificate_manager_certificate.plateau_cms.id,
  ]
  map     = google_certificate_manager_certificate_map.plateau_cms.name
  matcher = "PRIMARY"
  name    = "plateau-cms"
}

resource "google_certificate_manager_certificate_map_entry" "plateau_editor" {
  project = data.google_project.project.project_id

  certificates = [
    google_certificate_manager_certificate.plateau_editor.id,
  ]
  map     = google_certificate_manager_certificate_map.plateau_editor.name
  matcher = "PRIMARY"
  name    = "plateau-editor"
}

resource "google_certificate_manager_certificate_map_entry" "plateau_tiles" {
  project = data.google_project.project.project_id

  certificates = [
    google_certificate_manager_certificate.plateau_tiles.id,
  ]
  hostname = google_certificate_manager_dns_authorization.plateau_tiles.domain
  map      = google_certificate_manager_certificate_map.plateau_tiles.name
  name     = "plateau-tiles"
}

resource "google_certificate_manager_certificate_map_entry" "plateau_geo" {
  project = data.google_project.project.project_id

  certificates = [
    google_certificate_manager_certificate.plateau_geo.id,
  ]
  hostname = google_certificate_manager_dns_authorization.plateau_geo.domain
  map      = google_certificate_manager_certificate_map.plateau_geo.name
  name     = "plateau-geo"
}

resource "google_certificate_manager_certificate_map_entry" "plateau_api" {
  project = data.google_project.project.project_id

  certificates = [
    google_certificate_manager_certificate.plateau_api.id,
  ]
  hostname = google_certificate_manager_dns_authorization.plateau_api.domain
  map      = google_certificate_manager_certificate_map.plateau_api.name
  name     = "plateau-api"
}
