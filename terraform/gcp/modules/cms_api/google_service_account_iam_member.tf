resource "google_service_account_iam_member" "cms_api" {
  member             = "serviceAccount:${data.google_service_account.cms_api.email}"
  role               = "roles/iam.serviceAccountTokenCreator"
  service_account_id = data.google_service_account.cms_api.name
}
