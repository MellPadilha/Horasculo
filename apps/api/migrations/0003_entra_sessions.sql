ALTER TABLE users DROP COLUMN password_hash;
ALTER TABLE users ADD COLUMN entra_tid uuid NOT NULL;
ALTER TABLE users ADD COLUMN entra_oid uuid NOT NULL;
ALTER TABLE users DROP CONSTRAINT users_username_key;
ALTER TABLE users ADD CONSTRAINT users_entra_identity_key UNIQUE (entra_tid, entra_oid);

ALTER TABLE sessions ADD COLUMN last_seen_at timestamptz NOT NULL DEFAULT now();
CREATE INDEX sessions_last_seen_idx ON sessions(last_seen_at);

DROP TABLE login_attempts;

CREATE TABLE auth_flows (
    state_hash text PRIMARY KEY,
    binding_hash text NOT NULL,
    nonce text NOT NULL,
    pkce_verifier text NOT NULL,
    expires_at timestamptz NOT NULL
);
CREATE INDEX auth_flows_expires_idx ON auth_flows(expires_at);

ALTER TABLE time_entries ADD CONSTRAINT time_entries_title_length CHECK (char_length(title) <= 200);
ALTER TABLE projects ADD CONSTRAINT projects_name_length CHECK (char_length(name) <= 120);
