//! Memory and context management for Nora

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::agent::NoraRequest;

/// Conversation memory management
#[derive(Debug, Clone)]
pub struct ConversationMemory {
    interactions: Vec<InteractionRecord>,
    context_summaries: HashMap<String, ContextSummary>,
    max_interactions: usize,
    #[allow(dead_code)]
    encryption_enabled: bool,
    pending_action: Option<PendingAction>,
}

/// Pending action awaiting confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub action_id: String,
    pub action_type: PendingActionType,
    pub project_name: Option<String>,
    pub project_id: Option<Uuid>,
    pub tasks: Vec<PendingTask>,
    pub created_at: DateTime<Utc>,
}

/// Types of pending actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PendingActionType {
    CreateTasks,
    UpdateProject,
    DeleteTasks,
}

/// Task awaiting creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTask {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Individual interaction record
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InteractionRecord {
    pub interaction_id: String,
    pub session_id: String,
    pub user_input: String,
    pub nora_response: String,
    pub interaction_type: String,
    pub context_tags: Vec<String>,
    pub sentiment: Option<SentimentAnalysis>,
    pub timestamp: DateTime<Utc>,
    pub processing_time_ms: u64,
}

/// Context summary for efficient retrieval
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContextSummary {
    pub session_id: String,
    pub key_topics: Vec<String>,
    pub executive_priorities: Vec<String>,
    pub action_items: Vec<ActionItem>,
    pub decisions_made: Vec<Decision>,
    pub participants: Vec<String>,
    pub summary_text: String,
    pub last_updated: DateTime<Utc>,
}

/// Sentiment analysis result
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SentimentAnalysis {
    pub sentiment: Sentiment,
    pub confidence: f32,
    pub emotional_tone: Vec<EmotionalTone>,
}

/// Sentiment classification
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Sentiment {
    Positive,
    Negative,
    Neutral,
    Mixed,
}

/// Emotional tone detection
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum EmotionalTone {
    Formal,
    Urgent,
    Frustrated,
    Satisfied,
    Curious,
    Concerned,
}

/// Action item from conversations
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ActionItem {
    pub id: String,
    pub description: String,
    pub assigned_to: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub priority: ActionPriority,
    pub status: ActionStatus,
    pub created_at: DateTime<Utc>,
}

/// Action item priority
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ActionPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Action item status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ActionStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

/// Decision record
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub id: String,
    pub description: String,
    pub decision_maker: String,
    pub rationale: String,
    pub alternatives_considered: Vec<String>,
    pub impact_assessment: String,
    pub made_at: DateTime<Utc>,
}

/// Executive context for strategic decision making
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecutiveContext {
    pub current_priorities: Vec<ExecutivePriority>,
    pub active_projects: Vec<ProjectContext>,
    pub key_stakeholders: Vec<Stakeholder>,
    pub upcoming_deadlines: Vec<Deadline>,
    pub recent_decisions: Vec<Decision>,
    pub performance_metrics: HashMap<String, MetricValue>,
    pub market_conditions: MarketContext,
    pub team_status: TeamStatus,
    pub last_updated: DateTime<Utc>,
}

/// Executive priority
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecutivePriority {
    pub id: String,
    pub title: String,
    pub description: String,
    pub urgency: PriorityUrgency,
    pub impact: PriorityImpact,
    pub owner: String,
    pub target_date: Option<DateTime<Utc>>,
    pub status: PriorityStatus,
}

/// Priority urgency levels
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PriorityUrgency {
    Low,
    Medium,
    High,
    Critical,
}

/// Priority impact levels
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PriorityImpact {
    Low,
    Medium,
    High,
    Strategic,
}

/// Priority status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PriorityStatus {
    Planned,
    InProgress,
    OnTrack,
    AtRisk,
    Delayed,
    Completed,
}

/// Project context
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectContext {
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub status: ProjectStatus,
    pub progress_percentage: f32,
    pub team_members: Vec<String>,
    pub budget_status: BudgetStatus,
    pub key_milestones: Vec<Milestone>,
    pub risks: Vec<Risk>,
}

/// Project status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ProjectStatus {
    Planning,
    InProgress,
    OnHold,
    AtRisk,
    Completed,
    Cancelled,
}

/// Budget status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BudgetStatus {
    pub allocated: f64,
    pub spent: f64,
    pub remaining: f64,
    pub burn_rate: f64,
    pub forecast_completion: f64,
}

/// Project milestone
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Milestone {
    pub id: String,
    pub name: String,
    pub due_date: DateTime<Utc>,
    pub status: MilestoneStatus,
    pub completion_percentage: f32,
}

/// Milestone status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum MilestoneStatus {
    NotStarted,
    InProgress,
    Completed,
    Overdue,
}

/// Risk assessment
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Risk {
    pub id: String,
    pub description: String,
    pub probability: RiskProbability,
    pub impact: RiskImpact,
    pub mitigation_plan: String,
    pub owner: String,
}

