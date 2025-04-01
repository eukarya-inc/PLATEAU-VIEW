resource "google_compute_backend_bucket" "editor_static" {
  project     = data.google_project.project.project_id
  name        = "editor-static-backend"
  bucket_name = google_storage_bucket.editor_static.name
  enable_cdn  = true
  cdn_policy {
    signed_url_cache_max_age_sec = 7200
  }
}
