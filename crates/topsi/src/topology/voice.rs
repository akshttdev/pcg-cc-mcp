//! Voice topology integration - manages voice session nodes in the topology graph
//!
//! This module provides functions to represent voice sessions as nodes in the
//! Topsi topology, enabling:
//! - Route planning from voice input to workflow execution
//! - Visualization of active voice channels
//! - Clustering of voice sessions by channel type

use super::graph::{GraphEdge, GraphNode, TopologyGraph, ClusterInfo};
use chrono::Utc;
use uuid::Uuid;

/// Node types for voice topology
pub mod node_types {
    /// A voice gateway node - the central orchestrator
    pub const VOICE_GATEWAY: &str = "voice_gateway";
    /// A voice channel (glasses, browser, twilio, etc.)
    pub const VOICE_CHANNEL: &str = "voice_channel";
    /// An active voice session
    pub const VOICE_SESSION: &str = "voice_session";
    /// The STT processor node
    pub const STT_PROCESSOR: &str = "stt_processor";
    /// The TTS processor node
    pub const TTS_PROCESSOR: &str = "tts_processor";
    /// A voice command node (parsed intent)
    pub const VOICE_COMMAND: &str = "voice_command";
}

/// Edge types for voice topology
pub mod edge_types {
    /// Session belongs to a channel
    pub const SESSION_OF: &str = "session_of";
    /// Audio flows through this edge
    pub const AUDIO_FLOW: &str = "audio_flow";
    /// Transcription result flows through this edge
    pub const TRANSCRIPTION_FLOW: &str = "transcription_flow";
    /// Command triggers execution
    pub const TRIGGERS: &str = "triggers";
    /// Response flows back
    pub const RESPONSE_FLOW: &str = "response_flow";
    /// Gateway manages channel
    pub const MANAGES: &str = "manages";
}

/// Capabilities for voice nodes
pub mod capabilities {
    /// Can process audio input
    pub const AUDIO_INPUT: &str = "audio_input";
    /// Can produce audio output
    pub const AUDIO_OUTPUT: &str = "audio_output";
    /// Supports push-to-talk
    pub const PTT: &str = "push_to_talk";
    /// Supports wake word detection
    pub const WAKE_WORD: &str = "wake_word";
    /// Local processing (no cloud)
    pub const LOCAL: &str = "local";
    /// Streaming support
    pub const STREAMING: &str = "streaming";
}

/// Voice topology manager for adding/removing voice-related nodes
pub struct VoiceTopology;

impl VoiceTopology {
    /// Create the voice gateway node
    pub fn create_gateway_node(gateway_id: &str) -> GraphNode {
        GraphNode::new(
            Uuid::new_v4(),
            node_types::VOICE_GATEWAY,
            gateway_id,
        )
        .with_capabilities(vec![
            capabilities::AUDIO_INPUT.to_string(),
            capabilities::AUDIO_OUTPUT.to_string(),
            capabilities::STREAMING.to_string(),
        ])
        .with_status("active")
    }

    /// Create a voice channel node
    pub fn create_channel_node(
        channel_id: &str,
        channel_type: &str,
        caps: Vec<&str>,
    ) -> GraphNode {
        GraphNode::new(
            Uuid::new_v4(),
            node_types::VOICE_CHANNEL,
            channel_id,
        )
        .with_capabilities(caps.into_iter().map(String::from).collect())
        .with_status("ready")
    }

    /// Create a voice session node
    pub fn create_session_node(
        session_id: &str,
        channel_type: &str,
        device_id: Option<&str>,
    ) -> GraphNode {
        let mut metadata = serde_json::json!({
            "channel_type": channel_type,
            "started_at": Utc::now().to_rfc3339(),
        });

        if let Some(device) = device_id {
            metadata["device_id"] = serde_json::Value::String(device.to_string());
        }

        let mut node = GraphNode::new(
            Uuid::new_v4(),
            node_types::VOICE_SESSION,
            session_id,
        )
        .with_capabilities(vec![
            capabilities::AUDIO_INPUT.to_string(),
        ])
        .with_status("active");

        node.metadata = Some(metadata);
        node
    }

    /// Create STT processor node
    pub fn create_stt_node(provider: &str) -> GraphNode {
        let is_local = provider == "whisper" || provider == "local";
        let mut caps = vec![
            capabilities::AUDIO_INPUT.to_string(),
            capabilities::STREAMING.to_string(),
        ];
        if is_local {
            caps.push(capabilities::LOCAL.to_string());
        }

        GraphNode::new(
            Uuid::new_v4(),
            node_types::STT_PROCESSOR,
            format!("stt-{}", provider),
        )
        .with_capabilities(caps)
        .with_status("ready")
    }

