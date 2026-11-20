-- Baseline matching the tables the app used to create at startup.
-- IF NOT EXISTS so databases created before migrations adopt this as-is.

CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cgm_credentials (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    cgm_type VARCHAR(50) NOT NULL,
    username VARCHAR(255) NOT NULL,
    password VARCHAR(255) NOT NULL,
    region VARCHAR(50),
    is_active BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS glucose_readings (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    value_mg_dl DOUBLE PRECISION NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    is_high BOOLEAN NOT NULL DEFAULT FALSE,
    is_low BOOLEAN NOT NULL DEFAULT FALSE,
    trend VARCHAR(50),
    device_id VARCHAR(255),
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT glucose_readings_user_timestamp_unique UNIQUE (user_id, timestamp)
);

CREATE INDEX IF NOT EXISTS idx_glucose_readings_timestamp
    ON glucose_readings (timestamp DESC);
