resource "google_compute_resource_policy" "plateau_tiles" {
  project = data.google_project.project.project_id
  name    = "plateau-tiles"
  region  = "asia-northeast1"

  snapshot_schedule_policy {
    schedule {
      daily_schedule {
        days_in_cycle = 1
        start_time    = "01:00" # UTC (10:00 JST)
      }
    }
  }
}
