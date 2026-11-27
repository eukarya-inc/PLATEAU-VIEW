output "plateauview_cms_webhook_secret" {
  value     = random_password.plateauview_env["REEARTH_PLATEAUVIEW_CMS_WEBHOOK_SECRET"].result
  sensitive = true
}

output "plateauview_sdk_token" {
  value     = random_password.plateauview_env["REEARTH_PLATEAUVIEW_SDK_TOKEN"].result
  sensitive = true
}

output "plateauview_sidebar_token" {
  value     = random_password.plateauview_env["REEARTH_PLATEAUVIEW_SIDEBAR_TOKEN"].result
  sensitive = true
}


output "reearth_cms_server_custom_domain_association" {
  value = aws_apprunner_custom_domain_association.reearth_cms_server
}

output "plateauview_api_custom_domain_association" {
  value = aws_apprunner_custom_domain_association.plateauview_api
}

output "plateauview_geo_custom_domain_association" {
  value = aws_apprunner_custom_domain_association.plateauview_geo
}