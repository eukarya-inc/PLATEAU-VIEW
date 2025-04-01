resource "google_project_iam_member" "cms_api" {
  for_each = toset([
    "roles/cloudbuild.builds.builder",
    "roles/cloudprofiler.agent",
  ])

  project = data.google_project.project.project_id
  role    = each.value
  member  = "serviceAccount:${data.google_service_account.cms_api.email}"
}
