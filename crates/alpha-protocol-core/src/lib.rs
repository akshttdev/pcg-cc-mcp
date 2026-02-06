//! Alpha Protocol Core - Sovereign Mesh Networking
//!
//! This crate provides the networking foundation for the Vibertas OS ecosystem,
//! enabling secure peer-to-peer communication across:
//! - Desktop (PCG Dashboard via Tauri)
//! - Mobile (iOS/Android via Tauri)
//! - IoT (Omega devices)
//! - Wearables (future)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    ALPHA PROTOCOL CORE                       │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Identity    │  Ed25519 keypairs, BIP39 mnemonics           │
//! │  Encryption  │  X25519 key exchange, ChaCha20-Poly1305      │
//! │  Transport   │  libp2p (gossipsub, kad, mdns, noise)        │
//! │  Relay       │  NATS for NAT traversal fallback             │
//! │  Wire Format │  HANDSHAKE, DATA, ACK, PEER_ANNOUNCE, etc.   │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod identity;
pub mod crypto;
pub mod wire;
pub mod mesh;
pub mod relay;
pub mod node;
pub mod economics;
pub mod mining;
pub mod resources;
pub mod reward_tracker;
pub mod reward_distributor;

// Re-exports
pub use identity::{NodeIdentity, WalletInfo};
pub use crypto::{encrypt, decrypt, SessionKey};
pub use wire::{Message, MessageType, NodeResources};
pub use mesh::{MeshNode, PeerInfo, MeshMessage};
pub use node::{AlphaNode, NodeConfig};
pub use resources::collect_resources;
pub use economics::{
    ResourceContribution, ResourceTracker, RewardRates,
    NodeReputation, StakePool, ContributionProof,
    calculate_rewards, vibe_to_display, display_to_vibe,
    VibeAmount,
};
pub use reward_tracker::{RewardTracker as PeerRewardTracker, RewardTrackerStats, RewardTrackerConfig};
pub use reward_distributor::{RewardDistributor, DistributorConfig, DistributorStats};

/// Protocol version
pub const PROTOCOL_VERSION: &str = "alpha/1.0.0";

/// Default NATS relay server
pub const DEFAULT_NATS_RELAY: &str = "nats://nonlocal.info:4222";

/// Default libp2p port
pub const DEFAULT_P2P_PORT: u16 = 4001;

/// Initialize the Alpha Protocol core
pub fn init() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("alpha_protocol_core=debug".parse()?)
        )
        .init();

    tracing::info!("Alpha Protocol Core v{} initialized", env!("CARGO_PKG_VERSION"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        // Init should succeed (may fail if already initialized)
        let _ = init();
    }

    #[test]
    fn test_protocol_version() {
        assert_eq!(PROTOCOL_VERSION, "alpha/1.0.0");
    }
}
