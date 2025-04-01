resource "google_project_iam_custom_role" "batch_service_role" {
  project     = data.google_project.project.project_id
  role_id     = "batchServiceRole"
  title       = "batchServiceRole"
  description = "Custom role for Cloud Batch service account with minimal permissions"
  stage       = "GA"
  permissions = [
    "storage.buckets.get",
    "storage.objects.get",
    "storage.objects.list",
    "storage.objects.create",

    "logging.logEntries.create",

    "batch.jobs.create",
    "batch.jobs.get",
    "batch.jobs.list",
    "batch.jobs.delete",
    "batch.tasks.get",
    "batch.states.report",

    "compute.instances.get",
    "compute.instances.list",
    "compute.machineTypes.get",
    "compute.machineTypes.list",
    "compute.zones.get",

    "artifactregistry.repositories.get",
    "artifactregistry.repositories.list",
    "artifactregistry.repositories.downloadArtifacts",
    "artifactregistry.files.get",
    "artifactregistry.files.list"
  ]
}
