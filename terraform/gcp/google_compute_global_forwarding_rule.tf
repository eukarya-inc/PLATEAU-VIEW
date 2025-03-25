resource "google_compute_global_forwarding_rule" "plateau_cms" {
  project               = data.google_project.project.project_id
  load_balancing_scheme = "EXTERNAL_MANAGED"
  ip_address            = google_compute_global_address.cms.address
  name                  = "cms"
  port_range            = "443"
  target                = google_compute_target_https_proxy.plateau_cms.self_link
}

resource "google_compute_global_forwarding_rule" "plateau_tiles" {
  project    = data.google_project.project.project_id
  ip_address = google_compute_global_address.plateau_tiles.address
  name       = "plateau-tiles"
  port_range = "443"
  target     = google_compute_target_https_proxy.plateau_tiles.self_link
}


resource "google_compute_global_forwarding_rule" "editor" {
  project = data.google_project.project.project_id

  name                  = "editor"
  load_balancing_scheme = "EXTERNAL_MANAGED"
  ip_address            = google_compute_global_address.editor.id
  ip_protocol           = "TCP"
  port_range            = "443"
  target                = google_compute_target_https_proxy.editor.id
}

resource "google_compute_global_forwarding_rule" "plateau_api" {
  project = data.google_project.project.project_id

  name                  = "plateau-api"
  load_balancing_scheme = "EXTERNAL_MANAGED"
  ip_address            = google_compute_global_address.plateau_api.id
  ip_protocol           = "TCP"
  port_range            = "443"
  target                = google_compute_target_https_proxy.plateau_api.id
}
