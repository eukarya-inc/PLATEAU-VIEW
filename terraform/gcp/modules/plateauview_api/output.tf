output "plateauview_cms_webhook_secret" {
  value = random_password.plateauview_env["REEARTH_PLATEAUVIEW_CMS_WEBHOOK_SECRET"].result
}

output "plateauview_sdk_token" {
  value = random_password.plateauview_env["REEARTH_PLATEAUVIEW_SDK_TOKEN"].result
}

output "plateauview_sidebar_token" {
  value = random_password.plateauview_env["REEARTH_PLATEAUVIEW_SIDEBAR_TOKEN"].result
}

output "plateauview_flow_token" {
  value = random_password.plateauview_env["REEARTH_PLATEAUVIEW_FLOW_TOKEN"].result
}

output "network_endpoint_group" {
  value = google_compute_region_network_endpoint_group.plateauview_api
}
