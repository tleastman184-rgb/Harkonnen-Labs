import React, { useState, useEffect } from 'react';
import AgentCard from './components/AgentCard';

const API_BASE = 'http://localhost:3000/api';

const initialAgents = [
  { id: 'scout', name: 'Scout', role: 'Spec Retriever', status: 'idle', task: 'Awaiting spec...', latestLog: 'Ready.', accentColor: '#c4922a' },
  { id: 'coobie', name: 'Coobie', role: 'Memory Retriever', status: 'idle', task: 'Awaiting memory query', latestLog: 'Ready.', accentColor: '#7a2a3a' },
  { id: 'keeper', name: 'Keeper', role: 'Boundary Retriever', status: 'idle', task: 'Monitoring perimeter', latestLog: 'Ready.', accentColor: '#8a7a3a' },
  { id: 'mason', name: 'Mason', role: 'Build Retriever', status: 'idle', task: 'Awaiting design', latestLog: 'Ready.', accentColor: '#c4662a' },
  { id: 'piper', name: 'Piper', role: 'Tool Retriever', status: 'idle', task: 'Awaiting signal', latestLog: 'Ready.', accentColor: '#5a7a5a' },
  { id: 'ash', name: 'Ash', role: 'Twin Retriever', status: 'idle', task: 'Awaiting signal', latestLog: 'Ready.', accentColor: '#2a7a7a' },
  { id: 'bramble', name: 'Bramble', role: 'Test Retriever', status: 'idle', task: 'Awaiting signal', latestLog: 'Ready.', accentColor: '#a89a2a' },
  { id: 'sable', name: 'Sable', role: 'Scenario Retriever', status: 'idle', task: 'Awaiting signal', latestLog: 'Ready.', accentColor: '#3a4a5a' },
  { id: 'flint', name: 'Flint', role: 'Artifact Retriever', status: 'idle', task: 'Awaiting signal', latestLog: 'Ready.', accentColor: '#8a6a3a' },
];

