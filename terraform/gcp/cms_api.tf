module "cms_api" {
  source = "./modules/cms_api"

  project          = data.google_project.project.project_id
  account_database = "reearth_account"
  assets_bucket    = "${var.prefix}-cms-assets-bucket"
  auth_client_m2m = {
    client_id     = module.auth0_cms.client_m2m.client_id
    client_secret = module.auth0_cms.client_m2m.client_secret
  }

  auth_client_spa = {
    client_id = module.auth0_cms.client_spa.client_id
  }

  auth0_domain                  = var.auth0_domain
  auth0_client_client_secret_id = google_secret_manager_secret.cms_auth0_client_secret.secret_id

  cloudbuild_service_account_email = google_service_account.cms_cloudbuild.email
  database_secret_id               = google_secret_manager_secret.cms_db.secret_id
  domain                           = var.domain
  image                            = "eukarya/plateauview-cms-api"
  service_account_email            = google_service_account.cms_api.email
  task_database_secret_id          = google_secret_manager_secret.cms_db.secret_id
  task_region                      = google_cloudbuild_worker_pool.cms.location
  worker_pool_id                   = google_cloudbuild_worker_pool.cms.name
}
