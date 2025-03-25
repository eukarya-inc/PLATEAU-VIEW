resource "google_cloud_run_service_iam_member" "plateauview_geo_noauth" {
  location = google_cloud_run_v2_service.plateauview_geo.location
  project  = google_cloud_run_v2_service.plateauview_geo.project
  service  = google_cloud_run_v2_service.plateauview_geo.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}
