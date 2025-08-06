resource "google_compute_target_http_proxy" "editor" {
  project = data.google_project.project.project_id
  name    = "editor"
  url_map = google_compute_url_map.editor_http_redirect.id
}

resource "google_compute_target_http_proxy" "plateau_cms" {
  project = data.google_project.project.project_id
  name    = "plateau-cms"
  url_map = google_compute_url_map.plateau_cms_http_redirect.id
}

resource "google_compute_target_http_proxy" "plateau_flow" {
  project = data.google_project.project.project_id
  name    = "plateau-flow"
  url_map = google_compute_url_map.plateau_flow_http_redirect.id
}
