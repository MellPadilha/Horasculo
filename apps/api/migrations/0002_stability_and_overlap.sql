CREATE EXTENSION IF NOT EXISTS btree_gist;

ALTER TABLE time_entries
    ADD CONSTRAINT time_entries_no_user_overlap
    EXCLUDE USING gist (
        user_id WITH =,
        tstzrange(starts_at, ends_at, '[)') WITH &&
    );

CREATE INDEX time_entries_user_ends_idx ON time_entries(user_id, ends_at);
CREATE INDEX time_entries_starts_ends_idx ON time_entries(starts_at, ends_at);

ALTER TABLE sessions DROP CONSTRAINT sessions_user_id_fkey;
ALTER TABLE sessions
    ADD CONSTRAINT sessions_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE weekly_goals DROP CONSTRAINT weekly_goals_user_id_fkey;
ALTER TABLE weekly_goals
    ADD CONSTRAINT weekly_goals_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

CREATE TABLE login_attempts (
    identifier text PRIMARY KEY,
    attempts integer NOT NULL DEFAULT 0,
    window_started timestamptz NOT NULL DEFAULT now()
);
