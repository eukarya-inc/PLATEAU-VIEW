variable "api_url" {
  type        = string
  description = "URL of the Re:Earth API."
  default     = ""
}

variable "auth0_audience" {
  type        = string
  description = "Audient of the Auth0 credential."
}

variable "auth0_client_id" {
  type        = string
  description = "Client ID of the Auth0 SPA client."
}

variable "auth0_domain" {
  type        = string
  description = "Domain of the Auth0 tenant."
}

variable "cesium_ion_token_secret_id" {
  type        = string
  description = "Cesium ion access token's Secret Manager secret ID."
  default     = ""
}

variable "cover_image_url" {
  type        = string
  description = "URL of the cover image."
  default     = ""
}

variable "editor_url" {
  type        = string
  description = "URL of the Re:Earth editor."
  default     = ""
}

variable "image" {
  type        = string
  description = "Image of the Re:Earth CMS web."
}

variable "logo_url" {
  type        = string
  description = "URL of the cover image."
  default     = ""
}

variable "favicon_url" {
  type        = string
  description = "URL of the favicon."
  default     = ""
}

variable "title" {
  type        = string
  description = "Title of HTML."
  default     = ""
}

variable "multi_tenant" {
  type        = string
  description = "Multi tenant configuration."
  default     = ""
}

variable "project" {
  description = "ID of the GCP project."
  type        = string

  validation {
    condition     = can(regex("^[a-z][a-z0-9-]{4,28}[a-z0-9]$", var.project))
    error_message = "Project ID must start with a lowercase letter, and can include lowercase letters, numbers, or hyphens. It must be between 6 and 30 characters long."
  }
}

variable "region" {
  type        = string
  default     = "asia-northeast1"
  description = "Region to host the resources."

  validation {
    condition     = contains(["asia-northeast1", "southamerica-east1", "us-central1"], var.region)
    error_message = "Environment variable must be either 'asia-northeast1', 'southamerica-east1', or 'us-central1'."
  }
}

variable "service_account_email" {
  type        = string
  description = "Email of the service account to be used for Re:Earth."

  validation {
    condition     = can(regex("^[a-z0-9-_]+@[a-z0-9-_.]+$", var.service_account_email))
    error_message = "Service account email must be in the format of <name>@<domain>."
  }
}
