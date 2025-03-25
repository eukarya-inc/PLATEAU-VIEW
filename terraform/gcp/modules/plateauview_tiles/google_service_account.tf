resource "google_service_account" "plateauview_tiles" {
  project      = data.google_project.project.project_id
  account_id   = "plateauview-tiles"
  display_name = "plateauview-tiles"
}
