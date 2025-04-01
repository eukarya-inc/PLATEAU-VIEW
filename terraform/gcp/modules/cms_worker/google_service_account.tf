data "google_service_account" "cms_worker" {
  project    = data.google_project.project.project_id
  account_id = var.service_account_email
}
