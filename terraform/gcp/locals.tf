locals {
  services = [
    "certificatemanager.googleapis.com",
    "cloudresourcemanager.googleapis.com",
    "cloudbuild.googleapis.com",
    "cloudtasks.googleapis.com",
    "compute.googleapis.com",
    "dns.googleapis.com",
    "iam.googleapis.com",
    "run.googleapis.com",
    "secretmanager.googleapis.com",
    "servicenetworking.googleapis.com",
    "sts.googleapis.com",
    "pubsub.googleapis.com",
  ]
}

locals {
  cms_domain        = "cms.${var.domain}"
  cms_api_domain    = "api.${local.cms_domain}"
  cms_assets_domain = "assets.${local.cms_domain}"
  cms_worker_domain = "worker.${local.cms_domain}"

  editor_domain        = "editor.${var.domain}"
  editor_api_domain    = "api.${local.editor_domain}"
  editor_static_domain = "static.${local.editor_domain}"

  flow_domain = "flow.${var.domain}"

  geo_domain             = "geo.${var.domain}"
  plateauview_api_domain = "api.${var.domain}"
  tiles_domain           = "tiles.${var.domain}"
}

locals {
  plateau_tiles = {
    roles = [
      # Required permission for Node Problem Detector.
      "roles/monitoring.metricWriter"
    ]
  }
}