    /// Create TTS processor node
    pub fn create_tts_node(provider: &str) -> GraphNode {
        let is_local = provider == "chatterbox" || provider == "local";
        let mut caps = vec![
            capabilities::AUDIO_OUTPUT.to_string(),
        ];
        if is_local {
            caps.push(capabilities::LOCAL.to_string());
        }

        GraphNode::new(
            Uuid::new_v4(),
            node_types::TTS_PROCESSOR,
            format!("tts-{}", provider),
        )
        .with_capabilities(caps)
        .with_status("ready")
    }

    /// Create edge from session to channel
    pub fn create_session_channel_edge(
        session_node_id: Uuid,
        channel_node_id: Uuid,
    ) -> GraphEdge {
        GraphEdge::new(
            Uuid::new_v4(),
            session_node_id,
            channel_node_id,
            edge_types::SESSION_OF,
        )
    }

    /// Create edge from channel to gateway
    pub fn create_channel_gateway_edge(
        channel_node_id: Uuid,
        gateway_node_id: Uuid,
    ) -> GraphEdge {
        GraphEdge::new(
            Uuid::new_v4(),
            gateway_node_id,
            channel_node_id,
            edge_types::MANAGES,
        )
    }

    /// Create audio flow edge
    pub fn create_audio_flow_edge(
        from_node_id: Uuid,
        to_node_id: Uuid,
    ) -> GraphEdge {
        GraphEdge::new(
            Uuid::new_v4(),
            from_node_id,
            to_node_id,
            edge_types::AUDIO_FLOW,
        )
    }

    /// Add voice infrastructure to the graph
    /// Returns (gateway_id, stt_id, tts_id)
    pub fn add_voice_infrastructure(
        graph: &mut TopologyGraph,
        gateway_id: &str,
        stt_provider: &str,
        tts_provider: &str,
    ) -> (Uuid, Uuid, Uuid) {
        let gateway_node = Self::create_gateway_node(gateway_id);
        let gateway_uuid = gateway_node.id;
        graph.add_node(gateway_node);

        let stt_node = Self::create_stt_node(stt_provider);
        let stt_uuid = stt_node.id;
        graph.add_node(stt_node);

        let tts_node = Self::create_tts_node(tts_provider);
        let tts_uuid = tts_node.id;
        graph.add_node(tts_node);

        // Connect STT and TTS to gateway
        graph.add_edge(Self::create_audio_flow_edge(gateway_uuid, stt_uuid));
        graph.add_edge(Self::create_audio_flow_edge(tts_uuid, gateway_uuid));

        (gateway_uuid, stt_uuid, tts_uuid)
    }

    /// Add a voice channel to the graph
    pub fn add_channel(
        graph: &mut TopologyGraph,
        gateway_node_id: Uuid,
        channel_id: &str,
        channel_type: &str,
    ) -> Uuid {
        let caps = match channel_type {
            "glasses" => vec![
                capabilities::AUDIO_INPUT,
                capabilities::AUDIO_OUTPUT,
                capabilities::PTT,
                capabilities::LOCAL,
            ],
            "browser" => vec![
                capabilities::AUDIO_INPUT,
                capabilities::AUDIO_OUTPUT,
                capabilities::STREAMING,
            ],
            "twilio" => vec![
                capabilities::AUDIO_INPUT,
                capabilities::AUDIO_OUTPUT,
            ],
            _ => vec![capabilities::AUDIO_INPUT],
        };

        let channel_node = Self::create_channel_node(channel_id, channel_type, caps);
        let channel_uuid = channel_node.id;
        graph.add_node(channel_node);

        // Connect channel to gateway
        graph.add_edge(Self::create_channel_gateway_edge(channel_uuid, gateway_node_id));
        graph.add_edge(Self::create_audio_flow_edge(channel_uuid, gateway_node_id));

        channel_uuid
    }

