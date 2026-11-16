# This Terraform configuration is for building PLATEAU VIEW in a simple setup to check that it works.
# It differs from the configuration of the production environment actually in operation.
# Review it against your own availability, security, and cost requirements before using it in production.

terraform {
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.18"
    }

    random = {
      source  = "hashicorp/random"
      version = "~> 3.6"
    }

    auth0 = {
      source  = "auth0/auth0"
      version = "~> 0.50"
    }
  }

  required_version = ">= v1.7.4"
}
