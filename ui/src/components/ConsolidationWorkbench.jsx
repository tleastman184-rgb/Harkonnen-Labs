import React, { useEffect, useState, useCallback } from 'react';

const API_BASE = import.meta.env.VITE_API_BASE || 'http://127.0.0.1:3057/api';

const KIND_LABEL = { lesson: 'Lesson', causal_link: 'Causal Link', pattern: 'Pattern' };
const KIND_COLOR = { lesson: '#c4922a', causal_link: '#4a9eff', pattern: '#9c6fde' };

const STATUS_CHIP = {
  pending:   { bg: 'rgba(255,255,255,0.07)', color: '#aaa',    label: 'Pending' },
  kept:      { bg: 'rgba(60,180,80,0.18)',   color: '#4ecb71', label: 'Kept'    },
  discarded: { bg: 'rgba(220,60,60,0.18)',   color: '#e05555', label: 'Discarded' },
};

function StatusChip({ status }) {
  const chip = STATUS_CHIP[status] || STATUS_CHIP.pending;
  return (
    <span style={{
      padding: '2px 8px', borderRadius: 99, fontSize: 11,
      background: chip.bg, color: chip.color, fontWeight: 600,
    }}>
      {chip.label}
    </span>
  );
}

function ConfidenceBar({ value }) {
  const pct = Math.round((value || 0) * 100);
  const hue = Math.round((value || 0) * 120);
  const color = `hsl(${hue}, 58%, 52%)`;
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
      <div style={{
        flex: 1, height: 4, borderRadius: 2,
        background: 'rgba(255,255,255,0.08)',
      }}>
        <div style={{ width: `${pct}%`, height: '100%', borderRadius: 2, background: color }} />
      </div>
      <span style={{ fontSize: 11, color, minWidth: 30, textAlign: 'right' }}>{pct}%</span>
    </div>
  );
}

function CandidateCard({ candidate, onKeep, onDiscard, onEdit, busy }) {
  const [expanded, setExpanded] = useState(false);
  const [editing, setEditing] = useState(false);
  const [editText, setEditText] = useState('');
  const kindColor = KIND_COLOR[candidate.kind] || '#888';

  function startEdit() {
    const content = candidate.edited_json ?? candidate.content_json;
    setEditText(JSON.stringify(content, null, 2));
    setEditing(true);
  }

  function submitEdit() {
    try {
      const parsed = JSON.parse(editText);
      onEdit(candidate.candidate_id, parsed);
      setEditing(false);
    } catch {
      alert('Invalid JSON — please fix before saving.');
    }
  }

  const isBusy = busy === candidate.candidate_id;

  return (
    <div style={{
      background: '#1e2224',
      border: `1px solid ${candidate.status === 'kept' ? 'rgba(60,180,80,0.3)' : candidate.status === 'discarded' ? 'rgba(220,60,60,0.15)' : 'rgba(255,255,255,0.07)'}`,
      borderRadius: 8,
      padding: '12px 14px',
      opacity: candidate.status === 'discarded' ? 0.5 : 1,
      transition: 'opacity 0.2s, border-color 0.2s',
    }}>
      {/* Header row */}
      <div style={{ display: 'flex', alignItems: 'flex-start', gap: 8, marginBottom: 6 }}>
        <span style={{
          fontSize: 10, fontWeight: 700, letterSpacing: 0.5,
          color: kindColor, textTransform: 'uppercase', paddingTop: 2,
        }}>
          {KIND_LABEL[candidate.kind] || candidate.kind}
        </span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{
            fontSize: 13, fontWeight: 500, color: '#e5e1d8',
            whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
          }}>
            {candidate.label}
          </div>
        </div>
        <StatusChip status={candidate.status} />
      </div>

      <ConfidenceBar value={candidate.confidence} />

      {/* Expand/collapse content */}
      <button
        onClick={() => setExpanded((e) => !e)}
        style={{
          background: 'none', border: 'none', cursor: 'pointer',
          color: '#888', fontSize: 11, padding: '4px 0 0',
        }}
      >
        {expanded ? '▲ hide detail' : '▼ show detail'}
      </button>

      {expanded && !editing && (
        <pre style={{
          fontSize: 11, color: '#aaa', background: '#161819',
          borderRadius: 4, padding: '8px 10px', marginTop: 6,
          overflowX: 'auto', maxHeight: 200, overflowY: 'auto',
        }}>
          {JSON.stringify(candidate.edited_json ?? candidate.content_json, null, 2)}
        </pre>
      )}

      {expanded && editing && (
        <div style={{ marginTop: 6 }}>
          <textarea
            value={editText}
            onChange={(e) => setEditText(e.target.value)}
            rows={10}
            style={{
              width: '100%', fontSize: 11, fontFamily: 'monospace',
              background: '#161819', color: '#e5e1d8',
              border: '1px solid rgba(255,255,255,0.12)', borderRadius: 4,
              padding: '8px 10px', boxSizing: 'border-box', resize: 'vertical',
            }}
          />
          <div style={{ display: 'flex', gap: 6, marginTop: 6 }}>
            <button className="wb-action-btn wb-save" onClick={submitEdit} disabled={isBusy}>
              Save edit
            </button>
            <button
              className="wb-action-btn wb-cancel"
              onClick={() => setEditing(false)}
              disabled={isBusy}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Action row */}
      {candidate.status !== 'kept' && candidate.status !== 'discarded' && !editing && (
        <div style={{ display: 'flex', gap: 6, marginTop: 8 }}>
          <button
            className="wb-action-btn wb-keep"
            onClick={() => onKeep(candidate.candidate_id)}
            disabled={isBusy}
          >
            {isBusy ? '…' : 'Keep'}
          </button>
          <button
            className="wb-action-btn wb-discard"
            onClick={() => onDiscard(candidate.candidate_id)}
            disabled={isBusy}
          >
            {isBusy ? '…' : 'Discard'}
          </button>
          <button
            className="wb-action-btn wb-edit"
            onClick={startEdit}
            disabled={isBusy}
          >
            Edit
          </button>
        </div>
      )}

      {candidate.status === 'kept' && !editing && (
        <div style={{ display: 'flex', gap: 6, marginTop: 8 }}>
          <button
            className="wb-action-btn wb-discard"
            onClick={() => onDiscard(candidate.candidate_id)}
            disabled={isBusy}
          >
            Undo keep
          </button>
          <button className="wb-action-btn wb-edit" onClick={startEdit} disabled={isBusy}>
            Edit
          </button>
        </div>
      )}
    </div>
  );
}

