-- Agent telemetry stream (raw events)
CREATE TABLE IF NOT EXISTS agent_telemetry (
    time            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    agent_id        TEXT NOT NULL,
    run_id          TEXT NOT NULL,
    phase           TEXT,
    action_type     TEXT NOT NULL,
    provider        TEXT,
    model           TEXT,
    outcome         TEXT,
    latency_ms      INTEGER,
    tokens_in       INTEGER,
    tokens_out      INTEGER,
    drift_score     FLOAT,
    lab_ness_score  FLOAT
);

-- create_hypertable is idempotent when if_not_exists = true
SELECT create_hypertable('agent_telemetry', 'time', if_not_exists => TRUE);
CREATE INDEX IF NOT EXISTS idx_agent_telemetry_agent_time ON agent_telemetry (agent_id, time DESC);

-- D* snapshot per agent per window (pre-computed by Meta-Governor)
CREATE TABLE IF NOT EXISTS drift_bound_snapshots (
    time        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    agent_id    TEXT NOT NULL,
    d_star      FLOAT,
    alpha       FLOAT,
    gamma       FLOAT,
    window_min  INTEGER
);

SELECT create_hypertable('drift_bound_snapshots', 'time', if_not_exists => TRUE);

-- Retention policies (90 days raw, 365 days snapshots)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM timescaledb_information.jobs
        WHERE proc_name = 'policy_retention' AND hypertable_name = 'agent_telemetry'
    ) THEN
        PERFORM add_retention_policy('agent_telemetry', INTERVAL '90 days');
    END IF;
END;
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM timescaledb_information.jobs
        WHERE proc_name = 'policy_retention' AND hypertable_name = 'drift_bound_snapshots'
    ) THEN
        PERFORM add_retention_policy('drift_bound_snapshots', INTERVAL '365 days');
    END IF;
END;
$$;
