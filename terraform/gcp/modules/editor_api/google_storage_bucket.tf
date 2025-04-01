resource "google_storage_bucket" "editor_static" {
  project       = data.google_project.project.project_id
  name          = "editor-static-${data.google_project.project.project_id}"
  location      = "ASIA"
  storage_class = "MULTI_REGIONAL"

  cors {
    max_age_seconds = 60
    method = [
      "GET",
      "PATCH",
      "POST",
      "PUT",
      "HEAD",
      "OPTIONS",
    ]
    origin = [
      "*"
    ]
    response_header = [
      "Access-Control-Allow-Origin",
      "Content-Type",
      "Content-Encoding",
    ]
  }

  website {
    main_page_suffix = "index.html"
    not_found_page   = "index.html"
  }
}
