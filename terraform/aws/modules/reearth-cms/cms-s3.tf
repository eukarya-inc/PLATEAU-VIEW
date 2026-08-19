resource "aws_s3_bucket" "reearth_cms_assets" {
  bucket = "${var.prefix}-reearth-cms-asset"
}

resource "aws_s3_bucket_public_access_block" "reearth_cms_assets" {
  bucket = aws_s3_bucket.reearth_cms_assets.id

  block_public_acls  = true
  ignore_public_acls = true
  # The bucket policy intentionally grants anonymous read access to the assets,
  # so these two have to stay disabled.
  block_public_policy     = false
  restrict_public_buckets = false
}

resource "aws_s3_bucket_cors_configuration" "reearth_cms_assets" {
  bucket = aws_s3_bucket.reearth_cms_assets.id

  cors_rule {
    allowed_headers = ["*"]
    allowed_methods = ["PUT", "POST", "DELETE"]
    allowed_origins = ["https://${var.cms_domain}"]
    expose_headers  = ["ETag"]
  }

  depends_on = [aws_s3_bucket_public_access_block.reearth_cms_assets]
}


resource "aws_s3_bucket_policy" "reearth_cms_assets" {
  bucket = aws_s3_bucket.reearth_cms_assets.id
  policy = data.aws_iam_policy_document.reearth_cms_assets.json

  depends_on = [aws_s3_bucket_public_access_block.reearth_cms_assets]
}

data "aws_iam_policy_document" "reearth_cms_assets" {
  statement {
    principals {
      type        = "*"
      identifiers = ["*"]
    }

    # Anonymous access is read only. Uploads and deletions are done by the CMS
    # server and worker with their App Runner instance roles.
    actions = [
      "s3:GetObject",
    ]

    resources = [
      "${aws_s3_bucket.reearth_cms_assets.arn}/*",
    ]
  }
}