-- Materialize streaming views over TimescaleDB agent_telemetry
-- These views are deployed by the harmony service on startup.
-- Materialize reads via PostgreSQL replication protocol.

-- Drift alert: agents with > 5 tool failures or avg drift > 0.7 in a 15-minute window
CREATE OR REPLACE MATERIALIZED VIEW agent_drift_alerts AS
SELECT
    agent_id,
    COUNT(*) AS failure_count,
    AVG(drift_score) AS avg_drift,
    MAX(time) AS latest_event
FROM agent_telemetry
WHERE action_type = 'tool_execution'
  AND outcome = 'failure'
  AND time > mz_now() - INTERVAL '15 minutes'
GROUP BY agent_id
HAVING COUNT(*) > 5 OR AVG(drift_score) > 0.7;

-- D* monitoring: rolling 30-minute drift bound estimate
CREATE OR REPLACE MATERIALIZED VIEW drift_bound_monitor AS
SELECT
    agent_id,
    AVG(drift_score) AS alpha_estimate,
    COUNT(CASE WHEN outcome = 'success' THEN 1 END)::FLOAT /
        NULLIF(COUNT(*), 0) AS gamma_estimate,
    AVG(drift_score) / NULLIF(
        COUNT(CASE WHEN outcome = 'success' THEN 1 END)::FLOAT / NULLIF(COUNT(*), 0),
        0
    ) AS d_star
FROM agent_telemetry
WHERE time > mz_now() - INTERVAL '30 minutes'
GROUP BY agent_id;

-- Lab-ness degradation alert: avg < 0.6 over last hour
CREATE OR REPLACE MATERIALIZED VIEW lab_ness_alerts AS
SELECT
    agent_id,
    AVG(lab_ness_score) AS avg_lab_ness,
    MIN(lab_ness_score) AS min_lab_ness
FROM agent_telemetry
WHERE time > mz_now() - INTERVAL '60 minutes'
  AND lab_ness_score IS NOT NULL
GROUP BY agent_id
HAVING AVG(lab_ness_score) < 0.6;
