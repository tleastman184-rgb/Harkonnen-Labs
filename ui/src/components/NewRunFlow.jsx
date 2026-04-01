import { useState } from 'react';

const API_BASE = import.meta.env.VITE_API_BASE || 'http://localhost:3000/api';

const STEP_LABELS = ['Describe', 'Review Spec', 'Launch'];

/**
 * 3-step modal: (1) describe intent → (2) review Scout-drafted YAML → (3) confirm & start
 */
export default function NewRunFlow({ onClose, onRunStarted }) {
  const [step, setStep] = useState(0);
  const [intent, setIntent] = useState('');
  const [product, setProduct] = useState('');
  const [specYaml, setSpecYaml] = useState('');
  const [specPath, setSpecPath] = useState('');
  const [specId, setSpecId] = useState('');
  const [drafting, setDrafting] = useState(false);
  const [draftError, setDraftError] = useState('');
  const [launching, setLaunching] = useState(false);
  const [launchError, setLaunchError] = useState('');

  // ── Step 1 → 2: Scout drafts the spec ────────────────────────────────────────

  async function draftSpec() {
    if (!intent.trim() || !product.trim()) return;
    setDrafting(true);
    setDraftError('');
    try {
      const res = await fetch(`${API_BASE}/scout/draft`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ intent: intent.trim(), product: product.trim() }),
      });
      if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
      const data = await res.json();
      setSpecYaml(data.spec_yaml);
      setSpecPath(data.spec_path);
      setSpecId(data.spec_id);
      setStep(1);
    } catch (err) {
      setDraftError(err.message);
    } finally {
      setDrafting(false);
    }
  }

  // ── Step 3: start the run ─────────────────────────────────────────────────────

  async function startRun() {
    setLaunching(true);
    setLaunchError('');
    try {
      const res = await fetch(`${API_BASE}/runs/start`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ spec: specPath, product: product.trim() }),
      });
      if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
      const data = await res.json();
      onRunStarted?.(data.run_id);
      onClose?.();
    } catch (err) {
      setLaunchError(err.message);
    } finally {
      setLaunching(false);
    }
  }

  return (
    <div className="nrf-backdrop" onClick={e => { if (e.target === e.currentTarget) onClose?.(); }}>
      <div className="nrf-modal">

        {/* Header */}
        <div className="nrf-header">
          <div className="nrf-eyebrow">New Run</div>
          <div className="nrf-steps">
            {STEP_LABELS.map((label, i) => (
              <div key={i} className={`nrf-step ${i === step ? 'active' : ''} ${i < step ? 'done' : ''}`}>
                <span className="nrf-step-num">{i < step ? '✓' : i + 1}</span>
                <span className="nrf-step-label">{label}</span>
                {i < STEP_LABELS.length - 1 && <span className="nrf-step-sep">›</span>}
              </div>
            ))}
          </div>
          <button className="nrf-close" onClick={onClose}>✕</button>
        </div>

        {/* ── Step 0: Describe intent ── */}
        {step === 0 && (
          <div className="nrf-body">
            <p className="nrf-lead">
              Tell Scout what you want to build. Be as specific as possible —
              it will draft a structured YAML spec for you to review.
            </p>

            <div className="nrf-field">
              <label className="nrf-label">Product name</label>
              <input
                className="nrf-input"
                type="text"
                placeholder="e.g. ceres-station, lamdet, my-service"
                value={product}
                onChange={e => setProduct(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && intent.trim() && draftSpec()}
              />
            </div>

            <div className="nrf-field">
              <label className="nrf-label">Describe your intent</label>
              <textarea
                className="nrf-textarea"
                rows={5}
                placeholder={`What should this run accomplish?\n\nExamples:\n· "Validate the LamDet corpus pipeline end-to-end using the Python example dataset"\n· "Add rate-limiting middleware to the API gateway and test under load"\n· "Verify that the causal annotation endpoint writes correct markdown to factory/memory"`}
                value={intent}
                onChange={e => setIntent(e.target.value)}
              />
            </div>

            {draftError && <div className="nrf-error">{draftError}</div>}

            <div className="nrf-actions">
              <button className="nrf-btn secondary" onClick={onClose}>Cancel</button>
              <button
                className="nrf-btn primary"
                onClick={draftSpec}
                disabled={!intent.trim() || !product.trim() || drafting}
              >
                {drafting ? (
                  <>
                    <span className="nrf-spinner" />
                    Scout is drafting…
                  </>
                ) : 'Draft Spec →'}
              </button>
            </div>
          </div>
        )}

        {/* ── Step 1: Review spec ── */}
        {step === 1 && (
          <div className="nrf-body">
            <div className="nrf-spec-meta">
              <div className="nrf-spec-id">{specId}</div>
              <div className="nrf-spec-path">{specPath}</div>
            </div>
            <p className="nrf-lead">
              Scout drafted this spec. Edit it directly before launching — or go back and rephrase your intent.
            </p>

            <div className="nrf-field nrf-field-grow">
              <label className="nrf-label">Spec YAML</label>
              <textarea
                className="nrf-textarea nrf-mono"
                rows={18}
                value={specYaml}
                onChange={e => setSpecYaml(e.target.value)}
              />
            </div>

            <div className="nrf-actions">
              <button className="nrf-btn secondary" onClick={() => setStep(0)}>← Back</button>
              <button className="nrf-btn primary" onClick={() => setStep(2)}>
                Looks good →
              </button>
            </div>
          </div>
        )}

        {/* ── Step 2: Confirm & launch ── */}
        {step === 2 && (
          <div className="nrf-body">
            <div className="nrf-confirm-block">
              <div className="nrf-confirm-icon">🚀</div>
              <div className="nrf-confirm-text">
                <p className="nrf-confirm-title">Ready to launch</p>
                <p className="nrf-confirm-sub">
                  Product: <strong>{product}</strong>
                </p>
                <p className="nrf-confirm-sub">
                  Spec: <code>{specId}</code>
                </p>
                <p className="nrf-confirm-desc">
                  The full 9-agent pipeline will run — Scout through Coobie.
                  You can monitor progress on the Factory Floor and annotate causes in the Workbench when it completes.
                </p>
              </div>
            </div>

            {launchError && <div className="nrf-error">{launchError}</div>}

            <div className="nrf-actions">
              <button className="nrf-btn secondary" onClick={() => setStep(1)}>← Edit Spec</button>
              <button
                className="nrf-btn launch"
                onClick={startRun}
                disabled={launching}
              >
                {launching ? (
                  <>
                    <span className="nrf-spinner" />
                    Launching…
                  </>
                ) : '⚡ Launch Run'}
              </button>
            </div>
          </div>
        )}

      </div>

      <style jsx>{`
        .nrf-backdrop {
          position: fixed;
          inset: 0;
          z-index: 3000;
          background: rgba(0,0,0,0.72);
          display: flex;
          align-items: center;
          justify-content: center;
          backdrop-filter: blur(4px);
        }

        .nrf-modal {
          width: 620px;
          max-width: calc(100vw - 2rem);
          max-height: calc(100vh - 4rem);
          background: #14161a;
          border: 1px solid rgba(255,255,255,0.1);
          border-radius: 20px;
          box-shadow: 0 32px 80px rgba(0,0,0,0.6);
          display: flex;
          flex-direction: column;
          overflow: hidden;
          font-family: 'IBM Plex Sans', 'Segoe UI', sans-serif;
          color: #eaeaea;
        }

        /* ── Header ── */
        .nrf-header {
          display: flex;
          align-items: center;
          gap: 1rem;
          padding: 0.9rem 1.2rem;
          border-bottom: 1px solid rgba(255,255,255,0.07);
          background: rgba(18,20,22,0.9);
          flex-shrink: 0;
        }
        .nrf-eyebrow {
          font-size: 0.62rem;
          font-weight: 800;
          text-transform: uppercase;
          letter-spacing: 0.16em;
          color: #c2a372;
          white-space: nowrap;
        }
        .nrf-steps {
          display: flex;
          align-items: center;
          gap: 0.3rem;
          flex: 1;
          flex-wrap: wrap;
        }
        .nrf-step {
          display: flex;
          align-items: center;
          gap: 0.3rem;
          opacity: 0.3;
          transition: opacity 0.15s;
        }
        .nrf-step.active { opacity: 1; }
        .nrf-step.done { opacity: 0.6; }
        .nrf-step-num {
          width: 20px;
          height: 20px;
          border-radius: 50%;
          background: rgba(255,255,255,0.08);
          border: 1px solid rgba(255,255,255,0.15);
          font-size: 0.62rem;
          font-weight: 800;
          display: flex;
          align-items: center;
          justify-content: center;
        }
        .nrf-step.active .nrf-step-num {
          background: rgba(194,163,114,0.15);
          border-color: rgba(194,163,114,0.5);
          color: #c2a372;
        }
        .nrf-step.done .nrf-step-num {
          background: rgba(143,174,124,0.12);
          border-color: rgba(143,174,124,0.4);
          color: #8fae7c;
        }
        .nrf-step-label {
          font-size: 0.7rem;
          font-weight: 700;
          text-transform: uppercase;
          letter-spacing: 0.08em;
        }
        .nrf-step-sep {
          font-size: 0.7rem;
          color: rgba(255,255,255,0.2);
          margin: 0 0.1rem;
        }
        .nrf-close {
          background: none;
          border: 1px solid rgba(255,255,255,0.1);
          color: rgba(255,255,255,0.4);
          border-radius: 50%;
          width: 28px;
          height: 28px;
          cursor: pointer;
          font-size: 0.78rem;
          display: flex;
          align-items: center;
          justify-content: center;
          flex-shrink: 0;
        }
        .nrf-close:hover { color: #fff; border-color: rgba(255,255,255,0.3); }

        /* ── Body ── */
        .nrf-body {
          padding: 1.2rem 1.4rem;
          display: flex;
          flex-direction: column;
          gap: 0.9rem;
          overflow-y: auto;
          flex: 1;
        }
        .nrf-lead {
          font-size: 0.84rem;
          color: rgba(255,255,255,0.55);
          line-height: 1.5;
          margin: 0;
        }
        .nrf-field {
          display: flex;
          flex-direction: column;
          gap: 0.35rem;
        }
        .nrf-field-grow { flex: 1; }
        .nrf-label {
          font-size: 0.65rem;
          font-weight: 800;
          text-transform: uppercase;
          letter-spacing: 0.12em;
          color: rgba(255,255,255,0.38);
        }
        .nrf-input, .nrf-textarea {
          background: rgba(255,255,255,0.04);
          border: 1px solid rgba(255,255,255,0.1);
          border-radius: 10px;
          color: #eaeaea;
          padding: 0.6rem 0.8rem;
          font-size: 0.88rem;
          font-family: inherit;
          transition: border-color 0.12s;
          width: 100%;
          box-sizing: border-box;
        }
        .nrf-input:focus, .nrf-textarea:focus {
          outline: none;
          border-color: rgba(194,163,114,0.5);
        }
        .nrf-textarea { resize: vertical; line-height: 1.5; }
        .nrf-textarea.nrf-mono {
          font-family: 'IBM Plex Mono', 'Fira Code', monospace;
          font-size: 0.78rem;
          line-height: 1.6;
        }

        /* ── Spec meta ── */
        .nrf-spec-meta {
          display: flex;
          align-items: center;
          gap: 0.75rem;
          padding: 0.4rem 0.6rem;
          background: rgba(255,255,255,0.03);
          border: 1px solid rgba(255,255,255,0.06);
          border-radius: 8px;
        }
        .nrf-spec-id {
          font-size: 0.78rem;
          font-weight: 800;
          color: #c2a372;
          font-family: monospace;
        }
        .nrf-spec-path {
          font-size: 0.68rem;
          font-family: monospace;
          color: rgba(255,255,255,0.3);
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        /* ── Confirm block ── */
        .nrf-confirm-block {
          display: flex;
          gap: 1.2rem;
          align-items: flex-start;
          background: rgba(255,255,255,0.03);
          border: 1px solid rgba(255,255,255,0.07);
          border-radius: 14px;
          padding: 1.2rem;
        }
        .nrf-confirm-icon {
          font-size: 2.2rem;
          flex-shrink: 0;
          line-height: 1;
          margin-top: 0.1rem;
        }
        .nrf-confirm-title {
          font-size: 1.05rem;
          font-weight: 800;
          margin: 0 0 0.45rem;
        }
        .nrf-confirm-sub {
          font-size: 0.82rem;
          color: rgba(255,255,255,0.6);
          margin: 0 0 0.2rem;
        }
        .nrf-confirm-sub strong { color: #c2a372; }
        .nrf-confirm-sub code {
          font-family: monospace;
          color: #5a8acc;
          background: rgba(90,138,204,0.1);
          padding: 0.1rem 0.35rem;
          border-radius: 4px;
        }
        .nrf-confirm-desc {
          font-size: 0.78rem;
          color: rgba(255,255,255,0.38);
          margin: 0.6rem 0 0;
          line-height: 1.5;
        }

        /* ── Error ── */
        .nrf-error {
          font-size: 0.78rem;
          color: #f0c7bc;
          background: rgba(120,39,30,0.35);
          border: 1px solid rgba(199,104,76,0.4);
          border-radius: 8px;
          padding: 0.55rem 0.75rem;
        }

        /* ── Actions ── */
        .nrf-actions {
          display: flex;
          justify-content: flex-end;
          gap: 0.6rem;
          padding-top: 0.3rem;
        }
        .nrf-btn {
          padding: 0.55rem 1.1rem;
          border-radius: 10px;
          font-size: 0.78rem;
          font-weight: 800;
          text-transform: uppercase;
          letter-spacing: 0.1em;
          cursor: pointer;
          display: flex;
          align-items: center;
          gap: 0.4rem;
          transition: all 0.12s;
          border: 1px solid;
        }
        .nrf-btn:disabled { opacity: 0.45; cursor: default; }
        .nrf-btn.secondary {
          background: rgba(255,255,255,0.04);
          border-color: rgba(255,255,255,0.1);
          color: rgba(255,255,255,0.5);
        }
        .nrf-btn.secondary:hover:not(:disabled) {
          background: rgba(255,255,255,0.08);
          color: rgba(255,255,255,0.8);
        }
        .nrf-btn.primary {
          background: rgba(194,163,114,0.1);
          border-color: rgba(194,163,114,0.4);
          color: #c2a372;
        }
        .nrf-btn.primary:hover:not(:disabled) {
          background: rgba(194,163,114,0.18);
          border-color: rgba(194,163,114,0.65);
        }
        .nrf-btn.launch {
          background: rgba(90,138,204,0.12);
          border-color: rgba(90,138,204,0.45);
          color: #5a8acc;
          padding: 0.6rem 1.4rem;
          font-size: 0.84rem;
        }
        .nrf-btn.launch:hover:not(:disabled) {
          background: rgba(90,138,204,0.22);
          border-color: rgba(90,138,204,0.7);
        }

        /* ── Spinner ── */
        .nrf-spinner {
          width: 12px;
          height: 12px;
          border: 2px solid rgba(255,255,255,0.15);
          border-top-color: currentColor;
          border-radius: 50%;
          animation: nrf-spin 0.65s linear infinite;
          flex-shrink: 0;
        }
        @keyframes nrf-spin { to { transform: rotate(360deg); } }
      `}</style>
    </div>
  );
}
