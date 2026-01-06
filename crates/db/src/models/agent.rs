use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Agent status in the platform
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "agent_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Active,
    Inactive,
    Maintenance,
    Training,
}

impl Default for AgentStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Level of autonomy the agent has
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "autonomy_level", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevel {
    Full,
    Supervised,
    ApprovalRequired,
    Manual,
}

impl Default for AutonomyLevel {
    fn default() -> Self {
        Self::Supervised
    }
}

/// Proficiency level for a capability
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "proficiency_level", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ProficiencyLevel {
    Novice,
    Standard,
    Expert,
    Master,
}

impl Default for ProficiencyLevel {
    fn default() -> Self {
        Self::Standard
    }
}

/// Agent personality traits
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentPersonality {
    /// Core personality traits (e.g., "analytical", "creative", "methodical")
    pub traits: Vec<String>,
    /// Communication style (e.g., "formal", "casual", "technical")
    pub communication_style: String,
    /// How the agent approaches problems
    pub problem_solving_approach: String,
    /// Preferred interaction patterns
    pub interaction_preferences: Vec<String>,
    /// Agent's "backstory" or character description
    pub backstory: Option<String>,
    /// Catchphrases or signature expressions
    pub signature_phrases: Vec<String>,
    /// Emotional baseline (e.g., "calm", "enthusiastic", "focused")
    pub emotional_baseline: String,
}

/// Agent function definition
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub required_tools: Vec<String>,
    pub example_usage: Option<String>,
}

/// Full Agent entity
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Agent {
    pub id: Uuid,
    pub wallet_address: Option<String>,
    pub short_name: String,
    pub designation: String,
    pub description: Option<String>,

    // Personality & Character (stored as JSON strings)
    pub personality: Option<String>,
    pub voice_style: Option<String>,
    pub avatar_url: Option<String>,

    // Capabilities & Tools (stored as JSON strings)
    pub capabilities: Option<String>,
    pub tools: Option<String>,
    pub functions: Option<String>,

    // Model Configuration
    pub default_model: Option<String>,
    pub fallback_models: Option<String>,
    pub model_config: Option<String>,

    // Operational Settings
    pub status: AgentStatus,
    pub autonomy_level: AutonomyLevel,
    pub max_concurrent_tasks: Option<i64>,
    pub priority_weight: Option<i64>,

    // Statistics
    pub tasks_completed: Option<i64>,
    pub tasks_failed: Option<i64>,
    pub total_execution_time_ms: Option<i64>,
    pub average_rating: Option<f64>,

    // Metadata
    pub version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,

    // Relationships
    pub parent_agent_id: Option<Uuid>,
    pub team_id: Option<String>,
}

