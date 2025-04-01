variable "account_database" {
  type        = string
  description = "Name of the MongoDB's database to store account information."
  default     = null
}

variable "assets_bucket" {
  type        = string
  description = "Name of the assets' GCS bucket."
}

variable "auth0_domain" {
  type        = string
  description = "Domain of the Auth0 tenant."
}

variable "auth0_client_client_secret_id" {
  type        = string
  description = "Secret Manager ID of the Auth0 SPA client."
}

variable "auth_client_m2m" {
  type = object({
    client_id     = string
    client_secret = string
  })
}

variable "auth_client_spa" {
  type = object({
    client_id = string
  })
}

variable "cloudbuild_service_account_email" {
  type        = string
  description = "Cloud Build's service account."
}

# Currently, we are only using a different database for Re:Earth production.
variable "database" {
  type        = string
  description = "MongoDB's database to store the application's data."
  default     = "reearth_cms"
}

variable "database_secret_id" {
  type        = string
  description = "Secret ID of the database connection URL for MongoDB"
}

variable "domain" {
  type        = string
  description = "Custom domain to be used for Re:Earth."
}

variable "image" {
  type        = string
  description = "Image of the Cloud Run service."
}

variable "name" {
  type        = string
  description = "Name of the Cloud Run service."
  default     = "cms-api"
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
    condition     = contains(["asia-northeast1", "southamerica-east1"], var.region)
    error_message = "Environment variable must be either 'asia-northeast1' or 'southamerica-east1'."
  }
}

variable "service_account_email" {
  type        = string
  description = "Email of the service account to be used for Cloud Run."

  validation {
    condition     = can(regex("^[a-z0-9-_]+@[a-z0-9-_.]+$", var.service_account_email))
    error_message = "Service account email must be in the format of <name>@<domain>."
  }
}

# Useful when a Cloud Build requires a different connection string than that used by Cloud Run application.
# Otherwise, pass the same value as `database_secret_id` variable.
variable "task_database_secret_id" {
  type        = string
  description = "Secret ID of the database connection URL for MongoDB that is used for tasks(Cloud Build)."
}

variable "task_region" {
  type        = string
  description = "Region to run Cloud Build tasks."
  default     = null

  validation {
    condition     = contains(["asia-northeast1", "southamerica-east1"], var.task_region)
    error_message = "Environment variable must be either 'asia-northeast1' or 'southamerica-east1'."
  }
}

variable "worker_pool_id" {
  type        = string
  description = "Private pool ID of Cloud Build."
  default     = null

  validation {
    condition     = var.worker_pool_id != null && var.task_region != null
    error_message = "`task_region` must be specified when `worker_pool_id` is set."
  }
}
