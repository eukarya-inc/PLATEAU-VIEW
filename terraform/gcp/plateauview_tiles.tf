module "plateauview_tiles" {
  source = "./modules/plateauview_tiles"

  config_url            = "https://${local.plateauview_api_domain}/tiles/config.json"
  cors_origins          = "https://*.${var.domain}"
  gcp_project_id        = data.google_project.project.project_id
  gcp_region            = var.gcp_region
  service_account_email = google_service_account.plateau_tiles.email
  tile_cache_url        = "gs://${google_storage_bucket.app_tile_cache.name}/tiles"
}
