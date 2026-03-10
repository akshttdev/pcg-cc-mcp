//! Stub implementation of serenity-voice-model for songbird 0.4 compatibility.
//!
//! Provides the types and traits that songbird imports from `serenity_voice_model`.
//! Gateway version updated to v8 (Discord voice gateway v8).

pub mod constants {
    /// Gateway version of the Voice API.
    /// Updated from v4 to v8 for current Discord compatibility.
    pub const GATEWAY_VERSION: u8 = 8;
}

pub mod id {
    use std::fmt;
    use serde::{Deserialize, Serialize};
    use super::util::json_safe_u64;

    #[derive(
        Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
    )]
    pub struct GuildId(#[serde(with = "json_safe_u64")] pub u64);

    impl fmt::Display for GuildId {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(&self.0, f)
        }
    }

    #[derive(
        Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
    )]
    pub struct UserId(#[serde(with = "json_safe_u64")] pub u64);

    impl fmt::Display for UserId {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(&self.0, f)
        }
    }
}

mod util {
    pub(crate) mod json_safe_u64 {
        use core::fmt::{Formatter, Result as FmtResult};
        use serde::de::{Deserializer, Error, Visitor};
        use serde::ser::Serializer;

        struct U64Visitor;

        impl Visitor<'_> for U64Visitor {
            type Value = u64;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                formatter.write_str("a u64 represented by a string or number")
            }

            fn visit_u64<E: Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(value)
            }

            fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
                value.parse::<u64>().map_err(E::custom)
            }
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(U64Visitor)
        }

        pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.collect_str(value)
        }
    }
}

// --- SpeakingState (bitflags) ---

use bitflags::bitflags;
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

bitflags! {
    /// Flag set describing how a speaker is sending audio.
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    pub struct SpeakingState: u8 {
        /// Normal transmission of voice audio.
        const MICROPHONE = 1;
        /// Transmission of context audio for video, no speaking indicator.
        const SOUNDSHARE = 1 << 1;
        /// Priority speaker, lowering audio of other speakers.
        const PRIORITY = 1 << 2;
    }
}

impl SpeakingState {
    pub fn microphone(self) -> bool {
        self.contains(Self::MICROPHONE)
    }

    pub fn soundshare(self) -> bool {
        self.contains(Self::SOUNDSHARE)
    }

    pub fn priority(self) -> bool {
        self.contains(Self::PRIORITY)
    }
}

impl<'de> Deserialize<'de> for SpeakingState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::from_bits_truncate(u8::deserialize(deserializer)?))
    }
}

impl Serialize for SpeakingState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(self.bits())
    }
}

// --- ProtocolData ---

use std::net::IpAddr;

/// The client's response to a connection offer.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct ProtocolData {
    /// IP address of the client as seen by the server.
    pub address: IpAddr,
    /// The client's chosen encryption mode.
    pub mode: String,
    /// UDP source port of the client as seen by the server.
    pub port: u16,
}

// --- CloseCode ---

/// Discord Voice Gateway Websocket close codes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseCode {
    UnknownOpcode = 4001,
    InvalidPayload = 4002,
    NotAuthenticated = 4003,
    AuthenticationFailed = 4004,
    AlreadyAuthenticated = 4005,
    SessionInvalid = 4006,
    SessionTimeout = 4009,
    ServerNotFound = 4011,
    UnknownProtocol = 4012,
    Disconnected = 4014,
    VoiceServerCrash = 4015,
    UnknownEncryptionMode = 4016,
}

impl CloseCode {
    pub fn should_resume(&self) -> bool {
        matches!(self, CloseCode::VoiceServerCrash | CloseCode::SessionTimeout)
    }
}

impl num_traits::cast::FromPrimitive for CloseCode {
    fn from_u64(n: u64) -> Option<Self> {
        Some(match n {
            4001 => Self::UnknownOpcode,
            4002 => Self::InvalidPayload,
            4003 => Self::NotAuthenticated,
            4004 => Self::AuthenticationFailed,
            4005 => Self::AlreadyAuthenticated,
            4006 => Self::SessionInvalid,
            4009 => Self::SessionTimeout,
            4011 => Self::ServerNotFound,
            4012 => Self::UnknownProtocol,
            4014 => Self::Disconnected,
            4015 => Self::VoiceServerCrash,
            4016 => Self::UnknownEncryptionMode,
            _ => return None,
        })
    }

    fn from_i64(n: i64) -> Option<Self> {
        Self::from_u64(n as u64)
    }
}

// --- Opcode ---

use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Deserialize_repr, Serialize_repr,
)]
#[non_exhaustive]
#[repr(u8)]
pub enum Opcode {
    Identify = 0,
    SelectProtocol = 1,
    Ready = 2,
    Heartbeat = 3,
    SessionDescription = 4,
    Speaking = 5,
    HeartbeatAck = 6,
    Resume = 7,
    Hello = 8,
    Resumed = 9,
    ClientConnect = 12,
    ClientDisconnect = 13,
}

// --- Payload types ---