export default function ConsolidationWorkbench({ runId }) {
  const [data, setData] = useState(null);
  const [loading, setLoading] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [promoting, setPromoting] = useState(false);
  const [busy, setBusy] = useState('');
  const [error, setError] = useState('');
  const [promoted, setPromoted] = useState(null);
  const [filter, setFilter] = useState('all'); // 'all' | 'pending' | 'kept' | 'discarded'

  const load = useCallback(async () => {
    if (!runId) return;
    setLoading(true);
    setError('');
    try {
      const r = await fetch(`${API_BASE}/runs/${runId}/consolidation/candidates`);
      if (r.status === 404) { setData(null); return; }
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
      setData(await r.json());
    } catch (e) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }, [runId]);

  useEffect(() => { load(); }, [load]);

  async function handleGenerate() {
    setGenerating(true);
    setError('');
    try {
      const r = await fetch(`${API_BASE}/runs/${runId}/consolidation/candidates`, { method: 'POST' });
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
      const d = await r.json();
      setData(d);
    } catch (e) {
      setError(e.message);
    } finally {
      setGenerating(false);
    }
  }

  async function handleKeep(cid) {
    setBusy(cid);
    try {
      await fetch(`${API_BASE}/runs/${runId}/consolidation/candidates/${cid}/keep`, { method: 'POST' });
      await load();
    } finally { setBusy(''); }
  }

  async function handleDiscard(cid) {
    setBusy(cid);
    try {
      await fetch(`${API_BASE}/runs/${runId}/consolidation/candidates/${cid}/discard`, { method: 'POST' });
      await load();
    } finally { setBusy(''); }
  }

  async function handleEdit(cid, content) {
    setBusy(cid);
    try {
      await fetch(`${API_BASE}/runs/${runId}/consolidation/candidates/${cid}/edit`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content }),
      });
      await load();
    } finally { setBusy(''); }
  }

  async function handlePromote() {
    setPromoting(true);
    setError('');
    try {
      const r = await fetch(`${API_BASE}/runs/${runId}/consolidate`, { method: 'POST' });
      if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
      const d = await r.json();
      setPromoted(d);
      await load();
    } catch (e) {
      setError(e.message);
    } finally {
      setPromoting(false);
    }
  }

  const candidates = data?.candidates || [];
  const visible = filter === 'all' ? candidates : candidates.filter((c) => c.status === filter);
  const keptCount = candidates.filter((c) => c.status === 'kept').length;
  const pendingCount = candidates.filter((c) => c.status === 'pending').length;

  return (
    <div className="wb-root">
      {/* Header */}
      <div className="wb-header">
        <div>
          <div className="wb-title">Consolidation Workbench</div>
          <div className="wb-subtitle">
            Review what Coobie proposes to remember before anything enters durable memory.
          </div>
        </div>
        <button
          className="wb-action-btn wb-generate"
          onClick={handleGenerate}
          disabled={generating || loading}
        >
          {generating ? 'Generating…' : 'Generate candidates'}
        </button>
      </div>

      {error && <div className="wb-error">{error}</div>}

      {/* Stats bar */}
      {data && (
        <div className="wb-stats">
          <span className="wb-stat">
            <span className="wb-stat-n">{data.total}</span> total
          </span>
          <span className="wb-stat">
            <span className="wb-stat-n" style={{ color: '#aaa' }}>{data.pending}</span> pending
          </span>
          <span className="wb-stat">
            <span className="wb-stat-n" style={{ color: '#4ecb71' }}>{data.kept}</span> kept
          </span>
          <span className="wb-stat">
            <span className="wb-stat-n" style={{ color: '#e05555' }}>{data.discarded}</span> discarded
          </span>
        </div>
      )}

      {/* Filter row */}
      {candidates.length > 0 && (
        <div className="wb-filters">
          {['all', 'pending', 'kept', 'discarded'].map((f) => (
            <button
              key={f}
              className={`wb-filter-btn ${filter === f ? 'active' : ''}`}
              onClick={() => setFilter(f)}
            >
              {f[0].toUpperCase() + f.slice(1)}
            </button>
          ))}
        </div>
      )}

      {/* Candidate list */}
      {loading && <div className="wb-empty">Loading…</div>}
      {!loading && candidates.length === 0 && (
        <div className="wb-empty">
          No candidates yet — click <em>Generate candidates</em> to run the dry-run analysis.
        </div>
      )}
      {!loading && visible.length === 0 && candidates.length > 0 && (
        <div className="wb-empty">No candidates match this filter.</div>
      )}

      <div className="wb-list">
        {visible.map((c) => (
          <CandidateCard
            key={c.candidate_id}
            candidate={c}
            onKeep={handleKeep}
            onDiscard={handleDiscard}
            onEdit={handleEdit}
            busy={busy}
          />
        ))}
      </div>

      {/* Promote footer */}
      {keptCount > 0 && (
        <div className="wb-footer">
          <div className="wb-footer-info">
            {keptCount} candidate{keptCount !== 1 ? 's' : ''} kept
            {pendingCount > 0 ? ` · ${pendingCount} still pending` : ' · all reviewed'}
          </div>
          <button
            className="wb-action-btn wb-promote"
            onClick={handlePromote}
            disabled={promoting}
          >
            {promoting ? 'Promoting…' : `Commit ${keptCount} to memory`}
          </button>
        </div>
      )}

      {promoted && (
        <div className="wb-promoted-banner">
          Promoted {promoted.total_new_lessons || 0} lesson{promoted.total_new_lessons !== 1 ? 's' : ''} to durable memory.
        </div>
      )}

      <style>{`
        .wb-root {
          display: flex;
          flex-direction: column;
          gap: 0.75rem;
          padding: 0.25rem 0;
        }
        .wb-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          gap: 12px;
        }
        .wb-title {
          font-size: 13px;
          font-weight: 600;
          color: #e5e1d8;
        }
        .wb-subtitle {
          font-size: 11px;
          color: #888;
          margin-top: 2px;
        }
        .wb-error {
          font-size: 12px;
          color: #e05555;
          background: rgba(220,60,60,0.1);
          border-radius: 4px;
          padding: 8px 10px;
        }
        .wb-stats {
          display: flex;
          gap: 16px;
          font-size: 12px;
          color: #888;
        }
        .wb-stat-n {
          font-weight: 600;
          color: #e5e1d8;
          margin-right: 3px;
        }
        .wb-filters {
          display: flex;
          gap: 6px;
        }
        .wb-filter-btn {
          background: rgba(255,255,255,0.05);
          border: 1px solid rgba(255,255,255,0.07);
          color: #888;
          font-size: 11px;
          padding: 3px 10px;
          border-radius: 99px;
          cursor: pointer;
          transition: all 0.15s;
        }
        .wb-filter-btn:hover { color: #e5e1d8; }
        .wb-filter-btn.active {
          background: rgba(255,255,255,0.1);
          border-color: rgba(255,255,255,0.18);
          color: #e5e1d8;
        }
        .wb-list {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .wb-empty {
          font-size: 12px;
          color: #666;
          padding: 12px 0;
        }
        .wb-action-btn {
          font-size: 11px;
          padding: 4px 12px;
          border-radius: 4px;
          border: none;
          cursor: pointer;
          font-weight: 500;
          transition: opacity 0.15s;
        }
        .wb-action-btn:disabled { opacity: 0.45; cursor: default; }
        .wb-keep    { background: rgba(60,180,80,0.2);  color: #4ecb71; }
        .wb-discard { background: rgba(220,60,60,0.15); color: #e05555; }
        .wb-edit    { background: rgba(74,158,255,0.15); color: #4a9eff; }
        .wb-save    { background: rgba(74,158,255,0.2);  color: #4a9eff; }
        .wb-cancel  { background: rgba(255,255,255,0.07); color: #888; }
        .wb-generate { background: rgba(255,255,255,0.08); color: #e5e1d8; }
        .wb-promote {
          background: rgba(60,180,80,0.22);
          color: #4ecb71;
          font-size: 12px;
          padding: 6px 16px;
        }
        .wb-footer {
          display: flex;
          justify-content: space-between;
          align-items: center;
          border-top: 1px solid rgba(255,255,255,0.07);
          padding-top: 10px;
          margin-top: 4px;
        }
        .wb-footer-info { font-size: 12px; color: #888; }
        .wb-promoted-banner {
          font-size: 12px;
          color: #4ecb71;
          background: rgba(60,180,80,0.1);
          border-radius: 4px;
          padding: 8px 10px;
        }
      `}</style>
    </div>
  );
}
