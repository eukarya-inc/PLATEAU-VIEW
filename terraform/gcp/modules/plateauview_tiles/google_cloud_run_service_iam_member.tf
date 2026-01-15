resource "google_cloud_run_service_iam_member" "plateauview_tiles_noauth" {
  location = google_cloud_run_v2_service.plateauview_tiles.location
  project  = google_cloud_run_v2_service.plateauview_tiles.project
  service  = google_cloud_run_v2_service.plateauview_tiles.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}