/// Risk probability
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum RiskProbability {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Risk impact
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum RiskImpact {
    Low,
    Medium,
    High,
    Critical,
}

/// Stakeholder information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Stakeholder {
    pub id: String,
    pub name: String,
    pub role: String,
    pub influence_level: InfluenceLevel,
    pub communication_preferences: Vec<CommunicationPreference>,
    pub last_interaction: Option<DateTime<Utc>>,
}

/// Stakeholder influence level
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum InfluenceLevel {
    Low,
    Medium,
    High,
    Executive,
}

/// Communication preferences
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CommunicationPreference {
    Email,
    Phone,
    VideoCall,
    InPerson,
    Slack,
    Teams,
}

/// Deadline tracking
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Deadline {
    pub id: String,
    pub title: String,
    pub description: String,
    pub due_date: DateTime<Utc>,
    pub owner: String,
    pub status: DeadlineStatus,
    pub criticality: DeadlineCriticality,
}

/// Deadline status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum DeadlineStatus {
    OnTrack,
    AtRisk,
    Overdue,
    Completed,
}

/// Deadline criticality
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum DeadlineCriticality {
    Low,
    Medium,
    High,
    BusinessCritical,
}

/// Metric value with context
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MetricValue {
    pub value: f64,
    pub unit: String,
    pub trend: TrendDirection,
    pub last_updated: DateTime<Utc>,
    pub target: Option<f64>,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TrendDirection {
    Up,
    Down,
    Stable,
    Volatile,
}

/// Market context information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MarketContext {
    pub sector_performance: String,
    pub competitive_landscape: String,
    pub regulatory_changes: Vec<RegulatoryChange>,
    pub market_sentiment: MarketSentiment,
    pub key_trends: Vec<String>,
}

/// Regulatory change
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RegulatoryChange {
    pub title: String,
    pub description: String,
    pub effective_date: DateTime<Utc>,
    pub impact_level: RegulatoryImpact,
}

/// Regulatory impact level
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum RegulatoryImpact {
    Low,
    Medium,
    High,
    Critical,
}

/// Market sentiment
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum MarketSentiment {
    Bullish,
    Bearish,
    Neutral,
    Volatile,
}

/// Team status overview
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TeamStatus {
    pub total_members: u32,
    pub active_members: u32,
    pub utilization_rate: f32,
    pub morale_score: Option<f32>,
    pub recent_achievements: Vec<String>,
    pub current_challenges: Vec<String>,
}

impl ConversationMemory {
    pub fn new() -> Self {
        Self {
            interactions: Vec::new(),
            context_summaries: HashMap::new(),
            max_interactions: 1000,
            encryption_enabled: false,
            pending_action: None,
        }
    }

    /// Set a pending action awaiting confirmation
    pub fn set_pending_action(&mut self, action: PendingAction) {
        self.pending_action = Some(action);
    }

    /// Get the current pending action if any
    pub fn get_pending_action(&self) -> Option<&PendingAction> {
        self.pending_action.as_ref()
    }

    /// Clear the pending action
    pub fn clear_pending_action(&mut self) -> Option<PendingAction> {
        self.pending_action.take()
    }

    pub async fn add_interaction(
        &mut self,
        request: &NoraRequest,
        response: &str,
    ) -> crate::Result<()> {
        let interaction = InteractionRecord {
            interaction_id: uuid::Uuid::new_v4().to_string(),
            session_id: request.session_id.clone(),
            user_input: request.content.clone(),
            nora_response: response.to_string(),
            interaction_type: format!("{:?}", request.request_type),
            context_tags: self.extract_context_tags(&request.content, response),
            sentiment: self.analyze_sentiment(&request.content),
            timestamp: Utc::now(),
            processing_time_ms: 0, // Would be set by caller
        };

        self.interactions.push(interaction);

        // Trim if exceeding max size
        if self.interactions.len() > self.max_interactions {
            self.interactions.remove(0);
        }

        // Update context summary
        self.update_context_summary(&request.session_id).await?;

        Ok(())
    }

    pub fn recent_interactions(&self, limit: usize) -> Vec<InteractionRecord> {
        if self.interactions.is_empty() {
            return Vec::new();
        }

        let total = self.interactions.len();
        let start = total.saturating_sub(limit);
        self.interactions[start..].to_vec()
    }

    fn extract_context_tags(&self, input: &str, response: &str) -> Vec<String> {
        let mut tags = Vec::new();
        let combined_text = format!("{} {}", input, response);
        let lower_text = combined_text.to_lowercase();

        // Executive context tags
        if lower_text.contains("meeting") || lower_text.contains("conference") {
            tags.push("meeting".to_string());
        }
        if lower_text.contains("strategy") || lower_text.contains("strategic") {
            tags.push("strategy".to_string());
        }
        if lower_text.contains("decision") || lower_text.contains("decide") {
            tags.push("decision".to_string());
        }
        if lower_text.contains("deadline") || lower_text.contains("urgent") {
            tags.push("urgent".to_string());
        }
        if lower_text.contains("project") {
            tags.push("project".to_string());
        }
        if lower_text.contains("budget") || lower_text.contains("financial") {
            tags.push("financial".to_string());
        }
        if lower_text.contains("team") || lower_text.contains("staff") {
            tags.push("team".to_string());
        }

        tags
    }

