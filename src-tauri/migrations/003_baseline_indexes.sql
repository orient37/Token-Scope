ALTER TABLE usage_events ADD COLUMN carried_baseline TEXT;
CREATE INDEX usage_visible_time ON usage_events(event_at DESC,id DESC) WHERE normalization_version>=0;
CREATE INDEX usage_visible_model_time ON usage_events(model,event_at DESC,id DESC) WHERE normalization_version>=0;
CREATE INDEX usage_pending_session ON usage_events(session_id) WHERE quality='pending';
UPDATE usage_events SET quality='pending' WHERE normalization_version=2;
INSERT INTO schema_migrations VALUES(3);