/// Agent with parsed JSON fields for API responses
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentWithParsedFields {
    pub id: Uuid,
    pub wallet_address: Option<String>,
    pub short_name: String,
    pub designation: String,
    pub description: Option<String>,
    pub personality: Option<AgentPersonality>,
    pub voice_style: Option<String>,
    pub avatar_url: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub tools: Option<Vec<String>>,
    pub functions: Option<Vec<AgentFunction>>,
    pub default_model: Option<String>,
    pub fallback_models: Option<Vec<String>>,
    pub model_config: Option<Value>,
    pub status: AgentStatus,
    pub autonomy_level: AutonomyLevel,
    pub max_concurrent_tasks: Option<i64>,
    pub priority_weight: Option<i64>,
    pub tasks_completed: Option<i64>,
    pub tasks_failed: Option<i64>,
    pub average_rating: Option<f64>,
    pub version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Agent> for AgentWithParsedFields {
    fn from(agent: Agent) -> Self {
        Self {
            id: agent.id,
            wallet_address: agent.wallet_address,
            short_name: agent.short_name,
            designation: agent.designation,
            description: agent.description,
            personality: agent.personality.as_deref().and_then(|s| serde_json::from_str(s).ok()),
            voice_style: agent.voice_style,
            avatar_url: agent.avatar_url,
            capabilities: agent.capabilities.as_deref().and_then(|s| serde_json::from_str(s).ok()),
            tools: agent.tools.as_deref().and_then(|s| serde_json::from_str(s).ok()),
            functions: agent.functions.as_deref().and_then(|s| serde_json::from_str(s).ok()),
            default_model: agent.default_model,
            fallback_models: agent.fallback_models.as_deref().and_then(|s| serde_json::from_str(s).ok()),
            model_config: agent.model_config.as_deref().and_then(|s| serde_json::from_str(s).ok()),
            status: agent.status,
            autonomy_level: agent.autonomy_level,
            max_concurrent_tasks: agent.max_concurrent_tasks,
            priority_weight: agent.priority_weight,
            tasks_completed: agent.tasks_completed,
            tasks_failed: agent.tasks_failed,
            average_rating: agent.average_rating,
            version: agent.version,
            created_at: agent.created_at,
            updated_at: agent.updated_at,
        }
    }
}

/// Create a new agent
#[derive(Debug, Deserialize, TS)]
pub struct CreateAgent {
    pub wallet_address: Option<String>,
    pub short_name: String,
    pub designation: String,
    pub description: Option<String>,
    pub personality: Option<AgentPersonality>,
    pub voice_style: Option<String>,
    pub avatar_url: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub tools: Option<Vec<String>>,
    pub functions: Option<Vec<AgentFunction>>,
    pub default_model: Option<String>,
    pub fallback_models: Option<Vec<String>>,
    pub model_config: Option<Value>,
    pub status: Option<AgentStatus>,
    pub autonomy_level: Option<AutonomyLevel>,
    pub max_concurrent_tasks: Option<i64>,
    pub priority_weight: Option<i64>,
    pub parent_agent_id: Option<Uuid>,
    pub team_id: Option<String>,
    pub created_by: Option<String>,
}

/// Update an existing agent
#[derive(Debug, Deserialize, TS)]
pub struct UpdateAgent {
    pub wallet_address: Option<String>,
    pub short_name: Option<String>,
    pub designation: Option<String>,
    pub description: Option<String>,
    pub personality: Option<AgentPersonality>,
    pub voice_style: Option<String>,
    pub avatar_url: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub tools: Option<Vec<String>>,
    pub functions: Option<Vec<AgentFunction>>,
    pub default_model: Option<String>,
    pub fallback_models: Option<Vec<String>>,
    pub model_config: Option<Value>,
    pub status: Option<AgentStatus>,
    pub autonomy_level: Option<AutonomyLevel>,
    pub max_concurrent_tasks: Option<i64>,
    pub priority_weight: Option<i64>,
}

/// Brief agent info for lists
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentBrief {
    pub id: Uuid,
    pub short_name: String,
    pub designation: String,
    pub avatar_url: Option<String>,
    pub status: AgentStatus,
}

impl Agent {
    /// Find all agents
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Agent,
            r#"SELECT
                id as "id!: Uuid",
                wallet_address,
                short_name,
                designation,
                description,
                personality,
                voice_style,
                avatar_url,
                capabilities,
                tools,
                functions,
                default_model,
                fallback_models,
                model_config,
                status as "status!: AgentStatus",
                autonomy_level as "autonomy_level!: AutonomyLevel",
                max_concurrent_tasks,
                priority_weight,
                tasks_completed,
                tasks_failed,
                total_execution_time_ms,
                average_rating,
                version,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                created_by,
                parent_agent_id as "parent_agent_id: Uuid",
                team_id
            FROM agents
            ORDER BY short_name ASC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find active agents only
    pub async fn find_active(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Agent,
            r#"SELECT
                id as "id!: Uuid",
                wallet_address,
                short_name,
                designation,
                description,
                personality,
                voice_style,
                avatar_url,
                capabilities,
                tools,
                functions,
                default_model,
                fallback_models,
                model_config,
                status as "status!: AgentStatus",
                autonomy_level as "autonomy_level!: AutonomyLevel",
                max_concurrent_tasks,
                priority_weight,
                tasks_completed,
                tasks_failed,
                total_execution_time_ms,
                average_rating,
                version,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                created_by,
                parent_agent_id as "parent_agent_id: Uuid",
                team_id
            FROM agents
            WHERE status = 'active'
            ORDER BY priority_weight DESC, short_name ASC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find agent by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Agent,
            r#"SELECT
                id as "id!: Uuid",
                wallet_address,
                short_name,
                designation,
                description,
                personality,
                voice_style,
                avatar_url,
                capabilities,
                tools,
                functions,
                default_model,
                fallback_models,
                model_config,
                status as "status!: AgentStatus",
                autonomy_level as "autonomy_level!: AutonomyLevel",
                max_concurrent_tasks,
                priority_weight,
                tasks_completed,
                tasks_failed,
                total_execution_time_ms,
                average_rating,
                version,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                created_by,
                parent_agent_id as "parent_agent_id: Uuid",
                team_id
            FROM agents
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find agent by short name
    pub async fn find_by_short_name(pool: &SqlitePool, short_name: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Agent,
            r#"SELECT
                id as "id!: Uuid",
                wallet_address,
                short_name,
                designation,
                description,
                personality,
                voice_style,
                avatar_url,
                capabilities,
                tools,
                functions,
                default_model,
                fallback_models,
                model_config,
                status as "status!: AgentStatus",
                autonomy_level as "autonomy_level!: AutonomyLevel",
                max_concurrent_tasks,
                priority_weight,
                tasks_completed,
                tasks_failed,
                total_execution_time_ms,
                average_rating,
                version,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                created_by,
                parent_agent_id as "parent_agent_id: Uuid",
                team_id
            FROM agents
            WHERE LOWER(short_name) = LOWER($1)"#,
            short_name
        )
        .fetch_optional(pool)
        .await
    }

    /// Find agent by wallet address
    pub async fn find_by_wallet(pool: &SqlitePool, wallet_address: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Agent,
            r#"SELECT
                id as "id!: Uuid",
                wallet_address,
                short_name,
                designation,
                description,
                personality,
                voice_style,
                avatar_url,
                capabilities,
                tools,
                functions,
                default_model,
                fallback_models,
                model_config,
                status as "status!: AgentStatus",
                autonomy_level as "autonomy_level!: AutonomyLevel",
                max_concurrent_tasks,
                priority_weight,
                tasks_completed,
                tasks_failed,
                total_execution_time_ms,
                average_rating,
                version,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                created_by,
                parent_agent_id as "parent_agent_id: Uuid",
                team_id
            FROM agents
            WHERE wallet_address = $1"#,
            wallet_address
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new agent
    pub async fn create(pool: &SqlitePool, data: &CreateAgent) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let personality_json = data.personality.as_ref().map(|p| serde_json::to_string(p).unwrap());
        let capabilities_json = data.capabilities.as_ref().map(|c| serde_json::to_string(c).unwrap());
        let tools_json = data.tools.as_ref().map(|t| serde_json::to_string(t).unwrap());
        let functions_json = data.functions.as_ref().map(|f| serde_json::to_string(f).unwrap());
        let fallback_models_json = data.fallback_models.as_ref().map(|f| serde_json::to_string(f).unwrap());
        let model_config_json = data.model_config.as_ref().map(|m| serde_json::to_string(m).unwrap());
        let status = data.status.clone().unwrap_or_default();
        let autonomy_level = data.autonomy_level.clone().unwrap_or_default();

        sqlx::query_as!(
            Agent,
            r#"INSERT INTO agents (
                id, wallet_address, short_name, designation, description,
                personality, voice_style, avatar_url,
                capabilities, tools, functions,
                default_model, fallback_models, model_config,
                status, autonomy_level, max_concurrent_tasks, priority_weight,
                parent_agent_id, team_id, created_by
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8,
                $9, $10, $11,
                $12, $13, $14,
                $15, $16, $17, $18,
                $19, $20, $21
            )
            RETURNING
                id as "id!: Uuid",
                wallet_address,
                short_name,
                designation,
                description,
                personality,
                voice_style,
                avatar_url,
                capabilities,
                tools,
                functions,
                default_model,
                fallback_models,
                model_config,
                status as "status!: AgentStatus",
                autonomy_level as "autonomy_level!: AutonomyLevel",
                max_concurrent_tasks,
                priority_weight,
                tasks_completed,
                tasks_failed,
                total_execution_time_ms,
                average_rating,
                version,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                created_by,
                parent_agent_id as "parent_agent_id: Uuid",
                team_id"#,
            id,
            data.wallet_address,
            data.short_name,
            data.designation,
            data.description,
            personality_json,
            data.voice_style,
            data.avatar_url,
            capabilities_json,
            tools_json,
            functions_json,
            data.default_model,
            fallback_models_json,
            model_config_json,
            status,
            autonomy_level,
            data.max_concurrent_tasks,
            data.priority_weight,
            data.parent_agent_id,
            data.team_id,
            data.created_by
        )
        .fetch_one(pool)
        .await
    }

    /// Update an existing agent
    pub async fn update(pool: &SqlitePool, id: Uuid, data: &UpdateAgent) -> Result<Self, sqlx::Error> {
        let personality_json = data.personality.as_ref().map(|p| serde_json::to_string(p).unwrap());
        let capabilities_json = data.capabilities.as_ref().map(|c| serde_json::to_string(c).unwrap());
        let tools_json = data.tools.as_ref().map(|t| serde_json::to_string(t).unwrap());
        let functions_json = data.functions.as_ref().map(|f| serde_json::to_string(f).unwrap());
        let fallback_models_json = data.fallback_models.as_ref().map(|f| serde_json::to_string(f).unwrap());
        let model_config_json = data.model_config.as_ref().map(|m| serde_json::to_string(m).unwrap());

        sqlx::query_as!(
            Agent,
            r#"UPDATE agents SET
                wallet_address = COALESCE($2, wallet_address),
                short_name = COALESCE($3, short_name),
                designation = COALESCE($4, designation),
                description = COALESCE($5, description),
                personality = COALESCE($6, personality),
                voice_style = COALESCE($7, voice_style),
                avatar_url = COALESCE($8, avatar_url),
                capabilities = COALESCE($9, capabilities),
                tools = COALESCE($10, tools),
                functions = COALESCE($11, functions),
                default_model = COALESCE($12, default_model),
                fallback_models = COALESCE($13, fallback_models),
                model_config = COALESCE($14, model_config),
                status = COALESCE($15, status),
                autonomy_level = COALESCE($16, autonomy_level),
                max_concurrent_tasks = COALESCE($17, max_concurrent_tasks),
                priority_weight = COALESCE($18, priority_weight),
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                wallet_address,
                short_name,
                designation,
                description,
                personality,
                voice_style,
                avatar_url,
                capabilities,
                tools,
                functions,
                default_model,
                fallback_models,
                model_config,
                status as "status!: AgentStatus",
                autonomy_level as "autonomy_level!: AutonomyLevel",
                max_concurrent_tasks,
                priority_weight,
                tasks_completed,
                tasks_failed,
                total_execution_time_ms,
                average_rating,
                version,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                created_by,
                parent_agent_id as "parent_agent_id: Uuid",
                team_id"#,
            id,
            data.wallet_address,
            data.short_name,
            data.designation,
            data.description,
            personality_json,
            data.voice_style,
            data.avatar_url,
            capabilities_json,
            tools_json,
            functions_json,
            data.default_model,
            fallback_models_json,
            model_config_json,
            data.status,
            data.autonomy_level,
            data.max_concurrent_tasks,
            data.priority_weight
        )
        .fetch_one(pool)
        .await
    }

    /// Delete an agent
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM agents WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Update agent statistics after task completion
    pub async fn record_task_completion(
        pool: &SqlitePool,
        id: Uuid,
        success: bool,
        execution_time_ms: i64,
        rating: Option<f64>,
    ) -> Result<(), sqlx::Error> {
        if success {
            sqlx::query!(
                r#"UPDATE agents SET
                    tasks_completed = COALESCE(tasks_completed, 0) + 1,
                    total_execution_time_ms = COALESCE(total_execution_time_ms, 0) + $2,
                    average_rating = CASE
                        WHEN $3 IS NOT NULL THEN
                            (COALESCE(average_rating, 0) * COALESCE(tasks_completed, 0) + $3) / (COALESCE(tasks_completed, 0) + 1)
                        ELSE average_rating
                    END,
                    updated_at = datetime('now', 'subsec')
                WHERE id = $1"#,
                id,
                execution_time_ms,
                rating
            )
            .execute(pool)
            .await?;
        } else {
            sqlx::query!(
                r#"UPDATE agents SET
                    tasks_failed = COALESCE(tasks_failed, 0) + 1,
                    total_execution_time_ms = COALESCE(total_execution_time_ms, 0) + $2,
                    updated_at = datetime('now', 'subsec')
                WHERE id = $1"#,
                id,
                execution_time_ms
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    /// Update agent status
    pub async fn update_status(pool: &SqlitePool, id: Uuid, status: AgentStatus) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE agents SET status = $2, updated_at = datetime('now', 'subsec') WHERE id = $1",
            id,
            status
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
