moved {
  from = google_cloud_run_service_iam_member.reearth_visualizer_web_noauth
  to   = google_cloud_run_service_iam_member.editor_web_noauth
}

moved {
  from = google_cloud_run_v2_service.reearth_visualizer_web
  to   = google_cloud_run_v2_service.editor_web
}

moved {
  from = google_compute_region_network_endpoint_group.reearth_visualizer_web
  to   = google_compute_region_network_endpoint_group.editor_web
}

moved {
  from = google_monitoring_service.reearth_visualizer_web
  to   = google_monitoring_service.editor_web
}

moved {
  from = google_monitoring_slo.reearth_visualizer_web_availability
  to   = google_monitoring_slo.editor_web_availability
}

moved {
  from = google_monitoring_slo.reearth_visualizer_web_latency
  to   = google_monitoring_slo.editor_web_latency
}

moved {
  from = google_monitoring_uptime_check_config.reearth_visualizer_web
  to   = google_monitoring_uptime_check_config.editor_web
}
