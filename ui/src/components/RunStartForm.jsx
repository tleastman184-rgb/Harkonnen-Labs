import React, { useEffect, useState } from 'react';

const API_BASE = import.meta.env.VITE_API_BASE || 'http://localhost:3000/api';

/**
 * RunStartForm — lets the user pick a spec and product then POST /api/runs/start.
 * Calls onRunStarted(run) when the server accepts the request.
 *
 * Note: requires POST /api/runs/start on the server. While that endpoint is not
 * yet wired, the form renders and shows an informative error rather than crashing.
 */
export default function RunStartForm({ onRunStarted, onClose }) {
  const [spec, setSpec] = useState('');
  const [product, setProduct] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState('');

  // Close on Escape
  useEffect(() => {
    const handler = (e) => { if (e.key === 'Escape') onClose?.(); };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [onClose]);

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!spec.trim() || !product.trim()) {
      setError('Both spec path and product name are required.');
      return;
    }

    setSubmitting(true);
    setError('');

    try {
      const resp = await fetch(`${API_BASE}/runs/start`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ spec: spec.trim(), product: product.trim() }),
      });

      if (!resp.ok) {
        const text = await resp.text();
        throw new Error(`${resp.status}: ${text}`);
      }

      const run = await resp.json();
      onRunStarted?.(run);
      onClose?.();
    } catch (err) {
      setError(err.message);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="form-overlay">
      <div className="form-panel">
        <div className="form-header">
          <div>
            <div className="form-eyebrow">Harkonnen Labs</div>
            <h3>Start a Factory Run</h3>
          </div>
          <button className="form-close" onClick={onClose} type="button">✕</button>
        </div>

        <form onSubmit={handleSubmit} className="form-body">
          <label className="field">
            <span className="field-label">Spec file path</span>
            <input
              className="field-input"
              type="text"
              placeholder="factory/specs/examples/sample_feature.yaml"
              value={spec}
              onChange={(e) => setSpec(e.target.value)}
              disabled={submitting}
              autoFocus
            />
            <span className="field-hint">Path relative to the repo root, or absolute.</span>
          </label>

          <label className="field">
            <span className="field-label">Product name</span>
            <input
              className="field-input"
              type="text"
              placeholder="my-product"
              value={product}
              onChange={(e) => setProduct(e.target.value)}
              disabled={submitting}
            />
            <span className="field-hint">
              Name of a folder under <code>products/</code>, or use --product-path via CLI.
            </span>
          </label>

          {error && <div className="form-error">{error}</div>}

          <div className="form-actions">
            <button className="btn-cancel" type="button" onClick={onClose} disabled={submitting}>
              Cancel
            </button>
            <button className="btn-start" type="submit" disabled={submitting || !spec || !product}>
              {submitting ? 'Starting…' : 'Start Run'}
            </button>
          </div>
        </form>
      </div>

      <style jsx>{`
        .form-overlay {
          position: fixed;
          inset: 0;
          background: rgba(0, 0, 0, 0.6);
          backdrop-filter: blur(6px);
          z-index: 1100;
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 1rem;
        }
        .form-panel {
          background: #1a1d1f;
          border: 1px solid rgba(194, 163, 114, 0.2);
          border-radius: 20px;
          box-shadow: 0 32px 80px rgba(0, 0, 0, 0.55);
          width: min(480px, 100%);
          overflow: hidden;
        }
        .form-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          padding: 1.2rem 1.3rem 0.9rem;
          border-bottom: 1px solid rgba(255, 255, 255, 0.07);
        }
        .form-eyebrow {
          text-transform: uppercase;
          letter-spacing: 0.16em;
          font-size: 0.68rem;
          font-weight: 800;
          color: var(--accent-gold);
          margin-bottom: 0.3rem;
        }
        .form-header h3 {
          font-size: 1.15rem;
          font-weight: 700;
        }
        .form-close {
          background: none;
          border: 1px solid rgba(255, 255, 255, 0.12);
          color: var(--text-secondary);
          border-radius: 50%;
          width: 30px;
          height: 30px;
          cursor: pointer;
          font-size: 0.85rem;
          display: flex;
          align-items: center;
          justify-content: center;
        }
        .form-close:hover { color: var(--text-primary); }
        .form-body {
          padding: 1.2rem 1.3rem 1.3rem;
          display: flex;
          flex-direction: column;
          gap: 1rem;
        }
        .field {
          display: flex;
          flex-direction: column;
          gap: 0.4rem;
        }
        .field-label {
          font-size: 0.72rem;
          font-weight: 800;
          text-transform: uppercase;
          letter-spacing: 0.1em;
          color: var(--accent-gold);
        }
        .field-input {
          background: rgba(255, 255, 255, 0.04);
          border: 1px solid rgba(255, 255, 255, 0.1);
          border-radius: 10px;
          color: var(--text-primary);
          font: inherit;
          font-size: 0.9rem;
          padding: 0.72rem 0.85rem;
          transition: border-color 0.15s;
          outline: none;
        }
        .field-input:focus {
          border-color: rgba(194, 163, 114, 0.5);
        }
        .field-input:disabled { opacity: 0.55; }
        .field-hint {
          font-size: 0.72rem;
          color: var(--text-secondary);
          line-height: 1.4;
        }
        .field-hint code {
          font-family: var(--font-mono);
          background: rgba(255, 255, 255, 0.06);
          padding: 0.1rem 0.35rem;
          border-radius: 4px;
        }
        .form-error {
          background: rgba(120, 39, 30, 0.3);
          border: 1px solid rgba(199, 104, 76, 0.4);
          color: #f0c7bc;
          border-radius: 10px;
          padding: 0.7rem 0.85rem;
          font-size: 0.82rem;
          line-height: 1.45;
        }
        .form-actions {
          display: flex;
          justify-content: flex-end;
          gap: 0.65rem;
          margin-top: 0.25rem;
        }
        .btn-cancel {
          background: none;
          border: 1px solid rgba(255, 255, 255, 0.12);
          color: var(--text-secondary);
          border-radius: 10px;
          padding: 0.65rem 1.1rem;
          font: inherit;
          font-size: 0.84rem;
          cursor: pointer;
        }
        .btn-cancel:hover { color: var(--text-primary); border-color: rgba(255,255,255,0.25); }
        .btn-cancel:disabled { opacity: 0.4; cursor: default; }
        .btn-start {
          background: var(--accent-gold);
          border: none;
          color: #17191a;
          font-weight: 800;
          border-radius: 10px;
          padding: 0.65rem 1.3rem;
          font: inherit;
          font-size: 0.84rem;
          cursor: pointer;
          letter-spacing: 0.04em;
          transition: opacity 0.15s;
        }
        .btn-start:hover { opacity: 0.88; }
        .btn-start:disabled { opacity: 0.4; cursor: default; }
      `}</style>
    </div>
  );
}