pub mod payload {
    use serde::{Deserialize, Serialize};
    use std::net::IpAddr;
    use crate::id::*;
    use crate::ProtocolData;
    use crate::SpeakingState;
    use crate::util::json_safe_u64;

    /// Message indicating that another user has connected to the voice channel.
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
    pub struct ClientConnect {
        pub audio_ssrc: u32,
        pub user_id: UserId,
        pub video_ssrc: u32,
    }

    /// Message indicating that another user has disconnected from the voice channel.
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
    pub struct ClientDisconnect {
        pub user_id: UserId,
    }

    /// Used to keep the websocket connection alive.
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct Heartbeat {
        pub nonce: u64,
    }

    /// Heartbeat ACK.
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct HeartbeatAck {
        #[serde(with = "json_safe_u64")]
        pub nonce: u64,
    }

    /// Used to determine how often the client must send a heartbeat.
    #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
    pub struct Hello {
        pub heartbeat_interval: f64,
    }

    /// Used to begin a voice websocket connection.
    #[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
    pub struct Identify {
        pub server_id: GuildId,
        pub session_id: String,
        pub token: String,
        pub user_id: UserId,
    }

    /// RTP server's connection offer and supported encryption modes.
    #[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
    pub struct Ready {
        pub ip: IpAddr,
        pub modes: Vec<String>,
        pub port: u16,
        pub ssrc: u32,
    }

    /// Sent by the client after a disconnect to attempt to resume a session.
    #[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
    pub struct Resume {
        pub server_id: GuildId,
        pub session_id: String,
        pub token: String,
    }

    /// Used to select the voice protocol and encryption mechanism.
    #[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
    pub struct SelectProtocol {
        pub data: ProtocolData,
        pub protocol: String,
    }

    /// Server's confirmation of a negotiated encryption scheme.
    #[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
    pub struct SessionDescription {
        pub mode: String,
        pub secret_key: Vec<u8>,
    }

    /// Used to indicate which users are speaking.
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
    pub struct Speaking {
        pub delay: Option<u32>,
        pub speaking: SpeakingState,
        pub ssrc: u32,
        pub user_id: Option<UserId>,
    }
}

// --- Event enum ---

use serde::de::value::U8Deserializer;
use serde::de::{Error as DeError, IntoDeserializer, MapAccess, Unexpected, Visitor};
use serde::ser::SerializeStruct;
use serde_json::value::RawValue;

use payload::*;

/// A representation of data received for voice gateway events.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Event {
    Identify(Identify),
    SelectProtocol(SelectProtocol),
    Ready(Ready),
    Heartbeat(Heartbeat),
    SessionDescription(SessionDescription),
    Speaking(Speaking),
    HeartbeatAck(HeartbeatAck),
    Resume(Resume),
    Hello(Hello),
    Resumed,
    ClientConnect(ClientConnect),
    ClientDisconnect(ClientDisconnect),
}

impl Event {
    pub fn kind(&self) -> Opcode {
        match self {
            Event::Identify(_) => Opcode::Identify,
            Event::SelectProtocol(_) => Opcode::SelectProtocol,
            Event::Ready(_) => Opcode::Ready,
            Event::Heartbeat(_) => Opcode::Heartbeat,
            Event::SessionDescription(_) => Opcode::SessionDescription,
            Event::Speaking(_) => Opcode::Speaking,
            Event::HeartbeatAck(_) => Opcode::HeartbeatAck,
            Event::Resume(_) => Opcode::Resume,
            Event::Hello(_) => Opcode::Hello,
            Event::Resumed => Opcode::Resumed,
            Event::ClientConnect(_) => Opcode::ClientConnect,
            Event::ClientDisconnect(_) => Opcode::ClientDisconnect,
        }
    }
}

impl Serialize for Event {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Event", 2)?;
        s.serialize_field("op", &self.kind())?;
        match self {
            Event::Identify(e) => s.serialize_field("d", e)?,
            Event::SelectProtocol(e) => s.serialize_field("d", e)?,
            Event::Ready(e) => s.serialize_field("d", e)?,
            Event::Heartbeat(e) => s.serialize_field("d", e)?,
            Event::SessionDescription(e) => s.serialize_field("d", e)?,
            Event::Speaking(e) => s.serialize_field("d", e)?,
            Event::HeartbeatAck(e) => s.serialize_field("d", e)?,
            Event::Resume(e) => s.serialize_field("d", e)?,
            Event::Hello(e) => s.serialize_field("d", e)?,
            Event::Resumed => s.serialize_field("d", &None::<()>)?,
            Event::ClientConnect(e) => s.serialize_field("d", e)?,
            Event::ClientDisconnect(e) => s.serialize_field("d", e)?,
        }
        s.end()
    }
}

struct EventVisitor;

impl<'de> Visitor<'de> for EventVisitor {
    type Value = Event;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a map with at least two keys ('d', 'op')")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut d = None;
        let mut op = None;

