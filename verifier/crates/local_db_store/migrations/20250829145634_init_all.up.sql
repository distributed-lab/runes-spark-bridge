BEGIN TRANSACTION;

CREATE SCHEMA verifier;

CREATE TABLE IF NOT EXISTS verifier.user_state
(
    user_pubkey TEXT NOT NULL,
    signing_state JSONB NOT NULL,
    PRIMARY KEY (user_pubkey)
);

CREATE TABLE IF NOT EXISTS verifier.user_session_state
(
    user_pubkey TEXT NOT NULL,
    session_uuid UUID NOT NULL,
    session_state JSONB NOT NULL,
    PRIMARY KEY (user_pubkey, session_uuid)
);

COMMIT;