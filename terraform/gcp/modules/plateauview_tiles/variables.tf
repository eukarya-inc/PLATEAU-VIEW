variable "config_url" {
  type        = string
  description = "タイルサーバーの設定JSONのURL"
}

variable "cors_origins" {
  type        = string
  default     = "*"
  description = "許可するCORSオリジン（カンマ区切りまたは*）"
}

variable "domain" {
  type        = string
  description = "PLATEAU VIEWを提供するドメイン名"
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

variable "resources" {
  description = "Cloud Runサービスのリソース設定"
  default = {
    limits = {
      cpu    = "2000m"
      memory = "2Gi"
    }
  }

  type = object({
    limits = object({
      cpu    = string
      memory = string
    })
  })
}

variable "service_account_email" {
  type        = string
  description = "Cloud Runサービスに使用するサービスアカウントのメールアドレス"

  validation {
    condition     = can(regex("^[a-z0-9-_]+@[a-z0-9-_.]+$", var.service_account_email))
    error_message = "サービスアカウントのメールアドレスは <name>@<domain> の形式である必要があります。"
  }
}

variable "tile_cache_url" {
  type        = string
  default     = ""
  description = "タイルキャッシュ用のストレージURL (gs://, s3://, r2://, file://)"
}

variable "cache_size_mb" {
  type        = number
  default     = 512
  description = "メモリキャッシュサイズ（MB）"
}

variable "max_instance_count" {
  type        = number
  default     = 20
  description = "Cloud Runの最大インスタンス数"
}
