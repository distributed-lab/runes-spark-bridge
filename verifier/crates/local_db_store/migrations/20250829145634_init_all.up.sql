BEGIN TRANSACTION;

CREATE SCHEMA verifier;

CREATE TABLE IF NOT EXISTS verifier.user_state
(
    user_public_key TEXT NOT NULL,
    signing_state JSONB NOT NULL,
    PRIMARY KEY (user_public_key)
);

CREATE TABLE IF NOT EXISTS verifier.user_session_state
(
    user_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    session_state JSONB NOT NULL,
    PRIMARY KEY (user_id)
);

COMMIT;