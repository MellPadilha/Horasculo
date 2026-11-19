variable "aws_region" { type = string }
variable "domain" { type = string }
variable "route53_zone_id" { type = string }
variable "alert_email" { type = string }
variable "instance_type" {
  type    = string
  default = "t3.medium"
}

variable "environment" {
  type    = string
  default = "prod"
}
