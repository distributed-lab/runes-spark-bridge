BEGIN TRANSACTION;

DROP TABLE IF EXISTS verifier.user_state;
DROP TABLE IF EXISTS verifier.user_session_state;

COMMIT;