    fn analyze_sentiment(&self, text: &str) -> Option<SentimentAnalysis> {
        // Simple sentiment analysis - in a real implementation,
        // you would use a proper NLP library or service
        let lower_text = text.to_lowercase();

        let positive_words = [
            "good",
            "excellent",
            "great",
            "pleased",
            "happy",
            "satisfied",
        ];
        let negative_words = [
            "bad",
            "terrible",
            "frustrated",
            "angry",
            "disappointed",
            "concerned",
        ];
        let urgent_words = ["urgent", "immediate", "critical", "emergency", "asap"];

        let positive_count = positive_words
            .iter()
            .filter(|&word| lower_text.contains(word))
            .count();
        let negative_count = negative_words
            .iter()
            .filter(|&word| lower_text.contains(word))
            .count();

        let sentiment = if positive_count > negative_count {
            Sentiment::Positive
        } else if negative_count > positive_count {
            Sentiment::Negative
        } else if positive_count == negative_count && positive_count > 0 {
            Sentiment::Mixed
        } else {
            Sentiment::Neutral
        };

        let mut emotional_tone = Vec::new();
        if urgent_words.iter().any(|&word| lower_text.contains(word)) {
            emotional_tone.push(EmotionalTone::Urgent);
        }
        if lower_text.contains("formal") || lower_text.contains("official") {
            emotional_tone.push(EmotionalTone::Formal);
        }

        Some(SentimentAnalysis {
            sentiment,
            confidence: 0.7, // Placeholder confidence
            emotional_tone,
        })
    }

    async fn update_context_summary(&mut self, session_id: &str) -> crate::Result<()> {
        let session_interactions: Vec<_> = self
            .interactions
            .iter()
            .filter(|i| i.session_id == session_id)
            .collect();

        if session_interactions.is_empty() {
            return Ok(());
        }

        let key_topics = self.extract_key_topics(&session_interactions);
        let executive_priorities = self.extract_executive_priorities(&session_interactions);
        let action_items = self.extract_action_items(&session_interactions);
        let decisions_made = self.extract_decisions(&session_interactions);

        let summary = ContextSummary {
            session_id: session_id.to_string(),
            key_topics,
            executive_priorities,
            action_items,
            decisions_made,
            participants: vec!["user".to_string(), "nora".to_string()],
            summary_text: self.generate_summary_text(&session_interactions),
            last_updated: Utc::now(),
        };

        self.context_summaries
            .insert(session_id.to_string(), summary);

        Ok(())
    }

    fn extract_key_topics(&self, interactions: &[&InteractionRecord]) -> Vec<String> {
        let mut topic_counts: HashMap<String, usize> = HashMap::new();

        for interaction in interactions {
            for tag in &interaction.context_tags {
                *topic_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        let mut topics: Vec<_> = topic_counts.into_iter().collect();
        topics.sort_by(|a, b| b.1.cmp(&a.1));
        topics.into_iter().take(5).map(|(topic, _)| topic).collect()
    }

    fn extract_executive_priorities(&self, _interactions: &[&InteractionRecord]) -> Vec<String> {
        // Extract executive priorities from conversation context
        // This would be more sophisticated in a real implementation
        vec![]
    }

    fn extract_action_items(&self, _interactions: &[&InteractionRecord]) -> Vec<ActionItem> {
        // Extract action items from conversations
        // This would parse for commitments, assignments, etc.
        vec![]
    }

    fn extract_decisions(&self, _interactions: &[&InteractionRecord]) -> Vec<Decision> {
        // Extract decisions made during conversations
        // This would identify decision points and outcomes
        vec![]
    }

    fn generate_summary_text(&self, interactions: &[&InteractionRecord]) -> String {
        if interactions.is_empty() {
            return "No interactions recorded.".to_string();
        }

        let topics: Vec<String> = interactions
            .iter()
            .flat_map(|i| &i.context_tags)
            .cloned()
            .collect();

        format!(
            "Session with {} interactions covering topics: {}",
            interactions.len(),
            topics.join(", ")
        )
    }
}

impl ExecutiveContext {
    pub fn new() -> Self {
        Self {
            current_priorities: Vec::new(),
            active_projects: Vec::new(),
            key_stakeholders: Vec::new(),
            upcoming_deadlines: Vec::new(),
            recent_decisions: Vec::new(),
            performance_metrics: HashMap::new(),
            market_conditions: MarketContext {
                sector_performance: "Stable".to_string(),
                competitive_landscape: "Competitive".to_string(),
                regulatory_changes: Vec::new(),
                market_sentiment: MarketSentiment::Neutral,
                key_trends: Vec::new(),
            },
            team_status: TeamStatus {
                total_members: 0,
                active_members: 0,
                utilization_rate: 0.0,
                morale_score: None,
                recent_achievements: Vec::new(),
                current_challenges: Vec::new(),
            },
            last_updated: Utc::now(),
        }
    }

    pub async fn update_from_request(&mut self, _request: &NoraRequest) -> crate::Result<()> {
        // Update context based on the incoming request
        self.last_updated = Utc::now();
        Ok(())
    }
}
