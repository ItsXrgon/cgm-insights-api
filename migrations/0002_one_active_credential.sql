-- Every query filters readings by user; the (user_id, timestamp) unique
-- constraint's index already serves them. This one is never used.
DROP INDEX IF EXISTS idx_glucose_readings_timestamp;

-- Keep only the most recently updated active credential per user...
UPDATE cgm_credentials c
SET is_active = FALSE
WHERE c.is_active
  AND EXISTS (
      SELECT 1 FROM cgm_credentials o
      WHERE o.user_id = c.user_id
        AND o.is_active
        AND (o.updated_at, o.id) > (c.updated_at, c.id)
  );

-- ...and enforce it. An exclusion constraint (not a partial unique index) so it
-- can be checked at the end of the statement: activation flips the old row off
-- and the new row on in a single UPDATE.
ALTER TABLE cgm_credentials
    ADD CONSTRAINT cgm_credentials_one_active_per_user
    EXCLUDE USING btree (user_id WITH =) WHERE (is_active)
    DEFERRABLE INITIALLY IMMEDIATE;
