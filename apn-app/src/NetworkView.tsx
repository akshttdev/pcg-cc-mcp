import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface NodeResources {
  cpu_cores: number;
  ram_mb: number;
  storage_gb: number;
  gpu_available: boolean;
  gpu_model?: string;
}

interface NetworkPeer {
  node_id: string;
  wallet_address: string;
  capabilities: string[];
  resources?: NodeResources;
}

export function NetworkView() {
  const [peers, setPeers] = useState<NetworkPeer[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>('');

  useEffect(() => {
    const fetchPeers = async () => {
      try {
        // Try fetching from master node API first (for APN Core clients)
        const masterUrl = 'http://192.168.1.77:8080/api/status';

        try {
          const response = await fetch(masterUrl);
          if (response.ok) {
            const data = await response.json();
            setPeers(data.peers || []);
            setError('');
            setLoading(false);
            return;
          }
        } catch (fetchErr) {
          // If fetch fails, fall back to local Tauri command
          console.log('Falling back to local node data');
        }

        // Fallback: use local Tauri command (if running on master node)
        const networkPeers = await invoke<NetworkPeer[]>('get_network_peers');
        setPeers(networkPeers);
        setError('');
      } catch (e: any) {
        setError('Cannot connect to Pythia Master Node. Make sure the API server is running at http://192.168.1.77:8080');
      } finally {
        setLoading(false);
      }
    };

    fetchPeers();
    const interval = setInterval(fetchPeers, 5000); // Refresh every 5 seconds
    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <section className="card">
        <h2>Network Nodes</h2>
        <p className="muted">Loading network information...</p>
      </section>
    );
  }

  if (error) {
    return (
      <section className="card">
        <h2>Network Nodes</h2>
        <p className="error">Failed to load network: {error}</p>
        <p className="muted">Make sure the master node is running on this machine</p>
      </section>
    );
  }

  return (
    <section className="card">
      <h2>Alpha Protocol Network - Pythia Orchestrated</h2>
      <p className="muted" style={{ marginBottom: '1rem' }}>
        Connected Nodes: {peers.length} | Master: Pythia (192.168.1.77)
      </p>
      {peers.length === 0 ? (
        <p className="muted">No peers connected to Pythia yet. Waiting for APN Core clients to join...</p>
      ) : (
        <div className="peers-list">
          {peers.map((peer) => (
            <div key={peer.node_id} className="peer-card">
              <div className="peer-header">
                <div className="peer-status online" />
                <div>
                  <h3>{peer.node_id}</h3>
                  <code className="small">{peer.wallet_address.slice(0, 20)}...</code>
                </div>
              </div>

              {peer.resources && (
                <div className="peer-resources">
                  <div className="resource-item">
                    <span className="label">CPU:</span>
                    <span className="value">{peer.resources.cpu_cores} cores</span>
                  </div>
                  <div className="resource-item">
                    <span className="label">RAM:</span>
                    <span className="value">{(peer.resources.ram_mb / 1024).toFixed(1)} GB</span>
                  </div>
                  <div className="resource-item">
                    <span className="label">Storage:</span>
                    <span className="value">{peer.resources.storage_gb} GB</span>
                  </div>
                  {peer.resources.gpu_available && peer.resources.gpu_model && (
                    <div className="resource-item">
                      <span className="label">GPU:</span>
                      <span className="value">{peer.resources.gpu_model}</span>
                    </div>
                  )}
                </div>
              )}

              <div className="peer-capabilities">
                {peer.capabilities.map((cap) => (
                  <span key={cap} className="capability-badge">
                    {cap}
                  </span>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
