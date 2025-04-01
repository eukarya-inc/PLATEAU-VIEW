data "google_service_account" "cms_api" {
  project    = data.google_project.project.project_id
  account_id = var.service_account_email
}

# Deprecated. Should be removed after migration to reearth-cms-api service account.
resource "google_service_account" "cms_worker_m2m" {
  project      = data.google_project.project.project_id
  account_id   = "reearth-cms-worker-m2m"
  display_name = "Service Account for cms worker m2m"
}
