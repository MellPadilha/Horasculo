#!/bin/bash
set -euo pipefail
dnf update -y
dnf install -y docker curl
systemctl enable --now docker
mkdir -p /usr/local/lib/docker/cli-plugins /opt/horasculo
COMPOSE_VERSION=v5.8.0
BASE_URL="https://github.com/docker/compose/releases/download/${COMPOSE_VERSION}"
curl --fail --silent --show-error --location "${BASE_URL}/docker-compose-linux-x86_64" -o /usr/local/lib/docker/cli-plugins/docker-compose
curl --fail --silent --show-error --location "${BASE_URL}/checksums.txt" -o /tmp/compose-checksums.txt
EXPECTED="$(awk '$2 == "docker-compose-linux-x86_64" {print $1}' /tmp/compose-checksums.txt)"
test -n "$EXPECTED"
echo "$EXPECTED  /usr/local/lib/docker/cli-plugins/docker-compose" | sha256sum -c -
chmod 0755 /usr/local/lib/docker/cli-plugins/docker-compose
docker compose version
