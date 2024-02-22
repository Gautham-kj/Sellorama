CREATE TABLE IF NOT EXISTS "session"(
    session_id uuid UNIQUE NOT NULL DEFAULT uuid_generate_v4(),
    user_id uuid NOT NULL,
    expiry TIMESTAMP NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "user"(user_id) ON DELETE CASCADE
);