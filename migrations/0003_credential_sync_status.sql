-- Outcome of the latest sync attempt, so users can see why data stopped arriving.
-- last_sync_at is when the attempt was claimed (also used as the sync cooldown).
ALTER TABLE cgm_credentials
    ADD COLUMN last_sync_at TIMESTAMPTZ,
    ADD COLUMN last_sync_status VARCHAR(32),
    ADD COLUMN last_sync_error TEXT;
