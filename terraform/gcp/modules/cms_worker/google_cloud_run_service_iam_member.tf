resource "google_cloud_run_service_iam_member" "cms_worker_noauth" {
  location = google_cloud_run_v2_service.cms_worker.location
  project  = google_cloud_run_v2_service.cms_worker.project
  service  = google_cloud_run_v2_service.cms_worker.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}
