//! APN Node - Alpha Protocol Network CLI
//!
//! A simple CLI tool to test mesh networking between devices.
//!
//! Usage:
//!   apn_node [OPTIONS]
//!
//! Options:
//!   --port <PORT>       P2P port (default: 4001)
//!   --relay <URL>       NATS relay URL (default: nats://nonlocal.info:4222)
//!   --bootstrap <ADDR>  Bootstrap peer multiaddr
//!   --new               Generate new identity
//!   --import <PHRASE>   Import from mnemonic phrase

use alpha_protocol_core::{
    node::{AlphaNodeBuilder, NodeEvent},
    identity::NodeIdentity,
    DEFAULT_NATS_RELAY,
};
use tokio::sync::mpsc;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,alpha_protocol_core=debug")
        .init();

    let args: Vec<String> = env::args().collect();

    let mut port: u16 = 4001;
    let mut relay_url = DEFAULT_NATS_RELAY.to_string();
    let mut bootstrap_peers: Vec<String> = vec![];
    let mut mnemonic: Option<String> = None;
    let mut heartbeat_interval: u64 = 30; // seconds
    let mut enable_heartbeat = true;

    // Parse args
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                port = args.get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("--port requires a value"))?
                    .parse()?;
                i += 2;
            }
            "--relay" => {
                relay_url = args.get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("--relay requires a value"))?
                    .clone();
                i += 2;
            }
            "--bootstrap" => {
                let peer = args.get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("--bootstrap requires a value"))?
                    .clone();
                bootstrap_peers.push(peer);
                i += 2;
            }
            "--import" => {
                mnemonic = Some(args.get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("--import requires a mnemonic phrase"))?
                    .clone());
                i += 2;
            }
            "--new" => {
                mnemonic = None;
                i += 1;
            }
            "--heartbeat-interval" => {
                heartbeat_interval = args.get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("--heartbeat-interval requires a value"))?
                    .parse()?;
                i += 2;
            }
            "--no-heartbeat" => {
                enable_heartbeat = false;
                i += 1;
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_help();
                return Ok(());
            }
        }
    }

    // Create event channel
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<NodeEvent>();

    // Build the node
    let mut builder = AlphaNodeBuilder::new()
        .with_port(port)
        .with_relay(&relay_url)
        .with_bootstrap_peers(bootstrap_peers)
        .with_capabilities(vec![
            "compute".to_string(),
            "relay".to_string(),
            "storage".to_string(),
        ]);

    if let Some(phrase) = mnemonic {
        builder = builder.with_mnemonic(&phrase);
    }

    let mut node = builder.build(event_tx)?;

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║              ALPHA PROTOCOL NETWORK - Node                       ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  Node ID:  {}                                   ║", node.short_id());
    println!("║  Address:  {}  ║", node.address());
    println!("║  P2P Port: {}                                                 ║", port);
    println!("║  Relay:    {}                         ║", relay_url);
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Print mnemonic for backup
    let identity = NodeIdentity::from_mnemonic_phrase(&node.identity().mnemonic_phrase())?;
    println!("📝 Mnemonic (save this for recovery):");
    println!("   {}\n", identity.mnemonic_phrase());

    // Start the node
    println!("🚀 Starting node...");
    node.start().await?;
    println!("✅ Node started!\n");

    // Announce to the network (may fail if no peers yet, that's OK)
    println!("📢 Announcing to network...");
    match node.announce().await {
        Ok(_) => println!("✅ Announced!\n"),
        Err(e) => println!("⚠️  Announce pending (no peers yet): {}\n", e),
    }

    // Setup heartbeat interval
    let mut heartbeat_timer = if enable_heartbeat {
        println!("💓 Heartbeat enabled (interval: {}s)\n", heartbeat_interval);
        Some(tokio::time::interval(std::time::Duration::from_secs(heartbeat_interval)))
    } else {
        None
    };

    println!("👂 Listening for events... (Ctrl+C to quit)\n");

    // Event loop
    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                match event {
                    NodeEvent::Started { peer_id, address } => {
                        println!("🟢 Node started: {} @ {}", peer_id, address);
                    }
                    NodeEvent::PeerConnected(peer) => {
                        println!("🔗 Peer connected: {} ({} capabilities)",
                            peer.peer_id,
                            peer.capabilities.len()
                        );
                        if let Some(addr) = peer.wallet_address {
                            println!("   Wallet: {}", addr);
                        }
                        for cap in &peer.capabilities {
                            println!("   - {}", cap);
                        }
                    }
                    NodeEvent::PeerDisconnected(peer_id) => {
                        println!("🔴 Peer disconnected: {}", peer_id);
                    }
                    NodeEvent::MessageReceived { from, message } => {
                        println!("📨 Message from {}: {:?}", from, message);
                    }
                    NodeEvent::RelayConnected => {
                        println!("🌐 Relay connected");
                    }
                    NodeEvent::RelayDisconnected => {
                        println!("⚠️  Relay disconnected");
                    }
                    NodeEvent::Error(e) => {
                        eprintln!("❌ Error: {}", e);
                    }
                }
            }
            Some(_) = async {
                if let Some(ref mut timer) = heartbeat_timer {
                    Some(timer.tick().await)
                } else {
                    None
                }
            } => {
                // Send heartbeat
                if let Err(e) = node.send_heartbeat().await {
                    tracing::debug!("Heartbeat failed: {}", e);
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\n👋 Shutting down...");
                break;
            }
        }
    }

    Ok(())
}

fn print_help() {
    println!("APN Node - Alpha Protocol Network CLI\n");
    println!("Usage: apn_node [OPTIONS]\n");
    println!("Options:");
    println!("  --port <PORT>       P2P port (default: 4001)");
    println!("  --relay <URL>       NATS relay URL (default: nats://nonlocal.info:4222)");
    println!("  --bootstrap <ADDR>  Bootstrap peer multiaddr (can be used multiple times)");
    println!("  --new               Generate new identity (default)");
    println!("  --import <PHRASE>   Import from mnemonic phrase (quoted)");
    println!("  --heartbeat-interval <SECS>  Heartbeat interval in seconds (default: 30)");
    println!("  --no-heartbeat      Disable heartbeat broadcasts");
    println!("  -h, --help          Show this help\n");
    println!("Examples:");
    println!("  # Start with new identity on default port");
    println!("  apn_node\n");
    println!("  # Start on custom port");
    println!("  apn_node --port 4002\n");
    println!("  # Import existing identity");
    println!("  apn_node --import \"word1 word2 word3 ...\"");
}
