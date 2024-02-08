--migration for sessions table
CREATE TABLE IF NOT EXISTS "sessions"(
    session_id uuid UNIQUE NOT NULL uuid_generate_v4(),
    user_id uuid UNIQUE NOT NULL,
    expiry TIMESTAMP NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "user"(user_id) ON DELETE CASCADE
);