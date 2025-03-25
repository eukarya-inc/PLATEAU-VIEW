resource "google_cloud_run_service_iam_member" "editor_web_noauth" {
  project  = google_cloud_run_v2_service.editor_web.project
  location = google_cloud_run_v2_service.editor_web.location
  service  = google_cloud_run_v2_service.editor_web.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}
