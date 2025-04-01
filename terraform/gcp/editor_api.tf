module "editor_api" {
  source = "./modules/editor_api"

  assets_bucket                 = "${var.prefix}-static-bucket"
  account_database              = "reearth_account"
  auth0_client_id               = var.auth0_client_id
  auth0_client_secret_secret_id = google_secret_manager_secret.editor_auth0_client_secret.id
  auth0_domain                  = var.auth0_domain

  database_secret_id = google_secret_manager_secret.editor_db.id
  domain             = local.editor_domain
  env                = "dev"
  image              = "eukarya/plateauview-editor-api"
  origins = [
    "https://${local.editor_domain}",
    "https://*.${local.editor_domain}",
  ]
  project = data.google_project.project.project_id

  region = "asia-northeast1"
  resources = {
    limits = {
      cpu    = "1000m", # Using default value. remove later.
      memory = "512Mi"
    }
  }

  service_account_email   = google_service_account.editor_api.email
  signup_secret_secret_id = google_secret_manager_secret.editor_auth0_signup_secret.id
  visualizer_domain       = local.editor_domain
  prefix                  = var.prefix
}
