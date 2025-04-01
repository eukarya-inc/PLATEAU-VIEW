resource "google_secret_manager_secret_iam_member" "plateauview" {
  for_each  = toset(local.plateauview_secrets)
  project   = google_secret_manager_secret.plateauview[each.value].project
  secret_id = google_secret_manager_secret.plateauview[each.value].secret_id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${var.service_account_email}"
}
