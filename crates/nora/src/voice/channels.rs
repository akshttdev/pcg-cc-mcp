//! Voice channel implementations
//!
//! Stub module for various voice input channels.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Type of voice channel
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ChannelType {
    Browser,
    Glasses,
    Twilio,
    Desktop,
}

/// Audio chunk from a voice channel
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub data: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Voice channel trait
pub trait VoiceChannel: Send + Sync {
    fn channel_type(&self) -> ChannelType;
}

/// Voice channel session
#[derive(Debug, Clone)]
pub struct VoiceChannelSession {
    pub session_id: String,
    pub channel_type: ChannelType,
}

/// Voice channel event
#[derive(Debug, Clone)]
pub enum VoiceChannelEvent {
    AudioReceived(AudioChunk),
    SessionStarted(String),
    SessionEnded(String),
}

/// Browser channel for web-based voice input
pub struct BrowserChannel;

impl VoiceChannel for BrowserChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Browser
    }
}

/// Browser voice message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserVoiceMessage {
    pub audio_data: String,
    pub session_id: String,
}

/// Glasses channel for APN mesh voice input
pub struct GlassesChannel;

impl VoiceChannel for GlassesChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Glasses
    }
}

/// Mesh voice message for APN network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshVoiceMessage {
    pub payload: MeshVoicePayload,
    pub source_node: String,
}

/// Mesh voice payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshVoicePayload {
    pub audio_data: Vec<u8>,
    pub timestamp: u64,
}
