#!/bin/bash
set -euo pipefail
source /etc/horasculo/production.env
: "${AWS_REGION:?}"
: "${BACKUP_BUCKET:?}"
: "${ALERT_TOPIC:?}"

report_failure() {
    aws sns publish --region "$AWS_REGION" --topic-arn "$ALERT_TOPIC" \
        --subject 'Horasculo: falha no teste de restauração' \
        --message 'A restauração mensal falhou. Consulte o serviço horasculo-restore-check no journal da VM.' >/dev/null || true
}
trap report_failure ERR

key="$(aws s3api list-objects-v2 --region "$AWS_REGION" --bucket "$BACKUP_BUCKET" \
    --prefix daily/ --query 'sort_by(Contents,&LastModified)[-1].Key' --output text)"
test -n "$key"
test "$key" != "None"
name="horasculo-restore-check-$$"
cleanup() { docker rm -f "$name" >/dev/null 2>&1 || true; }
trap cleanup EXIT

docker run -d --name "$name" --network none \
    -e POSTGRES_HOST_AUTH_METHOD=trust \
    postgres:17-alpine@sha256:a77aa70e641402a21411b92b721c58d10f23b50ae5fdd2333efd22d166e1e5f7 >/dev/null
for attempt in $(seq 1 30); do
    if docker exec "$name" pg_isready -U postgres >/dev/null 2>&1; then break; fi
    sleep 1
done
docker exec "$name" pg_isready -U postgres >/dev/null
aws s3 cp "s3://$BACKUP_BUCKET/$key" - --region "$AWS_REGION" \
    | docker exec -i "$name" pg_restore -U postgres -d postgres --no-owner --no-acl
docker exec "$name" psql -U postgres -d postgres -tAc 'SELECT count(*) FROM users' >/dev/null
trap - ERR
