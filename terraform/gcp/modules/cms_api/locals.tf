locals {
  api_domain     = "api.${local.cms_domain}"
  assets_domain  = "assets.${local.cms_domain}"
  cms_domain     = "cms.${var.domain}"
  reearth_domain = "reearth.${var.domain}"
  worker_domain  = "worker.${local.cms_domain}"
}

locals {
  plateauview_randoms = [
    "REEARTH_PLATEAUVIEW_CMS_WEBHOOK_SECRET",
    "REEARTH_PLATEAUVIEW_SECRET",
    "REEARTH_PLATEAUVIEW_SDK_TOKEN",
    "REEARTH_PLATEAUVIEW_SIDEBAR_TOKEN",
  ]
}
