#!/bin/sh
set -eu

APP_PASSWORD="$(cat /run/secrets/db_app_password)"
psql -v ON_ERROR_STOP=1 -v app_password="$APP_PASSWORD" --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<'SQL'
SELECT format('CREATE ROLE horasculo_app LOGIN PASSWORD %L', :'app_password') \gexec
GRANT USAGE ON SCHEMA public TO horasculo_app;
ALTER DEFAULT PRIVILEGES FOR ROLE horasculo_migrator IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO horasculo_app;
SQL
