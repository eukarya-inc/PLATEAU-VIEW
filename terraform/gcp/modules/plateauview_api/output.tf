output "plateauview_cms_webhook_secret" {
  value = random_password.plateauview_env["REEARTH_PLATEUVIEW_CMS_WEBHOOK_SECRET"].result
}

output "plateauview_sdk_token" {
  value = random_password.plateauview_env["REEARTH_PLATEUVIEW_SDK_TOKEN"].result
}

output "plateauview_sidebar_token" {
  value = random_password.plateauview_env["REEARTH_PLATEUVIEW_SIDEBAR_TOKEN"].result
}

output "plateauview_setup_token" {
  value = random_password.plateauview_env["REEARTH_PLATEUVIEW_SETUP_TOKEN"].result
}

output "plateauview_flow_token" {
  value = random_password.plateauview_env["REEARTH_PLATEUVIEW_FLOW_TOKEN"].result
}

output "network_endpoint_group" {
  value = google_compute_region_network_endpoint_group.plateauview_api
}
