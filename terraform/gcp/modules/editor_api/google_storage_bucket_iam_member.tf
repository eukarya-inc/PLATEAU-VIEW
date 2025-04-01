# To manage with Terraform, the service account needs to have the `roles/storage.admin` role to 
# view the policy and viewer role is not enough.
# We can create a custom role with only `storage.buckets.getIamPolicy` permission but for simplicity,
# we use `roles/storage.admin` role here.
resource "google_storage_bucket_iam_member" "editor_static_allusers" {
  bucket = google_storage_bucket.editor_static.name
  role   = "roles/storage.objectViewer"
  member = "allUsers"
}


resource "google_storage_bucket_iam_member" "editor_static_serviceaccount" {
  bucket = google_storage_bucket.editor_static.name
  role   = "roles/storage.objectViewer"
  member = "serviceAccount:service-${data.google_project.project.number}@compute-system.iam.gserviceaccount.com"
}
