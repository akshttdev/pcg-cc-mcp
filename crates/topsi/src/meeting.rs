//! Meeting mode for Topsi - silent AI observer that documents conversations
//!
//! Enables real-time meeting transcription with speaker identification,
//! on-demand command execution when directly addressed, and structured
//! notes generation.

use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use ts_rs::TS;

/// In-memory state for an active meeting session
#[derive(Debug, Clone)]
pub struct MeetingState {
    /// Meeting session ID (matches DB row)
    pub session_id: String,
    /// Project this meeting belongs to
    pub project_id: String,
    /// User who started the meeting
    pub started_by: String,
    /// Running transcript entries
    pub transcript: Vec<MeetingTranscriptEntry>,
    /// Speaker tracking: speaker_label -> SpeakerInfo
    pub speakers: HashMap<String, SpeakerInfo>,
    /// Number of audio chunks processed
    pub chunk_count: u32,
    /// Whether Topsi is currently being addressed
    pub is_addressed: bool,
    /// Meeting start time
    pub started_at: DateTime<Utc>,
    /// Current elapsed time in milliseconds
    pub elapsed_ms: i64,
}

impl MeetingState {
    pub fn new(session_id: String, project_id: String, started_by: String) -> Self {
        Self {
            session_id,
            project_id,
            started_by,
            transcript: Vec::new(),
            speakers: HashMap::new(),
            chunk_count: 0,
            is_addressed: false,
            started_at: Utc::now(),
            elapsed_ms: 0,
        }
    }