    /// Add a voice session to the graph
    pub fn add_session(
        graph: &mut TopologyGraph,
        channel_node_id: Uuid,
        session_id: &str,
        channel_type: &str,
        device_id: Option<&str>,
    ) -> Uuid {
        let session_node = Self::create_session_node(session_id, channel_type, device_id);
        let session_uuid = session_node.id;
        graph.add_node(session_node);

        // Connect session to channel
        graph.add_edge(Self::create_session_channel_edge(session_uuid, channel_node_id));
        graph.add_edge(Self::create_audio_flow_edge(session_uuid, channel_node_id));

        session_uuid
    }

    /// Remove a session from the graph
    pub fn remove_session(graph: &mut TopologyGraph, session_node_id: Uuid) -> Option<GraphNode> {
        graph.remove_node(session_node_id)
    }

    /// Mark a session as ended (keeps in graph for history)
    pub fn end_session(graph: &mut TopologyGraph, session_node_id: Uuid) {
        if let Some(node) = graph.get_node_mut(session_node_id) {
            node.status = "ended".to_string();
            if let Some(ref mut metadata) = node.metadata {
                if let Some(obj) = metadata.as_object_mut() {
                    obj.insert(
                        "ended_at".to_string(),
                        serde_json::Value::String(Utc::now().to_rfc3339()),
                    );
                }
            }
        }
    }

    /// Get all active voice sessions
    pub fn active_sessions(graph: &TopologyGraph) -> Vec<&GraphNode> {
        graph
            .nodes_of_type(node_types::VOICE_SESSION)
            .into_iter()
            .filter(|n| n.status == "active")
            .collect()
    }

    /// Get all voice channels
    pub fn voice_channels(graph: &TopologyGraph) -> Vec<&GraphNode> {
        graph.nodes_of_type(node_types::VOICE_CHANNEL)
    }

    /// Create a cluster for voice sessions of the same type
    pub fn create_channel_cluster(
        graph: &TopologyGraph,
        channel_type: &str,
    ) -> Option<ClusterInfo> {
        let sessions: Vec<Uuid> = graph
            .nodes_of_type(node_types::VOICE_SESSION)
            .into_iter()
            .filter(|n| {
                n.metadata
                    .as_ref()
                    .and_then(|m| m.get("channel_type"))
                    .and_then(|v| v.as_str())
                    == Some(channel_type)
            })
            .map(|n| n.id)
            .collect();

        if sessions.is_empty() {
            None
        } else {
            Some(
                ClusterInfo::new(
                    Uuid::new_v4(),
                    format!("{}_sessions", channel_type),
                    sessions,
                )
                .with_purpose(format!("Voice sessions from {} channel", channel_type)),
            )
        }
    }

    /// Find the path from a voice session to an agent node
    pub fn find_execution_path(
        graph: &TopologyGraph,
        session_node_id: Uuid,
        agent_node_id: Uuid,
    ) -> Option<super::engine::Path> {
        super::engine::TopologyEngine::find_shortest_path(graph, session_node_id, agent_node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_voice_infrastructure() {
        let mut graph = TopologyGraph::new();
        let (gateway_id, stt_id, tts_id) = VoiceTopology::add_voice_infrastructure(
            &mut graph,
            "main-gateway",
            "whisper",
            "chatterbox",
        );

        assert!(graph.get_node(gateway_id).is_some());
        assert!(graph.get_node(stt_id).is_some());
        assert!(graph.get_node(tts_id).is_some());

        // Check STT is marked as local
        let stt_node = graph.get_node(stt_id).unwrap();
        assert!(stt_node.has_capability(capabilities::LOCAL));

        // Check TTS is marked as local
        let tts_node = graph.get_node(tts_id).unwrap();
        assert!(tts_node.has_capability(capabilities::LOCAL));
    }

    #[test]
    fn test_add_channel_and_session() {
        let mut graph = TopologyGraph::new();
        let (gateway_id, _, _) = VoiceTopology::add_voice_infrastructure(
            &mut graph,
            "main-gateway",
            "whisper",
            "chatterbox",
        );

        let channel_id = VoiceTopology::add_channel(
            &mut graph,
            gateway_id,
            "glasses-1",
            "glasses",
        );

        let session_id = VoiceTopology::add_session(
            &mut graph,
            channel_id,
            "session-123",
            "glasses",
            Some("device-abc"),
        );

        // Verify session is active
        let sessions = VoiceTopology::active_sessions(&graph);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].ref_id, "session-123");

        // End the session
        VoiceTopology::end_session(&mut graph, session_id);
        let active_sessions = VoiceTopology::active_sessions(&graph);
        assert_eq!(active_sessions.len(), 0);
    }
}
