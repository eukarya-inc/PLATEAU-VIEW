# Many of the resources are managed manually so we have to hard code it.
module "editor_web" {
  source = "./modules/editor_web"

  api_url = "https://${local.editor_api_domain}/api"

  auth0_audience  = "https://${local.editor_api_domain}"
  auth0_domain    = var.auth0_domain
  auth0_client_id = module.auth0_editor.client_spa.client_id

  brand = var.web_config.brand

  # cesium_ion_token_secret_id   = google_secret_manager_secret.cesium_ion_token.secret_id
  disable_workspace_management = true
  extension_urls = [
  ]
  favicon_url           = "https://www.mlit.go.jp/plateau/assets/img/icons/favicon.svg"
  image                 = "eukarya/plateauview-editor-web"
  plugins               = "https://${local.editor_static_domain}/plugins"
  project               = data.google_project.project.project_id
  published             = "https://{}.${local.editor_domain}"
  service_account_email = google_service_account.editor_web.email
  title                 = "PLATEAU VIEW(Ver2.0)" # TODO: fix value
  unsafe_plugin_urls = [
    "https://${local.plateauview_api_domain}/extension/PlateauView3.js"
  ]
}