    /// Get recent transcript text for context (last N entries)
    pub fn recent_context(&self, max_entries: usize) -> String {
        let start = if self.transcript.len() > max_entries {
            self.transcript.len() - max_entries
        } else {
            0
        };

        self.transcript[start..]
            .iter()
            .map(|entry| {
                let speaker = entry.speaker_label.as_deref().unwrap_or("Unknown");
                format!("[{}]: {}", speaker, entry.text)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A single transcript entry from the meeting
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MeetingTranscriptEntry {
    /// Speaker label (e.g. "Speaker 1", "Alice")
    pub speaker_label: Option<String>,
    /// Transcribed text
    pub text: String,
    /// Confidence score from STT
    pub confidence: f64,
    /// Start time in milliseconds from meeting start
    pub start_time_ms: i64,
    /// End time in milliseconds from meeting start
    pub end_time_ms: i64,
    /// Whether this segment addressed Topsi
    pub is_topsi_addressed: bool,
    /// Segment index within the meeting
    pub segment_index: i32,
}

/// Information about a detected speaker
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerInfo {
    /// Speaker label from diarization (e.g. "SPEAKER_00")
    pub label: String,
    /// Optional display name (user-assigned or auto-detected)
    pub display_name: Option<String>,
    /// Number of segments spoken
    pub segment_count: u32,
    /// Total speaking time in milliseconds
    pub speaking_time_ms: i64,
}

impl SpeakerInfo {
    pub fn new(label: String) -> Self {
        Self {
            label,
            display_name: None,
            segment_count: 0,
            speaking_time_ms: 0,
        }
    }

    pub fn add_segment(&mut self, duration_ms: i64) {
        self.segment_count += 1;
        self.speaking_time_ms += duration_ms;
    }
}

/// Result of wake word detection
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct WakeWordResult {
    /// Whether the wake word was detected
    pub detected: bool,
    /// Text following the wake word (the addressed command/question)
    pub addressed_text: Option<String>,
    /// Position of the wake word in the original text
    pub position: Option<usize>,
}

/// Manages active meeting sessions in memory
pub struct MeetingManager {
    /// Active meetings: session_id -> MeetingState
    meetings: Arc<RwLock<HashMap<String, MeetingState>>>,
}

impl MeetingManager {
    pub fn new() -> Self {
        Self {
            meetings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start tracking a new meeting
    pub async fn start_meeting(
        &self,
        session_id: String,
        project_id: String,
        started_by: String,
    ) -> MeetingState {
        let state = MeetingState::new(session_id.clone(), project_id, started_by);
        let mut meetings = self.meetings.write().await;
        meetings.insert(session_id, state.clone());
        state
    }

    /// End a meeting and return the final state
    pub async fn end_meeting(&self, session_id: &str) -> Option<MeetingState> {
        let mut meetings = self.meetings.write().await;
        meetings.remove(session_id)
    }

    /// Get a reference to an active meeting's state
    pub async fn get_meeting(&self, session_id: &str) -> Option<MeetingState> {
        let meetings = self.meetings.read().await;
        meetings.get(session_id).cloned()
    }

    /// Add a transcript entry to an active meeting
    pub async fn add_transcript_entry(
        &self,
        session_id: &str,
        entry: MeetingTranscriptEntry,
    ) -> bool {
        let mut meetings = self.meetings.write().await;
        if let Some(meeting) = meetings.get_mut(session_id) {
            // Update speaker tracking
            if let Some(ref label) = entry.speaker_label {
                let speaker = meeting
                    .speakers
                    .entry(label.clone())
                    .or_insert_with(|| SpeakerInfo::new(label.clone()));
                speaker.add_segment(entry.end_time_ms - entry.start_time_ms);
            }

            meeting.is_addressed = entry.is_topsi_addressed;
            meeting.elapsed_ms = entry.end_time_ms;
            meeting.transcript.push(entry);
            meeting.chunk_count += 1;
            true
        } else {
            false
        }
    }

    /// Increment chunk count for a meeting
    pub async fn increment_chunk_count(&self, session_id: &str) {
        let mut meetings = self.meetings.write().await;
        if let Some(meeting) = meetings.get_mut(session_id) {
            meeting.chunk_count += 1;
        }
    }

    /// Get the number of active meetings
    pub async fn active_count(&self) -> usize {
        let meetings = self.meetings.read().await;
        meetings.len()
    }

    /// Detect wake word in transcribed text
    ///
    /// Looks for "topsi", "topsy", "top see" (case-insensitive).
    /// Returns the text following the wake word as the addressed command.
    pub fn detect_wake_word(text: &str) -> WakeWordResult {
        let lower = text.to_lowercase();

        // Wake word patterns to match
        let patterns = ["topsi", "topsy", "top see", "topsie", "topsey"];

        for pattern in &patterns {
            if let Some(pos) = lower.find(pattern) {
                let after_wake = pos + pattern.len();
                let addressed_text = if after_wake < text.len() {
                    let remainder = text[after_wake..].trim();
                    // Strip common separators: ", " or ": " or "! " at the start
                    let cleaned = remainder
                        .trim_start_matches(',')
                        .trim_start_matches(':')
                        .trim_start_matches('!')
                        .trim_start_matches('?')
                        .trim();
                    if cleaned.is_empty() {
                        None
                    } else {
                        Some(cleaned.to_string())
                    }
                } else {
                    None
                };

                return WakeWordResult {
                    detected: true,
                    addressed_text,
                    position: Some(pos),
                };
            }
        }

        WakeWordResult {
            detected: false,
            addressed_text: None,
            position: None,
        }
    }
}

impl Default for MeetingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Structured meeting notes generated by LLM
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MeetingNotes {
    /// Executive summary of the meeting
    pub summary: String,
    /// Main topics discussed
    pub topics: Vec<String>,
    /// Decisions made during the meeting
    pub decisions: Vec<String>,
    /// Action items with assignees and deadlines
    pub action_items: Vec<ActionItem>,
    /// Open questions that remain unresolved
    pub open_questions: Vec<String>,
    /// Participants identified in the meeting
    pub participants: Vec<String>,
}

/// An action item from a meeting
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ActionItem {
    /// Description of the action
    pub description: String,
    /// Person assigned to this action
    pub assignee: Option<String>,
    /// Deadline for completion
    pub deadline: Option<String>,
    /// Priority level
    pub priority: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wake_word_detection_basic() {
        let result = MeetingManager::detect_wake_word("Hey Topsi, what do you think?");
        assert!(result.detected);
        assert_eq!(
            result.addressed_text,
            Some("what do you think?".to_string())
        );
    }

    #[test]
    fn test_wake_word_detection_variant() {
        let result = MeetingManager::detect_wake_word("Topsy, can you summarize?");
        assert!(result.detected);
        assert_eq!(
            result.addressed_text,
            Some("can you summarize?".to_string())
        );
    }

    #[test]
    fn test_wake_word_detection_top_see() {
        let result = MeetingManager::detect_wake_word("top see what's the status");
        assert!(result.detected);
        assert_eq!(result.addressed_text, Some("what's the status".to_string()));
    }

    #[test]
    fn test_wake_word_not_detected() {
        let result = MeetingManager::detect_wake_word("Let's discuss the budget");
        assert!(!result.detected);
        assert!(result.addressed_text.is_none());
    }

    #[test]
    fn test_wake_word_case_insensitive() {
        let result = MeetingManager::detect_wake_word("TOPSI, give me a summary");
        assert!(result.detected);
        assert_eq!(result.addressed_text, Some("give me a summary".to_string()));
    }

    #[test]
    fn test_wake_word_no_text_after() {
        let result = MeetingManager::detect_wake_word("Topsi");
        assert!(result.detected);
        assert!(result.addressed_text.is_none());
    }

    #[tokio::test]
    async fn test_meeting_manager_lifecycle() {
        let manager = MeetingManager::new();

        // Start meeting
        let state = manager
            .start_meeting(
                "session-1".to_string(),
                "project-1".to_string(),
                "user-1".to_string(),
            )
            .await;
        assert_eq!(state.session_id, "session-1");
        assert_eq!(manager.active_count().await, 1);

        // Add transcript entry
        let entry = MeetingTranscriptEntry {
            speaker_label: Some("Speaker 1".to_string()),
            text: "Let's get started".to_string(),
            confidence: 0.95,
            start_time_ms: 0,
            end_time_ms: 2000,
            is_topsi_addressed: false,
            segment_index: 0,
        };
        assert!(manager.add_transcript_entry("session-1", entry).await);

        // Verify state
        let state = manager.get_meeting("session-1").await.unwrap();
        assert_eq!(state.transcript.len(), 1);
        assert_eq!(state.speakers.len(), 1);

        // End meeting
        let final_state = manager.end_meeting("session-1").await.unwrap();
        assert_eq!(final_state.transcript.len(), 1);
        assert_eq!(manager.active_count().await, 0);
    }

    #[test]
    fn test_recent_context() {
        let mut state = MeetingState::new("s1".to_string(), "p1".to_string(), "u1".to_string());

        for i in 0..5 {
            state.transcript.push(MeetingTranscriptEntry {
                speaker_label: Some(format!("Speaker {}", i % 2 + 1)),
                text: format!("Message {}", i),
                confidence: 0.9,
                start_time_ms: i as i64 * 5000,
                end_time_ms: (i as i64 + 1) * 5000,
                is_topsi_addressed: false,
                segment_index: i,
            });
        }

        let context = state.recent_context(3);
        assert!(context.contains("Message 2"));
        assert!(context.contains("Message 3"));
        assert!(context.contains("Message 4"));
        assert!(!context.contains("Message 0"));
    }
}
