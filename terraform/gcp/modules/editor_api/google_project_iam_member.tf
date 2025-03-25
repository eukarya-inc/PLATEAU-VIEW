resource "google_project_iam_member" "editor" {
  project = data.google_project.project.project_id
  role    = google_project_iam_custom_role.editor.id
  member  = "serviceAccount:${var.service_account_email}"
}