        loop {
            match map.next_key::<&str>()? {
                Some("op") => {
                    let raw = map.next_value::<u8>()?;
                    let des: U8Deserializer<A::Error> = raw.into_deserializer();
                    let valid_op = Opcode::deserialize(des).map_err(|_| {
                        DeError::invalid_value(
                            Unexpected::Unsigned(raw.into()),
                            &"opcode in [0--9] + [12--13]",
                        )
                    })?;
                    op = Some(valid_op);
                },
                Some("d") => match op {
                    Some(Opcode::Identify) => return Ok(map.next_value::<Identify>()?.into()),
                    Some(Opcode::SelectProtocol) =>
                        return Ok(map.next_value::<SelectProtocol>()?.into()),
                    Some(Opcode::Ready) => return Ok(map.next_value::<Ready>()?.into()),
                    Some(Opcode::Heartbeat) => return Ok(map.next_value::<Heartbeat>()?.into()),
                    Some(Opcode::HeartbeatAck) =>
                        return Ok(map.next_value::<HeartbeatAck>()?.into()),
                    Some(Opcode::SessionDescription) =>
                        return Ok(map.next_value::<SessionDescription>()?.into()),
                    Some(Opcode::Speaking) => return Ok(map.next_value::<Speaking>()?.into()),
                    Some(Opcode::Resume) => return Ok(map.next_value::<Resume>()?.into()),
                    Some(Opcode::Hello) => return Ok(map.next_value::<Hello>()?.into()),
                    Some(Opcode::Resumed) => {
                        let _ = map.next_value::<Option<()>>()?;
                        return Ok(Event::Resumed);
                    },
                    Some(Opcode::ClientConnect) =>
                        return Ok(map.next_value::<ClientConnect>()?.into()),
                    Some(Opcode::ClientDisconnect) =>
                        return Ok(map.next_value::<ClientDisconnect>()?.into()),
                    None => {
                        d = Some(map.next_value::<&RawValue>()?);
                    },
                },
                Some(_) => {
                    // skip unknown keys
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                },
                None => {
                    if d.is_none() {
                        return Err(DeError::missing_field("d"));
                    } else if op.is_none() {
                        return Err(DeError::missing_field("op"));
                    }
                },
            }

            if d.is_some() && op.is_some() {
                break;
            }
        }

        let d = d.expect("Struct body known to exist if loop has been escaped.").get();
        let op = op.expect("Struct variant known to exist if loop has been escaped.");

        (match op {
            Opcode::Identify => serde_json::from_str::<Identify>(d).map(Into::into),
            Opcode::SelectProtocol => serde_json::from_str::<SelectProtocol>(d).map(Into::into),
            Opcode::Ready => serde_json::from_str::<Ready>(d).map(Into::into),
            Opcode::Heartbeat => serde_json::from_str::<Heartbeat>(d).map(Into::into),
            Opcode::HeartbeatAck => serde_json::from_str::<HeartbeatAck>(d).map(Into::into),
            Opcode::SessionDescription =>
                serde_json::from_str::<SessionDescription>(d).map(Into::into),
            Opcode::Speaking => serde_json::from_str::<Speaking>(d).map(Into::into),
            Opcode::Resume => serde_json::from_str::<Resume>(d).map(Into::into),
            Opcode::Hello => serde_json::from_str::<Hello>(d).map(Into::into),
            Opcode::Resumed => Ok(Event::Resumed),
            Opcode::ClientConnect => serde_json::from_str::<ClientConnect>(d).map(Into::into),
            Opcode::ClientDisconnect =>
                serde_json::from_str::<ClientDisconnect>(d).map(Into::into),
        })
        .map_err(DeError::custom)
    }
}

impl<'de> Deserialize<'de> for Event {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(EventVisitor)
    }
}

// --- From impls for Event ---

impl From<Identify> for Event {
    fn from(i: Identify) -> Self { Event::Identify(i) }
}
impl From<SelectProtocol> for Event {
    fn from(i: SelectProtocol) -> Self { Event::SelectProtocol(i) }
}
impl From<Ready> for Event {
    fn from(i: Ready) -> Self { Event::Ready(i) }
}
impl From<Heartbeat> for Event {
    fn from(i: Heartbeat) -> Self { Event::Heartbeat(i) }
}
impl From<SessionDescription> for Event {
    fn from(i: SessionDescription) -> Self { Event::SessionDescription(i) }
}
impl From<Speaking> for Event {
    fn from(i: Speaking) -> Self { Event::Speaking(i) }
}
impl From<HeartbeatAck> for Event {
    fn from(i: HeartbeatAck) -> Self { Event::HeartbeatAck(i) }
}
impl From<Resume> for Event {
    fn from(i: Resume) -> Self { Event::Resume(i) }
}
impl From<Hello> for Event {
    fn from(i: Hello) -> Self { Event::Hello(i) }
}
impl From<ClientConnect> for Event {
    fn from(i: ClientConnect) -> Self { Event::ClientConnect(i) }
}
impl From<ClientDisconnect> for Event {
    fn from(i: ClientDisconnect) -> Self { Event::ClientDisconnect(i) }
}

// --- Re-exports matching the real crate's public API ---

pub use num_traits::FromPrimitive;
