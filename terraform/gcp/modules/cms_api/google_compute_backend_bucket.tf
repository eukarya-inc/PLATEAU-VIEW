resource "google_compute_backend_bucket" "assets" {
  project = data.google_project.project.project_id
  name    = google_storage_bucket.assets.name

  description = google_storage_bucket.assets.name
  bucket_name = google_storage_bucket.assets.name
  enable_cdn  = false
}

resource "google_compute_backend_bucket" "static" {
  project = data.google_project.project.project_id
  name    = google_storage_bucket.static.name

  description = google_storage_bucket.static.name
  bucket_name = google_storage_bucket.static.name
  enable_cdn  = false
}
