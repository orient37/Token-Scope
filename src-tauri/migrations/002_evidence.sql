ALTER TABLE usage_events ADD COLUMN raw_evidence TEXT;
ALTER TABLE usage_events ADD COLUMN normalization_version INTEGER NOT NULL DEFAULT 0;
UPDATE usage_events SET delta=NULL,quality='legacy_assignment_unverified';
INSERT INTO schema_migrations VALUES(2);
