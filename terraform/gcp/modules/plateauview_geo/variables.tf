variable "domain" {
  type        = string
  description = "PLATEAU VIEWを提供するドメイン名"
}

variable "dns_managed_zone_name" {
  type        = string
  description = "Cloud DNSマネージドゾーンの名前"
}

variable "gcp_project_id" {
  type        = string
  description = "GCPプロジェクトのID"

  validation {
    condition     = can(regex("^[a-z][a-z0-9-]{4,28}[a-z0-9]$", var.gcp_project_id))
    error_message = "GCPプロジェクトIDは、小文字で始まり、小文字、数字、またはハイフンを含む必要があります。また、6文字以上30文字以下の長さである必要があります。"
  }
}

variable "gcp_region" {
  type        = string
  default     = "asia-northeast1"
  description = "GCPで使用するリージョン"
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

variable "tile_cache_bucket_name" {
  type        = string
  description = "タイルキャッシュ用のストレージバケット名"
}

