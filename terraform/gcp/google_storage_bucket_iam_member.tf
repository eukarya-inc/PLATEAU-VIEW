
resource "google_storage_bucket_iam_binding" "cms_assets_public_read" {
  bucket = google_storage_bucket.cms_assets.name
  role   = "roles/storage.objectViewer"
  members = [
    "allUsers",
    "serviceAccount:service-${data.google_project.project.number}@compute-system.iam.gserviceaccount.com",
  ]
}

resource "google_storage_bucket_iam_member" "cerbos_is_cerbos_policy_object_admin" {
  bucket = google_storage_bucket.cerbos_policy.name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${google_service_account.cerbos.email}"
}

# Flow assets are served directly to browsers, so anonymous access is limited to read only.
resource "google_storage_bucket_iam_member" "plateau_flow_public_read" {
  bucket = google_storage_bucket.plateau_flow_bucket.name
  role   = "roles/storage.objectViewer"
  member = "allUsers"
}

# Write access is granted only to the service accounts of the services that run Flow.
resource "google_storage_bucket_iam_member" "plateau_flow_api_object_admin" {
  bucket = google_storage_bucket.plateau_flow_bucket.name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${google_service_account.plateau_flow_api.email}"
}

resource "google_storage_bucket_iam_member" "plateau_flow_subscriber_object_admin" {
  bucket = google_storage_bucket.plateau_flow_bucket.name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${google_service_account.plateau_flow_subscriber.email}"
}

resource "google_storage_bucket_iam_member" "plateau_flow_worker_batch_object_admin" {
  bucket = google_storage_bucket.plateau_flow_bucket.name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${google_service_account.plateau_flow_worker_batch.email}"
}

resource "google_storage_bucket_iam_member" "plateau_flow_websocket_public_read" {
  bucket = google_storage_bucket.plateau_flow_websocket_bucket.name
  role   = "roles/storage.objectViewer"
  member = "allUsers"
}

resource "google_storage_bucket_iam_member" "plateau_flow_websocket_object_admin" {
  bucket = google_storage_bucket.plateau_flow_websocket_bucket.name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${google_service_account.plateau_flow_websocket.email}"
}
