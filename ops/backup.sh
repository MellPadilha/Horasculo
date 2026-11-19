#!/bin/bash
set -euo pipefail
set -a
source /etc/horasculo/production.env
set +a
export HORASCULO_SECRETS_DIR=/run/horasculo/secrets
: "${AWS_REGION:?}"
: "${BACKUP_BUCKET:?}"
: "${BACKUP_KMS_KEY:?}"

report_failure() {
    aws cloudwatch put-metric-data --region "$AWS_REGION" --namespace Horasculo \
        --metric-data MetricName=BackupSuccess,Value=0,Unit=Count || true
}
trap report_failure ERR

day="$(date -u +%F)"
key="daily/$day.dump"
cd /opt/horasculo
docker compose -f compose.prod.yaml exec -T postgres \
    pg_dump -U horasculo_migrator -d horasculo -Fc \
    | aws s3 cp - "s3://$BACKUP_BUCKET/$key" --region "$AWS_REGION" \
        --sse aws:kms --sse-kms-key-id "$BACKUP_KMS_KEY"
aws s3api head-object --bucket "$BACKUP_BUCKET" --key "$key" --region "$AWS_REGION" >/dev/null

if [ "$(date -u +%u)" = 7 ]; then
    aws s3 cp "s3://$BACKUP_BUCKET/$key" "s3://$BACKUP_BUCKET/weekly/$day.dump" \
        --region "$AWS_REGION" --sse aws:kms --sse-kms-key-id "$BACKUP_KMS_KEY"
fi
trap - ERR
aws cloudwatch put-metric-data --region "$AWS_REGION" --namespace Horasculo \
    --metric-data MetricName=BackupSuccess,Value=1,Unit=Count
