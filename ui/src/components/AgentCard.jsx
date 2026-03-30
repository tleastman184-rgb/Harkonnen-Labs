import React from 'react';

const AgentCard = ({ agent, variant = 'dark', isSingleton = false }) => {
  const { id, name, status, task, latestLog, accentColor, role } = agent;

  const isSilhouette = status === 'idle' || status === 'waiting';
  const effectiveVariant = isSilhouette ? 'silhouette' : variant;

  const statusColor = {
    idle: '#5d5d66',
    waiting: '#5d5d66',
    running: 'var(--accent-gold)',
    blocked: '#e74c3c',
    complete: '#a8c69f', // Muted green matching the industrial tone
    scanned: 'var(--accent-gold)',
  }[status];

  const iconPath = `/icons/agents/${id}/icon.png`;

  return (
    <div className={`split-agent-card v-${effectiveVariant} s-${status} ${status === 'running' ? 'pulse-border' : ''} ${isSingleton ? 'singleton-layout' : ''}`}>
      {/* ── PART 1: THE TILE ──────────────────────────────────────────────── */}
      <div className="icon-tile" style={{ borderColor: accentColor + '44' }}>
        {status === 'running' && (
          <div className="tech-rings">
            <div className="ring ring-1"></div>
            <div className="ring ring-2"></div>
            <div className="h-emblem">H</div>
          </div>
        )}
        <div className="image-container">
          {status === 'running' && <div className="scanning-beam"></div>}
          <img
            src={iconPath}
            alt={name}
            className={effectiveVariant === 'silhouette' ? 'img-silhouette' : ''}
            onError={(e) => {
              e.target.parentElement.innerHTML = `<div class="silhouette-placeholder">${name[0]}</div>`;
            }}
          />
        </div>
        <div className="nameplate" style={{ backgroundColor: isSingleton ? 'var(--accent-gold)' : accentColor }}>
          <span className="agent-name" style={{ color: isSingleton ? 'var(--bg-primary)' : 'white' }}>{name}</span>
          <span className="agent-role-short" style={{ color: isSingleton ? 'var(--bg-primary)' : 'white' }}>{role ? role.split(' ')[0] : ''}</span>
          {status === 'running' && <div className="active-pulse"></div>}
        </div>
        {status === 'running' && (
          <div className="corner-brackets">
            <div className="bracket top-left"></div>
            <div className="bracket bottom-right"></div>
          </div>
        )}
      </div>

      {/* ── PART 2: THE DATA ─────────────────────────────────────────────── */}
      <div className="activity-panel">
        <div className="panel-header">
          <div className="role-full">{role}</div>
          <div className="status-indicator">
            <span className="status-dot" style={{ backgroundColor: statusColor }}></span>
            <span className="status-text">{status}</span>
          </div>
        </div>

        {effectiveVariant !== 'silhouette' && (
          <div className="panel-body">
            <div className="task-box">
              <label>ACTIVE_TASK</label>
              <div className="task-text">{task}</div>
            </div>
            {!isSingleton && (
              <div className="log-box">
                <label>LATEST_LOG</label>
                <div className="log-text mono">{latestLog}</div>
              </div>
            )}
            {isSingleton && (
              <div className="singleton-log">
                <div className="log-line">{`>> ${latestLog}`}</div>
                <div className="log-cursor">_</div>
              </div>
            )}
          </div>
        )}

        {effectiveVariant === 'silhouette' && (
          <div className="inactive-shade">
            <span>AWAITING_SIGNAL</span>
          </div>
        )}
      </div>

      <style jsx>{`
        .split-agent-card {
          display: flex;
          background: var(--bg-secondary);
          border-radius: 12px;
          overflow: hidden;
          border: 1px solid var(--border-glass);
          height: 180px;
          transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
          position: relative;
        }

        .singleton-layout {
          height: auto;
          flex-direction: column;
          border: 1px solid var(--border-strong);
        }

        .singleton-layout .icon-tile {
          width: 100%;
          height: 180px;
          border-right: none;
          border-bottom: 1px solid var(--border-glass);
        }

        .split-agent-card:hover {
          transform: translateY(-2px);
          border-color: var(--accent-gold);
          box-shadow: 0 10px 30px -10px rgba(0,0,0,0.5);
        }

        .v-light {
          background: #e5e1d8;
          color: #1b1e20;
        }

        .icon-tile {
          width: 140px;
          flex-shrink: 0;
          display: flex;
          flex-direction: column;
          background: rgba(0, 0, 0, 0.2);
          border-right: 1px solid var(--border-glass);
          position: relative;
        }

        .image-container {
          flex: 1;
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 10px;
          overflow: hidden;
          position: relative;
        }

        .image-container img {
          max-width: 90%;
          max-height: 90%;
          object-fit: contain;
          z-index: 2;
        }

        .img-silhouette {
          filter: grayscale(1) brightness(0.3) contrast(1.2);
          opacity: 0.4;
        }

        .silhouette-placeholder {
          font-size: 3rem;
          font-weight: 900;
          opacity: 0.1;
          color: white;
        }

        .nameplate {
          padding: 8px 12px;
          display: flex;
          flex-direction: column;
          justify-content: center;
          height: 52px;
          position: relative;
          z-index: 3;
        }

        .agent-name {
          font-size: 0.9rem;
          font-weight: 900;
          letter-spacing: 2px;
          text-transform: uppercase;
        }

        .agent-role-short {
          font-size: 0.65rem;
          font-weight: 800;
          opacity: 0.7;
          text-transform: uppercase;
        }

        .activity-panel {
          flex: 1;
          display: flex;
          flex-direction: column;
          padding: 1.25rem;
          gap: 1rem;
        }

        .panel-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          border-bottom: 1px solid var(--border-glass);
          padding-bottom: 0.75rem;
        }

        .role-full {
          font-size: 0.7rem;
          font-weight: 800;
          text-transform: uppercase;
          letter-spacing: 1px;
          color: var(--text-secondary);
        }

        .status-text {
          font-size: 0.7rem;
          font-weight: 900;
          text-transform: uppercase;
          letter-spacing: 1.5px;
        }

        .panel-body {
          display: flex;
          flex-direction: column;
          gap: 1rem;
        }

        label {
          font-size: 0.6rem;
          font-weight: 900;
          color: var(--accent-gold);
          opacity: 0.6;
          letter-spacing: 2px;
          margin-bottom: 0.25rem;
          display: block;
        }

        .task-text {
          font-size: 0.9rem;
          font-weight: 600;
          letter-spacing: 0.5px;
        }

        .log-box {
          background: rgba(0, 0, 0, 0.3);
          padding: 0.75rem;
          border-radius: 6px;
          border: 1px solid var(--border-glass);
        }

        .log-text {
          font-size: 0.75rem;
          color: var(--text-secondary);
        }

        .singleton-log {
          background: #000;
          padding: 1rem;
          border-radius: 4px;
          font-family: var(--font-mono);
          font-size: 0.8rem;
          min-height: 80px;
          color: var(--accent-gold);
          border-left: 2px solid var(--accent-gold);
        }

        .log-cursor {
          display: inline-block;
          animation: blink 1s infinite;
        }

        @keyframes blink {
          0%, 100% { opacity: 1; }
          50% { opacity: 0; }
        }

        .scanning-beam {
          position: absolute;
          top: -20%;
          left: 0;
          width: 100%;
          height: 15px;
          background: linear-gradient(180deg, transparent, var(--accent-gold-glow), transparent);
          z-index: 5;
          animation: scan 4s infinite linear;
        }

        @keyframes scan {
          0% { top: -20%; opacity: 0; }
          10%, 90% { opacity: 1; }
          100% { top: 120%; opacity: 0; }
        }

        .tech-rings {
          position: absolute;
          top: 50%;
          left: 50%;
          transform: translate(-50%, -50%);
          width: 130px;
          height: 130px;
          z-index: 1;
        }

        .h-emblem {
          position: absolute;
          top: 50%;
          left: 50%;
          transform: translate(-50%, -50%);
          font-size: 3rem;
          font-weight: 900;
          color: var(--accent-gold);
          opacity: 0.05;
          font-family: serif;
        }

        .ring {
          position: absolute;
          border: 1px solid var(--accent-gold-glow);
          border-radius: 50%;
          top: 0; left: 0; right: 0; bottom: 0;
        }

        .ring-1 { border-style: dashed; animation: rotate 20s infinite linear; }
        .ring-2 { border-style: dotted; animation: rotate 30s infinite linear reverse; width: 80%; height: 80%; top: 10%; left: 10%; }

        @keyframes rotate {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }

        .bracket {
          position: absolute;
          width: 12px;
          height: 12px;
          border: 2px solid var(--accent-gold);
          opacity: 0.4;
        }

        .top-left { top: -2px; left: -2px; border-right: 0; border-bottom: 0; }
        .bottom-right { bottom: -2px; right: -2px; border-left: 0; border-top: 0; }

        .pulse-border {
          border-color: var(--accent-gold);
          box-shadow: 0 0 20px var(--accent-gold-glow);
        }

        .active-pulse {
          position: absolute;
          right: 12px;
          top: 50%;
          transform: translateY(-50%);
          width: 8px;
          height: 8px;
          background: var(--accent-gold);
          border-radius: 50%;
          box-shadow: 0 0 10px var(--accent-gold);
          animation: pulse-active 2s infinite;
        }

        @keyframes pulse-active {
          0% { transform: translateY(-50%) scale(1); opacity: 1; }
          100% { transform: translateY(-50%) scale(2.5); opacity: 0; }
        }

        .inactive-shade {
          flex: 1;
          display: flex;
          align-items: center;
          justify-content: center;
          opacity: 0.2;
        }

        .inactive-shade span {
          font-size: 0.75rem;
          font-weight: 700;
          letter-spacing: 2px;
        }

        .mono {
          font-family: var(--font-mono);
        }
      `}</style>
    </div>
  );
};

export default AgentCard;
