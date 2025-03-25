resource "google_service_account" "plateauview_api" {
  project      = data.google_project.project.project_id
  account_id   = "plateauview-api"
  display_name = "plateauview-api"
}

resource "google_service_account" "plateauview_geo" {
  project      = data.google_project.project.project_id
  account_id   = "plateauview-geo"
  display_name = "plateauview-geo"
}


resource "google_service_account" "cms_api" {
  project      = data.google_project.project.project_id
  account_id   = "cms-api"
  display_name = "cms-api"
}

resource "google_service_account" "cms_cloudbuild" {
  project      = data.google_project.project.project_id
  account_id   = "cms-cloudbuild"
  display_name = "cms-cloudbuild"
}

resource "google_service_account" "cms_web" {
  project      = data.google_project.project.project_id
  account_id   = "cms-web"
  display_name = "cms-web"
}

resource "google_service_account" "cms_worker" {
  project      = data.google_project.project.project_id
  account_id   = "cms-worker"
  display_name = "cms-worker"
}

resource "google_service_account" "plateau_tiles" {
  project      = data.google_project.project.project_id
  account_id   = "plateau-tiles"
  display_name = "plateau-tiles"
}

resource "google_service_account" "editor_api" {
  project      = data.google_project.project.project_id
  account_id   = "editor-api"
  display_name = "editor-api"
}

resource "google_service_account" "editor_web" {
  project      = data.google_project.project.project_id
  account_id   = "editor-web"
  display_name = "editor-web"
}
