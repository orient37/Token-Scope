CREATE TABLE generation_samples(
 session_id TEXT NOT NULL REFERENCES sessions(id), turn_id TEXT NOT NULL,
 completed_at TEXT NOT NULL, duration_ms TEXT, first_token_ms TEXT,
 output_tokens TEXT, reasoning_tokens TEXT, response_key TEXT,
 eligible INTEGER NOT NULL,
 PRIMARY KEY(session_id,turn_id)
);
CREATE INDEX generation_recent ON generation_samples(completed_at DESC);
CREATE INDEX usage_generation_turn ON usage_events(session_id,turn_id);
INSERT INTO schema_migrations VALUES(6);
