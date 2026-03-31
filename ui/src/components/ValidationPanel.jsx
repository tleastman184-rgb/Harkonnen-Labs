import React from 'react';

function CheckRow({ result }) {
  const passed = result.passed;
  const color = passed ? '#8fae7c' : '#c7684c';
  return (
    <div className="check-row">
      <span className="check-icon" style={{ color }}>{passed ? '✓' : '✗'}</span>
      <div className="check-body">
        <div className="check-id">{result.scenario_id}</div>
        <div className="check-details">{result.details}</div>
      </div>
      <style jsx>{`
        .check-row {
          display: flex;
          gap: 0.65rem;
          padding: 0.55rem 0;
          border-top: 1px solid rgba(255,255,255,0.05);
          align-items: flex-start;
        }
        .check-row:first-of-type { border-top: none; }
        .check-icon {
          font-size: 0.9rem;
          font-weight: 900;
          flex-shrink: 0;
          margin-top: 0.05rem;
        }
        .check-id {
          font-size: 0.8rem;
          font-weight: 700;
          font-family: var(--font-mono);
          margin-bottom: 0.2rem;
        }
        .check-details {
          font-size: 0.74rem;
          color: var(--text-secondary);
          line-height: 1.45;
          word-break: break-word;
        }
      `}</style>
    </div>
  );
}

function HiddenCheckRow({ check }) {
  const color = check.passed ? '#8fae7c' : '#c7684c';
  return (
    <div className="h-check">
      <span style={{ color }}>{check.passed ? '✓' : '✗'}</span>
      <span className="h-check-kind">{check.kind}</span>
      <span className="h-check-detail">{check.details}</span>
      <style jsx>{`
        .h-check {
          display: flex;
          gap: 0.5rem;
          font-size: 0.72rem;
          padding: 0.3rem 0;
          color: var(--text-secondary);
          border-top: 1px solid rgba(255,255,255,0.04);
          align-items: baseline;
        }
        .h-check-kind {
          font-family: var(--font-mono);
          font-weight: 700;
          flex-shrink: 0;
        }
        .h-check-detail {
          line-height: 1.4;
          word-break: break-word;
        }
      `}</style>
    </div>
  );
}

export default function ValidationPanel({ validation, hiddenScenarios }) {
  if (!validation && !hiddenScenarios) {
    return null;
  }

  const visiblePassed = validation?.passed ?? null;
  const hiddenPassed = hiddenScenarios?.passed ?? null;

  return (
    <div className="validation-panel">
      {validation && (
        <section className="val-section">
          <div className="val-header">
            <span className="val-label">Visible Validation</span>
            <span
              className="val-badge"
              style={{
                background: visiblePassed ? 'rgba(143,174,124,0.15)' : 'rgba(199,104,76,0.15)',
                color: visiblePassed ? '#8fae7c' : '#c7684c',
                borderColor: visiblePassed ? 'rgba(143,174,124,0.4)' : 'rgba(199,104,76,0.4)',
              }}
            >
              {visiblePassed ? 'PASSED' : 'FAILED'}
            </span>
          </div>
          <div className="val-checks">
            {validation.results.map((r) => (
              <CheckRow key={r.scenario_id} result={r} />
            ))}
          </div>
        </section>
      )}

      {hiddenScenarios && (
        <section className="val-section">
          <div className="val-header">
            <span className="val-label">Hidden Scenarios</span>
            <span
              className="val-badge"
              style={{
                background: hiddenPassed ? 'rgba(143,174,124,0.15)' : 'rgba(199,104,76,0.15)',
                color: hiddenPassed ? '#8fae7c' : '#c7684c',
                borderColor: hiddenPassed ? 'rgba(143,174,124,0.4)' : 'rgba(199,104,76,0.4)',
              }}
            >
              {hiddenPassed ? 'PASSED' : 'FAILED'}
            </span>
          </div>
          {hiddenScenarios.results.map((scenario) => (
            <div key={scenario.scenario_id} className="hidden-scenario">
              <div className="hs-title">
                <span
                  style={{ color: scenario.passed ? '#8fae7c' : '#c7684c' }}
                >
                  {scenario.passed ? '✓' : '✗'}
                </span>
                {scenario.title || scenario.scenario_id}
              </div>
              {scenario.details && (
                <div className="hs-details">{scenario.details}</div>
              )}
              {scenario.checks?.length > 0 && (
                <div className="hs-checks">
                  {scenario.checks.map((c, i) => (
                    <HiddenCheckRow key={i} check={c} />
                  ))}
                </div>
              )}
            </div>
          ))}
        </section>
      )}

      <style jsx>{`
        .validation-panel {
          display: flex;
          flex-direction: column;
          gap: 1rem;
        }
        .val-section {
          background: rgba(22, 24, 26, 0.88);
          border: 1px solid rgba(229, 225, 216, 0.1);
          border-radius: 18px;
          padding: 1rem;
          box-shadow: 0 18px 36px rgba(0, 0, 0, 0.24);
        }
        .val-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 0.75rem;
        }
        .val-label {
          text-transform: uppercase;
          letter-spacing: 0.12em;
          font-size: 0.82rem;
          font-weight: 800;
          color: var(--accent-gold);
        }
        .val-badge {
          border: 1px solid;
          border-radius: 999px;
          padding: 0.22rem 0.65rem;
          font-size: 0.7rem;
          font-weight: 800;
          letter-spacing: 0.1em;
        }
        .val-checks {
          display: flex;
          flex-direction: column;
        }
        .hidden-scenario {
          border: 1px solid rgba(255,255,255,0.06);
          background: rgba(0,0,0,0.18);
          border-radius: 12px;
          padding: 0.72rem 0.85rem;
          margin-top: 0.55rem;
        }
        .hs-title {
          display: flex;
          gap: 0.55rem;
          font-size: 0.86rem;
          font-weight: 700;
          align-items: baseline;
          margin-bottom: 0.3rem;
        }
        .hs-details {
          font-size: 0.78rem;
          color: var(--text-secondary);
          line-height: 1.45;
          margin-bottom: 0.4rem;
        }
        .hs-checks {
          display: flex;
          flex-direction: column;
          padding-left: 0.5rem;
        }
      `}</style>
    </div>
  );
}
