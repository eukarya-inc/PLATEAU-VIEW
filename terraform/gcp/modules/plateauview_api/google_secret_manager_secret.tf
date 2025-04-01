resource "google_secret_manager_secret" "plateauview" {
  for_each  = toset(local.plateauview_secrets)
  project   = data.google_project.project.project_id
  secret_id = "plateau-view-${each.value}"

  replication {
    auto {}
  }
}
