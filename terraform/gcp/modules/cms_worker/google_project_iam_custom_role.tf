resource "google_project_iam_custom_role" "cms_worker" {
  role_id     = "reearthcmsworker"
  project     = data.google_project.project.project_id
  title       = "reearth-cms-worker"
  description = "IAM role for reearth-cms-worker"
  stage       = "GA"
  permissions = [
    "pubsub.topics.publish",
    "secretmanager.versions.access",
    "storage.objects.create",
    "storage.objects.delete",
    "storage.objects.get",
    "storage.objects.update",
  ]
}
