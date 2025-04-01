module "plateauview_tiles" {
  source = "./modules/plateauview_tiles"

  domain                = var.domain
  dns_managed_zone_name = google_dns_managed_zone.zone.name
  gcp_project_id        = data.google_project.project.project_id
  gcp_region            = var.gcp_region
  prefix                = var.prefix
}
