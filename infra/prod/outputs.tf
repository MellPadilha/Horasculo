output "instance_id" { value = aws_instance.app.id }
output "public_ip" { value = aws_eip.app.public_ip }
output "backup_bucket" { value = aws_s3_bucket.backup.bucket }
output "backup_kms_key" { value = aws_kms_key.backup.arn }
output "cloudwatch_log_group" { value = aws_cloudwatch_log_group.app.name }
output "alert_topic" { value = aws_sns_topic.alerts.arn }
output "secret_names" {
  value = { for key, secret in aws_secretsmanager_secret.app : key => secret.name }
}
