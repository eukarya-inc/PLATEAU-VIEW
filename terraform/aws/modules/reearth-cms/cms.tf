resource "aws_apprunner_service" "reearth_cms_server" {
  service_name = "${var.prefix}-reearth-cms-server"
  health_check_configuration {
    healthy_threshold   = 1
    interval            = 10
    path                = "/"
    protocol            = "TCP"
    timeout             = 5
    unhealthy_threshold = 5
  }
  instance_configuration {
    cpu               = "512"
    instance_role_arn = aws_iam_role.reearth_cms_server_instance.arn
    memory            = "1024"
  }
  network_configuration {
    ip_address_type = "IPV4"
    egress_configuration {
      egress_type = "DEFAULT"
    }
    ingress_configuration {
      is_publicly_accessible = true
    }
  }
  observability_configuration {
    observability_enabled = false
  }
  source_configuration {
    auto_deployments_enabled = true
    authentication_configuration {
      access_role_arn = aws_iam_role.reearth_cms_server_access.arn
    }
    image_repository {
      image_identifier      = var.cms_image_identifier
      image_repository_type = "ECR"
      image_configuration {
        port = "8080"

        runtime_environment_secrets = {
          "${local.reearth_cms_secret_cms_db}" = aws_ssm_parameter.reearth_cms_secret_reearth_cms_db.arn
        }


        runtime_environment_variables = {
          REEARTH_CMS_AUTHM2M_AUD             = var.cognito_user_pool_client_id
          REEARTH_CMS_AUTHM2M_ISS             = var.cognito_user_pool_endpoint
          REEARTH_CMS_AWSTASK_NOTIFYTOKEN     = random_string.reearth_cms_notify_token.result
          REEARTH_CMS_AWSTASK_TOPICARN        = aws_sns_topic.reearth_cms_decompress.arn
          REEARTH_CMS_AWSTASK_WEBHOOKARN      = aws_sns_topic.reearth_cms_webhook.arn
          REEARTH_CMS_COGNITO_CLIENTID        = var.cognito_user_pool_client_id
          REEARTH_CMS_COGNITO_REGION          = var.region
          REEARTH_CMS_COGNITO_USERPOOLID      = var.cognito_user_pool_id
          REEARTH_CMS_DEV                     = "true"
          REEARTH_CMS_GRAPHQL_COMPLEXITYLIMIT = "6000"
          REEARTH_CMS_ORIGINS                 = "https://${var.cms_domain}"
          REEARTH_CMS_S3_BUCKETNAME           = aws_s3_bucket.reearth_cms_assets.id
          REEARTH_CMS_DB_ACCOUNT              = "reearth-account"
          REEARTH_CMS_WEB_CONFIG = jsonencode(
            {
              api                         = "https://${var.cms_domain}/api",
              cognitoRegion               = "us-east-1",
              cognitoUserPoolId           = var.cognito_user_pool_id
              cognitoUserPoolWebClientId  = var.cognito_user_pool_client_id
              cognitoOauthScope           = "email, openid, profile, aws.cognito.signin.user.admin",
              cognitoOauthDomain          = var.cognito_auth_domain,
              cognitoOauthRedirectSignIn  = "https://${var.cms_domain}",
              cognitoOauthRedirectSignOut = "https://${var.cms_domain}",
              cognitoOauthResponseType    = "code",
              authProvider                = "cognito"
              cesiumIonAccessToken        = var.cesium_ion_access_token
              editorUrl                   = var.editor_url
              coverImageUrl               = var.reearth_cms_web_config.coverImageUrl
              logoUrl                     = var.reearth_cms_web_config.logoUrl
          })
        }
      }
    }
  }
}

resource "aws_iam_role" "reearth_cms_server_access" {
  name = "${title(var.prefix)}ReearthCMSServerECRAccessRole"
  assume_role_policy = jsonencode({
    Version = "2012-10-17",
    Statement = [
      {
        Effect = "Allow",
        Principal = {
          Service = "build.apprunner.amazonaws.com"
        },
        Action = "sts:AssumeRole"
      },
      {
        Effect = "Allow",
        Principal = {
          Service = "tasks.apprunner.amazonaws.com"
        },
        Action = "sts:AssumeRole"
      }
    ]
  })
}

resource "aws_iam_role" "reearth_cms_server_instance" {
  name = "${title(var.prefix)}ReearthCMSServerInstanceRole"
  assume_role_policy = jsonencode({
    Version = "2012-10-17",
    Statement = [
      {
        Effect = "Allow",
        Principal = {
          Service = "build.apprunner.amazonaws.com"
        },
        Action = "sts:AssumeRole"
      },
      {
        Effect = "Allow",
        Principal = {
          Service = "tasks.apprunner.amazonaws.com"
        },
        Action = "sts:AssumeRole"
      }
    ]
  })
}

resource "aws_iam_role_policy" "reearth_cms_server_access" {
  name = "${title(var.prefix)}ReearthCMSServerECRAccessPolicy"
  role = aws_iam_role.reearth_cms_server_access.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "ecr:GetDownloadUrlForLayer",
          "ecr:BatchCheckLayerAvailability",
          "ecr:BatchGetImage",
          "ecr:DescribeImages",
          "ecr:GetAuthorizationToken"
        ]
        Resource = "*"
      },
    ]
  })
}

resource "aws_iam_role_policy" "reearth_cms_server_instance" {
  name = "${title(var.prefix)}ReearthCMSServerInstancePolicy"
  role = aws_iam_role.reearth_cms_server_instance.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:DeleteObject",
        ]
        Resource = ["${aws_s3_bucket.reearth_cms_assets.arn}/*"]
      },
      {
        Effect = "Allow"
        Action = ["sns:Publish"]
        Resource = [
          aws_sns_topic.reearth_cms_decompress.arn,
          aws_sns_topic.reearth_cms_webhook.arn,
        ]
      },
      {
        Effect   = "Allow"
        Action   = ["ssm:GetParameters"]
        Resource = [aws_ssm_parameter.reearth_cms_secret_reearth_cms_db.arn]
      },
    ]
  })
}

locals {
  reearth_cms_secret_cms_db = "REEARTH_CMS_DB"
}

resource "aws_ssm_parameter" "reearth_cms_secret_reearth_cms_db" {
  name  = "/${var.prefix}/reearth-cms/${local.reearth_cms_secret_cms_db}"
  type  = "SecureString"
  value = var.mongodb_connection_string
}

resource "aws_apprunner_custom_domain_association" "reearth_cms_server" {
  service_arn          = aws_apprunner_service.reearth_cms_server.arn
  domain_name          = var.cms_domain
  enable_www_subdomain = false
}
