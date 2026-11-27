resource "google_secret_manager_secret_version" "plateauview_ckan_token" {
  secret      = google_secret_manager_secret.plateauview["REEARTH_PLATEAUVIEW_CKAN_TOKEN"].id
  secret_data = var.ckan_token
}

resource "google_secret_manager_secret_version" "plateauview_cms_token" {
  secret      = google_secret_manager_secret.plateauview["REEARTH_PLATEAUVIEW_CMS_TOKEN"].id
  secret_data = "DUMMY" # MEMO: Will be added after CMS has started.
}

resource "google_secret_manager_secret_version" "plateauview_cms_webhook_secret" {
  secret      = google_secret_manager_secret.plateauview["REEARTH_PLATEAUVIEW_CMS_WEBHOOK_SECRET"].id
  secret_data = random_password.plateauview_env["REEARTH_PLATEAUVIEW_CMS_WEBHOOK_SECRET"].result
}

resource "google_secret_manager_secret_version" "plateauview_fme_token" {
  secret      = google_secret_manager_secret.plateauview["REEARTH_PLATEAUVIEW_FME_TOKEN"].id
  secret_data = var.fme_token
}

resource "google_secret_manager_secret_version" "plateauview_secret" {
  secret      = google_secret_manager_secret.plateauview["REEARTH_PLATEAUVIEW_SECRET"].id
  secret_data = random_password.plateauview_env["REEARTH_PLATEAUVIEW_SECRET"].result
}

resource "google_secret_manager_secret_version" "plateauview_sendgrid_api_key" {
  secret      = google_secret_manager_secret.plateauview["REEARTH_PLATEAUVIEW_SENDGRID_API_KEY"].id
  secret_data = var.sendgrid_api_key
}
