#!/bin/bash
set -euo pipefail
umask 077

APP_DIR=/opt/horasculo
CONFIG_FILE=/etc/horasculo/production.env
test -f "$CONFIG_FILE"
set -a
source "$CONFIG_FILE"
set +a

: "${AWS_REGION:?}"
: "${DOMAIN:?}"
: "${ENTRA_TENANT_ID:?}"
: "${ENTRA_CLIENT_ID:?}"
: "${CLOUDWATCH_LOG_GROUP:?}"
: "${ALERT_TOPIC:?}"

export HORASCULO_SECRETS_DIR=/run/horasculo/secrets
install -d -m 0700 "$HORASCULO_SECRETS_DIR"
for name in db_admin_password db_app_password database_url migration_database_url entra_client_secret; do
    secret_value="$(aws secretsmanager get-secret-value \
        --region "$AWS_REGION" \
        --secret-id "horasculo-prod/$name" \
        --query SecretString --output text)"
    printf '%s' "$secret_value" > "$HORASCULO_SECRETS_DIR/$name"
    unset secret_value
    chmod 0400 "$HORASCULO_SECRETS_DIR/$name"
    case "$name" in
        db_admin_password|db_app_password) chown 70:70 "$HORASCULO_SECRETS_DIR/$name" ;;
        *) chown 65532:65532 "$HORASCULO_SECRETS_DIR/$name" ;;
    esac
done

cd "$APP_DIR"
docker compose -f compose.prod.yaml up -d --build
install -m 0644 ops/horasculo-backup.service /etc/systemd/system/
install -m 0644 ops/horasculo-backup.timer /etc/systemd/system/
install -m 0644 ops/horasculo-restore-check.service /etc/systemd/system/
install -m 0644 ops/horasculo-restore-check.timer /etc/systemd/system/
install -m 0644 ops/horasculo-deploy.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable horasculo-deploy.service
systemctl enable --now horasculo-backup.timer horasculo-restore-check.timer
