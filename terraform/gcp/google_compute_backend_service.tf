resource "google_compute_backend_service" "cms_api" {
  project = data.google_project.project.project_id
  name    = "cms-api"

  load_balancing_scheme = "EXTERNAL_MANAGED"

  backend {
    group = module.cms_api.network_endpoint_group.id
  }
}

resource "google_compute_backend_service" "cms_web" {
  project = data.google_project.project.project_id
  name    = "cms-web"

  load_balancing_scheme = "EXTERNAL_MANAGED"

  backend {
    group = module.cms_web.network_endpoint_group.id
  }
}

resource "google_compute_backend_service" "cms_worker" {
  project = data.google_project.project.project_id
  name    = "cms-worker"

  load_balancing_scheme = "EXTERNAL_MANAGED"

  backend {
    group = module.cms_worker.network_endpoint_group.id
  }
}

resource "google_compute_backend_service" "plateau_tiles" {
  project = data.google_project.project.project_id

  connection_draining_timeout_sec = 30
  enable_cdn                      = true
  name                            = "plateau-tiles"
  timeout_sec                     = 30

  backend {
    group = module.plateauview_tiles.compute_instance_group_manager.instance_group
  }

  cdn_policy {
    cache_mode                   = "CACHE_ALL_STATIC"
    client_ttl                   = 60 * 60 * 24 * 30  # 30 days
    default_ttl                  = 60 * 60 * 24 * 30  # 30 days
    max_ttl                      = 60 * 60 * 24 * 365 # 1 year
    signed_url_cache_max_age_sec = 60 * 60 * 24 * 30  # 30 days
  }

  health_checks = [module.plateauview_tiles.compute_health_check.id]

  log_config {
    enable = true
  }
}

resource "google_compute_backend_service" "editor_api" {
  project = data.google_project.project.project_id
  name    = "editor-api"

  load_balancing_scheme = "EXTERNAL_MANAGED"

  backend {
    group = module.editor_api.network_endpoint_group.id
  }
}

resource "google_compute_backend_service" "editor_web" {
  project = data.google_project.project.project_id
  name    = "editor-web"

  load_balancing_scheme = "EXTERNAL_MANAGED"

  backend {
    group = module.editor_web.network_endpoint_group.id
  }
}

resource "google_compute_backend_service" "plateau_geo" {
  project = data.google_project.project.project_id
  name    = "plateauview-geo"

  load_balancing_scheme = "EXTERNAL_MANAGED"

  backend {
    group = module.plateauview_geo.network_endpoint_group.id
  }
}


resource "google_compute_backend_service" "plateau_api" {
  project = data.google_project.project.project_id
  name    = "plateauview-api"

  load_balancing_scheme = "EXTERNAL_MANAGED"

  backend {
    group = module.plateauview_api.network_endpoint_group.id
  }
}
