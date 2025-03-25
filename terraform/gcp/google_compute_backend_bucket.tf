# resource "google_compute_backend_bucket" "plateau_app" {
#   project = data.google_project.project.project_id

#   bucket_name = data.google_storage_bucket.plateau_reearth_app.name
#   enable_cdn  = true
#   name        = "plateau-app"
# }

# resource "google_compute_backend_bucket" "plateau_assets" {
#   project = data.google_project.project.project_id

#   bucket_name = data.google_storage_bucket.assets_cms_plateau.name
#   enable_cdn  = true
#   name        = "plateau-assets"
# }

# resource "google_compute_backend_bucket" "plateau_static" {
#   project = data.google_project.project.project_id

#   bucket_name = data.google_storage_bucket.plateau_reearth_static.name
#   enable_cdn  = true
#   name        = "plateau-static"
# }

# TODO: Remove after migrating to `reearth_editor_web`.
# data "google_compute_backend_bucket" "reearth_editor_static" {
#   project =data.google_project.project.project_id
#   name    = "plateau-dev-editor-static-backend"
# }