function App() {
  const [run, setRun] = useState(null);
  const [events, setEvents] = useState([]);
  const [agents, setAgents] = useState(initialAgents);

  // 1. Fetch latest run on mount
  useEffect(() => {
    const fetchLatest = async () => {
      try {
        const res = await fetch(`${API_BASE}/runs`);
        const data = await res.json();
        if (data && data.length > 0) {
          setRun(data[0]);
        }
      } catch (e) {
        console.error("Failed to fetch runs", e);
      }
    };
    fetchLatest();
    const interval = setInterval(fetchLatest, 5000);
    return () => clearInterval(interval);
  }, []);

  // 2. Poll events for current run
  useEffect(() => {
    if (!run) return;
    const fetchEvents = async () => {
      try {
        const res = await fetch(`${API_BASE}/runs/${run.run_id}/events`);
        const data = await res.json();
        setEvents(data);
      } catch (e) {
        console.error("Failed to fetch events", e);
      }
    };
    fetchEvents();
    const interval = setInterval(fetchEvents, 1000);
    return () => clearInterval(interval);
  }, [run]);

  // 3. Map events to agent state
  useEffect(() => {
    if (events.length === 0) return;

    setAgents(prev => prev.map(agent => {
      const agentEvents = events.filter(e => e.agent.toLowerCase() === agent.id.toLowerCase());
      if (agentEvents.length === 0) return agent;

      const latest = agentEvents[agentEvents.length - 1];
      return {
        ...agent,
        status: latest.status.toLowerCase(),
        task: latest.phase || agent.task,
        latestLog: latest.message
      };
    }));
  }, [events]);

  const memoryAgent = agents.find(a => a.id === 'coobie');
  const planningAgents = agents.filter(a => ['scout', 'keeper'].includes(a.id));
  const actionAgents = agents.filter(a => ['mason', 'piper', 'ash'].includes(a.id) || (!['scout', 'coobie', 'keeper', 'bramble', 'sable', 'flint'].includes(a.id) && !a.id.startsWith('verifier')));
  const verificationAgents = agents.filter(a => ['bramble', 'sable', 'flint'].includes(a.id));

  return (
    <div className="app-container">
      <header className="run-header glass">
        <div className="header-brand">
          <div className="logo-container">
            <div className="css-logo">H</div>
            <img
              src="/images/logo.png"
              alt=""
              className="brand-logo"
              onError={(e) => e.target.style.display = 'none'}
            />
          </div>
          <div className="header-info">
            <h1>HARKONNEN LABS / THE PACK</h1>
            <div className="run-meta">
              <span className="run-id">RUN: {run?.run_id?.split('-')[0] || 'OFFLINE'}</span>
              <span className="spec-title">SPEC: {run?.product || 'NO ACTIVE SPEC'}</span>
            </div>
          </div>
        </div>
        <div className="run-status">
          <span className={`status-badge ${run?.status === 'active' ? 'running' : ''}`}>
            {run?.status?.toUpperCase() || 'IDLE'}
          </span>
        </div>
        <div className="header-actions">
          <button className="btn-secondary">VIEW_LOGS</button>
          <button className="btn-primary">TERMINATE</button>
        </div>
      </header>

      <main className="dashboard-content">
        <div className="grid-layout">
          <div className="main-phases">
            <section className="phase-section">
              <div className="section-header">
                <h2>01_INTAKE & PLANNING</h2>
                <div className="section-line"></div>
              </div>
              <div className="agent-grid">
                {planningAgents.map((agent) => (
                  <AgentCard key={agent.id} agent={agent} variant="dark" />
                ))}
              </div>
            </section>

            <section className="phase-section">
              <div className="section-header">
                <h2>02_IMPLEMENTATION & ACTION</h2>
                <div className="section-line"></div>
              </div>
              <div className="agent-grid dynamic-grid">
                {actionAgents.map((agent) => (
                  <AgentCard key={agent.id} agent={agent} variant={agent.id === 'mason' ? 'light' : 'dark'} />
                ))}
              </div>
            </section>

            <section className="phase-section">
              <div className="section-header">
                <h2>03_VERIFICATION & BUNDLING</h2>
                <div className="section-line"></div>
              </div>
              <div className="agent-grid">
                {verificationAgents.map((agent) => (
                  <AgentCard key={agent.id} agent={agent} variant="dark" />
                ))}
              </div>
            </section>
          </div>

          <aside className="sidebar-coobie">
            <div className="section-header">
              <h2>MEMORY_VAULT</h2>
              <div className="section-line"></div>
            </div>
            {memoryAgent && (
              <div className="coobie-singleton">
                <AgentCard agent={memoryAgent} variant="dark" isSingleton />
              </div>
            )}
            <div className="vault-stats glass">
              <div className="stat-row">
                <span className="stat-label">PATTERNS_CACHED:</span>
                <span className="stat-value">1,248</span>
              </div>
              <div className="stat-row">
                <span className="stat-label">CONTEXT_DEPTH:</span>
                <span className="stat-value">84%</span>
              </div>
            </div>
          </aside>
        </div>
      </main>

      <footer className="dashboard-footer glass">
        <div className="stat-group">
          <span className="stat-label">RESOURCES:</span>
          <span className="stat-value">CPU: 38% / MEM: 0.9GB</span>
        </div>
        <div className="stat-group">
          <span className="stat-label">ACTIVE_THREADS:</span>
          <span className="stat-value">
            {agents.filter(a => a.status === 'running').map(a => a.name.toUpperCase()).join(', ') || 'NONE'}
          </span>
        </div>
        <div className="stat-group ml-auto">
          <span className="stat-label">SYSTEM_UPTIME:</span>
          <span className="stat-value">04:12:33</span>
        </div>
      </footer>

      <style jsx>{`
        .app-container {
          min-height: 100vh;
          background: var(--bg-primary);
          color: var(--text-primary);
          padding: 2rem;
          display: flex;
          flex-direction: column;
          gap: 2rem;
          max-width: 1800px;
          margin: 0 auto;
        }

        .run-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 1rem 2rem;
          border-radius: 12px;
          border: 1px solid var(--border-glass);
        }

        .header-brand {
          display: flex;
          align-items: center;
          gap: 1.5rem;
        }

        .logo-container {
          position: relative;
          width: 50px;
          height: 50px;
          display: flex;
          align-items: center;
          justify-content: center;
        }

        .css-logo {
          position: absolute;
          font-family: serif;
          font-size: 2.5rem;
          font-weight: 900;
          color: var(--accent-gold);
          text-shadow: 0 0 15px var(--accent-gold-glow);
          z-index: 1;
        }

        .brand-logo {
          height: 100%;
          position: relative;
          z-index: 2;
          filter: drop-shadow(0 0 10px var(--accent-gold-glow));
        }

        .header-info h1 {
          font-size: 1.25rem;
          letter-spacing: 4px;
          color: var(--accent-gold);
          margin: 0;
          font-weight: 900;
        }

        .run-meta {
          display: flex;
          gap: 1.5rem;
          font-family: var(--font-mono);
          font-size: 0.75rem;
          color: var(--text-secondary);
          margin-top: 4px;
        }

        .status-badge {
          background: rgba(194, 163, 114, 0.1);
          color: var(--accent-gold);
          padding: 6px 16px;
          border: 1px solid var(--border-strong);
          border-radius: 4px;
          font-size: 0.8rem;
          font-weight: 800;
          letter-spacing: 2px;
        }

        .grid-layout {
          display: grid;
          grid-template-columns: 1fr 400px;
          gap: 2.5rem;
        }

        .main-phases {
          display: flex;
          flex-direction: column;
          gap: 3rem;
        }

        .phase-section {
          display: flex;
          flex-direction: column;
          gap: 1.5rem;
        }

        .section-header {
          display: flex;
          align-items: center;
          gap: 1.5rem;
        }

        .section-header h2 {
          font-size: 0.75rem;
          letter-spacing: 3px;
          color: var(--accent-gold);
          white-space: nowrap;
          margin: 0;
          opacity: 0.8;
        }

        .section-line {
          height: 1px;
          background: linear-gradient(90deg, var(--border-strong) 0%, transparent 100%);
          flex: 1;
        }

        .agent-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(400px, 1fr));
          gap: 1.5rem;
        }

        .dynamic-grid {
          grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
        }

        .sidebar-coobie {
          display: flex;
          flex-direction: column;
          gap: 1.5rem;
        }

        .coobie-singleton {
          transform: scale(1);
          transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
        }

        .vault-stats {
          padding: 1.5rem;
          border-radius: 12px;
          display: flex;
          flex-direction: column;
          gap: 1rem;
          border: 1px solid var(--border-glass);
        }

        .stat-row {
          display: flex;
          justify-content: space-between;
          font-size: 0.75rem;
          font-family: var(--font-mono);
        }

        .stat-label {
          color: var(--text-secondary);
        }

        .stat-value {
          color: var(--accent-gold);
        }

        .dashboard-footer {
          margin-top: auto;
          display: flex;
          gap: 4rem;
          padding: 1rem 2rem;
          border-radius: 12px;
          align-items: center;
          border: 1px solid var(--border-glass);
        }

        .btn-primary {
          background: var(--accent-gold);
          border: none;
          color: var(--bg-primary);
          font-weight: 900;
        }

        .btn-secondary {
          background: transparent;
          border: 1px solid var(--border-strong);
          color: var(--accent-gold);
        }

        @media (max-width: 1400px) {
          .grid-layout {
            grid-template-columns: 1fr;
          }
          .sidebar-coobie {
            order: -1;
          }
        }
      `}</style>
    </div>
  );
}

export default App;
