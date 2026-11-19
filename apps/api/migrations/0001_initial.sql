CREATE EXTENSION IF NOT EXISTS citext;

CREATE TYPE user_role AS ENUM ('admin', 'user');

CREATE TABLE users (
    id uuid PRIMARY KEY,
    username citext NOT NULL UNIQUE,
    name text NOT NULL,
    password_hash text NOT NULL,
    role user_role NOT NULL DEFAULT 'user',
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users(id),
    token_hash text NOT NULL UNIQUE,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX sessions_user_id_idx ON sessions(user_id);
CREATE INDEX sessions_expires_at_idx ON sessions(expires_at);

CREATE TABLE projects (
    id uuid PRIMARY KEY,
    name text NOT NULL,
    color char(7) NOT NULL,
    tint char(7) NOT NULL,
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE weekly_goals (
    user_id uuid PRIMARY KEY REFERENCES users(id),
    hours double precision NOT NULL DEFAULT 40 CHECK (hours >= 0.25 AND hours <= 168),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE time_entries (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users(id),
    project_id uuid NOT NULL REFERENCES projects(id),
    title text NOT NULL,
    starts_at timestamptz NOT NULL,
    ends_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT valid_time_range CHECK (ends_at > starts_at AND ends_at <= starts_at + interval '24 hours')
);
CREATE INDEX time_entries_user_starts_idx ON time_entries(user_id, starts_at);
CREATE INDEX time_entries_project_idx ON time_entries(project_id);
