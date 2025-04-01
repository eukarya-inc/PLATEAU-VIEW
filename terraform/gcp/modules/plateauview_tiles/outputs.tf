output "app_tile_cache_bucket_name" {
  value       = google_storage_bucket.app_tile_cache.name
  description = "タイルキャッシュ用のストレージバケット名"
}

output "compute_instance_group_manager" {
  value = google_compute_instance_group_manager.plateauview_tiles
}

output "compute_health_check" {
  value = google_compute_health_check.plateauview_tiles
}
