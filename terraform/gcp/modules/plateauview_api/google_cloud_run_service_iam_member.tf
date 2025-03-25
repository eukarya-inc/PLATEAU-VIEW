resource "google_cloud_run_service_iam_member" "plateauview_api_noauth" {
  location = google_cloud_run_v2_service.plateauview_api.location
  project  = google_cloud_run_v2_service.plateauview_api.project
  service  = google_cloud_run_v2_service.plateauview_api.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}
