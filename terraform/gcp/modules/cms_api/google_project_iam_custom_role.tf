resource "google_project_iam_custom_role" "cms" {
  role_id     = "cmsapi"
  project     = data.google_project.project.project_id
  title       = "cms-api"
  description = "IAM role for cms-api"
  stage       = "GA"
  permissions = [
    "cloudbuild.builds.create",
    "cloudprofiler.profiles.create",
    "cloudprofiler.profiles.update",
    "cloudtasks.tasks.create",
    "pubsub.topics.publish",
    "secretmanager.versions.access",
    "storage.objects.create",
    "storage.objects.delete",
    "storage.objects.get",
    "storage.objects.update",
  ]
}
