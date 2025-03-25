resource "google_cloud_run_service_iam_member" "editor_api_noauth" {
  project  = google_cloud_run_v2_service.editor_api.project
  location = google_cloud_run_v2_service.editor_api.location
  service  = google_cloud_run_v2_service.editor_api.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}
