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

variable "brand" {
  type = object({
    background = string
    logoUrl    = string
  })
  description = "Styling to change the brand."
  default     = null
}

variable "cesium_ion_token_secret_id" {
  type        = string
  description = "Cesium ion access token's Secret Manager secret ID."
  default     = ""
}

variable "cloud_api_url" {
  type        = string
  description = "URL of the Re:Earth Cloud API."
  default     = ""
}

variable "current_tos" {
  type        = string
  description = "Terms of service version"
  default     = ""
}

variable "developer_mode" {
  type        = bool
  description = "Run with developer mode."
  default     = false
}

variable "disable_workspace_management" {
  type        = bool
  description = "Disable workspace management."
  default     = null
}

variable "documentation_url" {
  type        = string
  description = "URL of the Re:Earth documentation."
  default     = ""
}

variable "early_access_admins" {
  type        = list(string)
  description = "URLs of the admins."
  default     = []
}

variable "extension_urls" {
  type        = list(string)
  description = "URLs of the Re:Earth extensions."
  default     = []
}

variable "favicon_url" {
  type        = string
  description = "URL of the favicon."
  default     = ""
}

variable "marketplace_url" {
  type        = string
  description = "URL of the Re:Earth editor."
  default     = ""
}

variable "multi_tenant" {
  type = map(object({
    auth0Audience = string
    auth0ClientId = string
    auth0Domain   = string
  }))
  description = "Multi tenant configuration."
  default     = null
}

variable "image" {
  type        = string
  description = "Image of the Re:Earth CMS web."
}

variable "ip" {
  type        = string
  description = "IP address of what...?" # FIXME: What is this?
  default     = ""
}

variable "name" {
  type        = string
  description = "Name of the Cloud Run service."
  default     = "reearth-visualizer-web"
}

variable "password_policy" {
  type = object({
    highSecurity = string
    lowSecurity  = string
    medSecurity  = string
    tooLong      = string
    tooShort     = string
  })
  description = "Password policy of the Re:Earth CMS."
  default     = null
}

variable "plugins" {
  type        = string
  description = "URL of the cover image."
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

variable "policy" {
  description = "Policy." # FIXME: What is this?
  type = object({
    limitNotifications = object({
      asset = object({
        en = string
        ja = optional(string)
      })
      createdProject = optional(object({
        en = string
        ja = optional(string)
      }))
      dataset = object({
        en = string
        ja = optional(string)
      })
      layer = object({
        en = string
        ja = optional(string)
      })
      member = object({
        en = string
        ja = optional(string)
      })
      publishProject = object({
        en = string
        ja = optional(string)
      })
    })
    modalDescription = object({
      en = string
      ja = optional(string)
    })
    modalTitle = object({
      en = string
      ja = optional(string)
    })
    url = object({
      en = string
      ja = optional(string)
    })
  })

  default = null
}

variable "published" {
  type        = string
  description = "URL of the published site."
  default     = ""
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

variable "title" {
  type        = string
  description = "Title of HTML."
  default     = ""
}

variable "unsafe_plugin_urls" {
  type        = list(string)
  description = "URLs of the unsafe plugins."
  default     = []
}
