variable "account_database" {
  type        = string
  description = "Name of the MongoDB's database to store account information."
  default     = "reearth-account" # Backward compatibility
}

variable "assets_bucket" {
  type        = string
  description = "Name of the assets' GCS bucket."
}

variable "auth0_domain" {
  type        = string
  description = "Domain of the Auth0 tenant."
}

variable "auth0_client_id" {
  type        = string
  description = "Client ID of the Auth0 M2M client."
}

variable "auth0_client_secret_secret_id" {
  type        = string
  description = "Secret Manager ID of the Auth0 M2M client."
}

variable "database_secret_id" {
  type        = string
  description = "Secret ID of the database connection URL for MongoDB"
  default     = "reearth-db" # Backward compatibility
}

# Currently, we are only using a different database for Re:Earth production.
variable "database" {
  type        = string
  description = "MongoDB's database to store the application's data."
  default     = null
}

variable "domain" {
  type        = string
  description = "Custom domain to be used for Re:Earth."
}

variable "env" {
  type        = string
  description = "Name of the environment."

  validation {
    condition     = contains(["oss", "dev", "prod"], var.env)
    error_message = "Environment variable must be either 'oss', 'dev' or 'prod'."
  }
}

variable "ext_plugin_url" {
  description = "Value for the REEARTH_EXT_PLUGIN environment variable"
  type        = string
  default     = ""
}

variable "image" {
  type        = string
  description = "Image of the Re:Earth API."
}

variable "ip_address" {
  type        = string
  default     = ""
  description = "IP address of the Re:Earth load balancer."
}

variable "name" {
  type        = string
  description = "Name of the Cloud Run service."
  default     = "editor-api"
}

variable "origins" {
  type        = list(string)
  description = "CORS origins allowed."

  validation {
    condition     = alltrue([for origin in var.origins : can(regex("^(http|https)://", origin))])
    error_message = "CORS origins must start with http:// or https://."
  }
}

variable "redis_url_secret_id" {
  type        = string
  default     = ""
  description = "Secret ID of the Redis URL"
}

variable "resources" {
  description = "Resources configuration for the Cloud Run service."
  default = {
    limits = {
      cpu    = "1000m"
      memory = "256Mi"
    }
  }

  type = object({
    limits = object({
      cpu    = optional(string)
      memory = optional(string)
    })
  })
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
}

variable "prefix" {
  type        = string
  description = "作成されるリソース名のプレフィックス"
}

variable "service_account_email" {
  type        = string
  description = "Email of the service account to be used for Re:Earth."

  validation {
    condition     = can(regex("^[a-z0-9-_]+@[a-z0-9-_.]+$", var.service_account_email))
    error_message = "Service account email must be in the format of <name>@<domain>."
  }
}

variable "signup_secret_secret_id" {
  type        = string
  description = "Secret ID used for Re:Earth verify signup webhook."
}

variable "visualizer_domain" {
  type        = string
  default     = ""
  description = "Custom domain for visualizer to be used for Re:Earth Visualizer"
}
