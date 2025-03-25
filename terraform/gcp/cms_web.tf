module "cms_web" {
  source = "./modules/cms_web"

  auth0_audience  = "https://${local.cms_api_domain}"
  auth0_domain    = var.auth0_domain
  auth0_client_id = module.auth0_cms.client_spa.client_id

  api_url = "https://${local.cms_api_domain}/api"
  # cesium_ion_token_secret_id = google_secret_manager_secret.cesium_ion_token.secret_id
  editor_url = "https://${local.editor_domain}"

  image                 = "eukarya/plateauview-cms-web"
  project               = data.google_project.project.project_id
  service_account_email = google_service_account.cms_web.email
}
