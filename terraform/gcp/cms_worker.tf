module "cms_worker" {
  source  = "./modules/cms_worker"
  project = data.google_project.project.project_id

  assets_bucket         = "${var.prefix}-cms-assets-bucket"
  auth0_domain          = var.auth0_domain
  database_secret_id    = google_secret_manager_secret.cms_db.secret_id
  domain                = local.cms_worker_domain
  image                 = "eukarya/plateauview-cms-worker"
  service_account_email = google_service_account.cms_worker.email
}
