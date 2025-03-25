resource "google_project_iam_member" "plateauview_api" {
  # At the beginning of the migration, we don't know the exact roles that need to be assigned to the service account.
  # Thus, it might have more roles than necessary but we can remove them later.
  for_each = toset([
    "roles/cloudprofiler.agent",
    "roles/cloudbuild.builds.builder",
    "roles/iam.serviceAccountTokenCreator",
    "roles/run.developer",
    "roles/pubsub.publisher",
    "roles/storage.objectAdmin",
  ])

  project = data.google_project.project.project_id
  member  = "serviceAccount:${google_service_account.plateauview_api.email}"
  role    = each.value
}

resource "google_project_iam_member" "plateauview_geo" {
  # At the beginning of the migration, we don't know the exact roles that need to be assigned to the service account.
  # Thus, it might have more roles than necessary but we can remove them later.
  for_each = toset([
    "roles/cloudprofiler.agent",
    "roles/cloudbuild.builds.builder",
    "roles/datastore.viewer",
    "roles/run.developer",
    "roles/pubsub.publisher",
    "roles/storage.objectAdmin",
  ])

  project = data.google_project.project.project_id
  member  = "serviceAccount:${google_service_account.plateauview_geo.email}"
  role    = each.value
}

resource "google_project_iam_member" "cms_api" {
  for_each = toset([
    "roles/cloudbuild.workerPoolUser",
    "roles/logging.logWriter",
  ])
  project = data.google_project.project.project_id
  member  = "serviceAccount:${google_service_account.cms_api.email}"
  role    = each.value
}

resource "google_project_iam_member" "cms_worker" {
  # At the beginning of the migration, we don't know the exact roles that need to be assigned to the service account.
  # Thus, it might have more roles than necessary but we can remove them later.
  for_each = toset([
    "roles/cloudprofiler.agent",
    "roles/cloudbuild.builds.builder",
    "roles/run.developer",
    "roles/pubsub.publisher",
    "roles/storage.objectAdmin",
  ])

  project = data.google_project.project.project_id
  member  = "serviceAccount:${google_service_account.cms_worker.email}"
  role    = each.value
}

resource "google_project_iam_member" "cms_cloudbuild" {
  for_each = toset([
    "roles/logging.logWriter",
  ])

  project = data.google_project.project.project_id
  member  = "serviceAccount:${google_service_account.cms_cloudbuild.email}"
  role    = each.value
}


resource "google_project_iam_member" "plateau_tiles" {
  for_each = toset(local.plateau_tiles.roles)
  project  = data.google_project.project.project_id
  member   = "serviceAccount:${google_service_account.plateau_tiles.email}"
  role     = each.value
}
