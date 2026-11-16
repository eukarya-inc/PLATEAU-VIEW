# This Terraform configuration is for building PLATEAU VIEW in a simple setup to check that it works.
# It differs from the configuration of the production environment actually in operation.
# Review it against your own availability, security, and cost requirements before using it in production.

terraform {
  required_version = ">= 1.1"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = "us-east-1"
}