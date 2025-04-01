locals {
  api_domain     = "api.${local.cms_domain}"
  assets_domain  = "assets.${local.cms_domain}"
  cms_domain     = "cms.${var.domain}"
  reearth_domain = "reearth.${var.domain}"
  worker_domain  = "worker.${local.cms_domain}"
}

locals {
  plateauview_randoms = [
    "REEARTH_PLATEUVIEW_CMS_WEBHOOK_SECRET",
    "REEARTH_PLATEUVIEW_SECRET",
    "REEARTH_PLATEUVIEW_SDK_TOKEN",
    "REEARTH_PLATEUVIEW_SIDEBAR_TOKEN",
  ]
}
