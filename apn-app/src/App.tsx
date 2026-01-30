import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface NodeStatus {
  node_id: string | null;
  wallet_address: string | null;
  peer_id: string | null;
  is_running: boolean;
  peers_connected: number;
  vibe_balance: number;
  resources: {
    cpu_hours: number;
    bandwidth_gb: number;
    storage_gb: number;
    tasks_completed: number;
    uptime_hours: number;
  };
  system: {
    cpu_usage: number;
    memory_used_mb: number;
    memory_total_mb: number;
    cpu_cores: number;
  };
}

function App() {
  const [status, setStatus] = useState<NodeStatus | null>(null);
  const [mnemonic, setMnemonic] = useState<string>('');
  const [savedMnemonic, setSavedMnemonic] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>('');
  const [logs, setLogs] = useState<string[]>([]);

  const addLog = (msg: string) => {
    setLogs(prev => [...prev.slice(-50), `[${new Date().toLocaleTimeString()}] ${msg}`]);
  };

  useEffect(() => {
    // Poll status
    const interval = setInterval(async () => {
      try {
        const s = await invoke<NodeStatus>('get_status');
        setStatus(s);
      } catch (e) {
        console.error('Failed to get status:', e);
      }
    }, 2000);

    // Listen for events
    const unlistenStarted = listen('node-started', (e) => {
      addLog(`Node started: ${e.payload}`);
    });
    const unlistenPeerConnected = listen('peer-connected', (e) => {
      addLog(`Peer connected: ${e.payload}`);
    });
    const unlistenPeerDisconnected = listen('peer-disconnected', (e) => {
      addLog(`Peer disconnected: ${e.payload}`);
    });
    const unlistenMessage = listen('message-received', (e: any) => {
      addLog(`Message from ${e.payload.from}: ${e.payload.message}`);
    });
    const unlistenRelay = listen('relay-connected', () => {
      addLog('Relay connected');
    });
    const unlistenError = listen('node-error', (e) => {
      addLog(`Error: ${e.payload}`);
    });

    return () => {
      clearInterval(interval);
      unlistenStarted.then(f => f());
      unlistenPeerConnected.then(f => f());
      unlistenPeerDisconnected.then(f => f());
      unlistenMessage.then(f => f());
      unlistenRelay.then(f => f());
      unlistenError.then(f => f());
    };
  }, []);

  const startNode = async () => {
    setLoading(true);
    setError('');
    try {
      const result = await invoke<string>('start_node', {
        port: 4001,
        mnemonic: mnemonic || null,
      });
      const data = JSON.parse(result);
      setSavedMnemonic(data.mnemonic);
      addLog(`Node started: ${data.node_id}`);
    } catch (e: any) {
      setError(e.toString());
      addLog(`Failed to start: ${e}`);
    }
    setLoading(false);
  };

  const stopNode = async () => {
    setLoading(true);
    try {
      await invoke('stop_node');
      addLog('Node stopped');
    } catch (e: any) {
      setError(e.toString());
    }
    setLoading(false);
  };

  return (
    <div className="app">
      <header className="header">
        <div className="logo">
          <span className="logo-icon">&#9670;</span>
          <h1>Alpha Protocol Network</h1>
        </div>
        <div className={`status-badge ${status?.is_running ? 'online' : 'offline'}`}>
          {status?.is_running ? 'ONLINE' : 'OFFLINE'}
        </div>
      </header>

      <main className="main">
        {/* Node Info Card */}
        <section className="card">
          <h2>Node Identity</h2>
          {status?.node_id ? (
            <div className="info-grid">
              <div className="info-item">
                <label>Node ID</label>
                <code>{status.node_id}</code>
              </div>
              <div className="info-item">
                <label>Wallet Address</label>
                <code className="small">{status.wallet_address}</code>
              </div>
              <div className="info-item">
                <label>Peer ID</label>
                <code className="small">{status.peer_id || 'Not connected'}</code>
              </div>
            </div>
          ) : (
            <p className="muted">Start the node to generate identity</p>
          )}
        </section>

        {/* Vibe Balance Card */}
        <section className="card highlight">
          <h2>Vibe Balance</h2>
          <div className="balance">
            <span className="amount">{status?.vibe_balance?.toFixed(2) || '0.00'}</span>
            <span className="unit">VIBE</span>
          </div>
          <div className="stats-row">
            <div className="stat">
              <span className="value">{status?.peers_connected || 0}</span>
              <span className="label">Peers</span>
            </div>
            <div className="stat">
              <span className="value">{status?.resources.tasks_completed || 0}</span>
              <span className="label">Tasks</span>
            </div>
            <div className="stat">
              <span className="value">{status?.resources.uptime_hours?.toFixed(1) || '0.0'}h</span>
              <span className="label">Uptime</span>
            </div>
          </div>
        </section>

        {/* System Resources */}
        <section className="card">
          <h2>System Resources</h2>
          <div className="resource-bars">
            <div className="resource">
              <div className="resource-header">
                <span>CPU</span>
                <span>{status?.system.cpu_usage?.toFixed(1) || 0}%</span>
              </div>
              <div className="bar">
                <div className="bar-fill" style={{ width: `${status?.system.cpu_usage || 0}%` }} />
              </div>
            </div>
            <div className="resource">
              <div className="resource-header">
                <span>Memory</span>
                <span>{status?.system.memory_used_mb || 0} / {status?.system.memory_total_mb || 0} MB</span>
              </div>
              <div className="bar">
                <div
                  className="bar-fill"
                  style={{
                    width: `${((status?.system.memory_used_mb || 0) / (status?.system.memory_total_mb || 1)) * 100}%`
                  }}
                />
              </div>
            </div>
          </div>
          <p className="muted">{status?.system.cpu_cores || 0} CPU cores available</p>
        </section>

        {/* Controls */}
        <section className="card">
          <h2>Node Control</h2>
          {!status?.is_running && (
            <div className="input-group">
              <input
                type="text"
                placeholder="Enter mnemonic (optional, leave blank to generate new)"
                value={mnemonic}
                onChange={(e) => setMnemonic(e.target.value)}
              />
            </div>
          )}
          <div className="button-group">
            {status?.is_running ? (
              <button className="btn danger" onClick={stopNode} disabled={loading}>
                {loading ? 'Stopping...' : 'Stop Node'}
              </button>
            ) : (
              <button className="btn primary" onClick={startNode} disabled={loading}>
                {loading ? 'Starting...' : 'Start Node'}
              </button>
            )}
          </div>
          {error && <p className="error">{error}</p>}
          {savedMnemonic && !status?.is_running && (
            <div className="mnemonic-display">
              <label>Save your recovery phrase:</label>
              <code>{savedMnemonic}</code>
            </div>
          )}
        </section>

        {/* Event Log */}
        <section className="card">
          <h2>Event Log</h2>
          <div className="log-container">
            {logs.length === 0 ? (
              <p className="muted">No events yet</p>
            ) : (
              logs.map((log, i) => (
                <div key={i} className="log-entry">{log}</div>
              ))
            )}
          </div>
        </section>
      </main>

      <footer className="footer">
        <p>Alpha Protocol Network v0.1.0 | Relay: nonlocal.info:4222</p>
      </footer>
    </div>
  );
}

export default App;
