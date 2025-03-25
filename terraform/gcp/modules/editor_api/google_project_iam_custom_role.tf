resource "google_project_iam_custom_role" "editor" {
  role_id     = "PlateauReearthVisualizer"
  title       = "PlateauReearthVisualizer"
  project     = data.google_project.project.project_id
  description = "IAM Role for RE:Eearth Visualizer"
  stage       = "GA"
  permissions = [
    "cloudprofiler.profiles.create",
    "cloudprofiler.profiles.update",
    "pubsub.topics.publish",
    "secretmanager.versions.access",
    "storage.objects.create",
    "storage.objects.delete",
    "storage.objects.get",
    "storage.objects.update",
    "storage.objects.list",
  ]
}
