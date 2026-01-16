use axum::{
    Json,
    extract::multipart::MultipartError,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use db::models::{
    agent_flow::AgentFlowError,
    agent_flow_event::AgentFlowEventError,
    artifact_review::ArtifactReviewError,
    crm_contact::CrmContactError,
    email_account::EmailAccountError,
    execution_artifact::ExecutionArtifactError,
    execution_process::ExecutionProcessError,
    project::ProjectError,
    social_account::SocialAccountError,
    social_mention::SocialMentionError,
    social_post::SocialPostError,
    task_artifact::TaskArtifactError,
    task_attempt::TaskAttemptError,
    token_usage::TokenUsageError,
    wide_research::WideResearchError,
};
use deployment::DeploymentError;
use executors::executors::ExecutorError;
use git2::Error as Git2Error;
use services::services::{
    auth::AuthError, config::ConfigError, container::ContainerError, git::GitServiceError,
    github_service::GitHubServiceError, image::ImageError, worktree_manager::WorktreeError,
};
use thiserror::Error;
use utils::response::ApiResponse;

#[derive(Debug, Error, ts_rs::TS)]
#[ts(type = "string")]
pub enum ApiError {
    #[error(transparent)]
    Project(#[from] ProjectError),
    #[error(transparent)]
    TaskAttempt(#[from] TaskAttemptError),
    #[error(transparent)]
    ExecutionProcess(#[from] ExecutionProcessError),
    #[error(transparent)]
    GitService(#[from] GitServiceError),
    #[error(transparent)]
    GitHubService(#[from] GitHubServiceError),
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Deployment(#[from] DeploymentError),
    #[error(transparent)]
    Container(#[from] ContainerError),
    #[error(transparent)]
    Executor(#[from] ExecutorError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Worktree(#[from] WorktreeError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Image(#[from] ImageError),
    #[error(transparent)]
    EmailAccount(#[from] EmailAccountError),
    #[error(transparent)]
    CrmContact(#[from] CrmContactError),
    #[error("Multipart error: {0}")]
    Multipart(#[from] MultipartError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Not Found: {0}")]
    NotFound(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
    #[error("Too Many Requests: {0}")]
    TooManyRequests(String),
    #[error("Internal Server Error: {0}")]
    InternalError(String),
    #[error("Payment Required: {0}")]
    PaymentRequired(String),
}

impl From<Git2Error> for ApiError {
    fn from(err: Git2Error) -> Self {
        ApiError::GitService(GitServiceError::from(err))
    }
}

impl From<AgentFlowError> for ApiError {
    fn from(err: AgentFlowError) -> Self {
        match err {
            AgentFlowError::Database(e) => ApiError::Database(e),
            AgentFlowError::NotFound => ApiError::NotFound("Agent flow not found".into()),
            AgentFlowError::InvalidTransition(msg) => ApiError::BadRequest(msg),
        }
    }
}

impl From<AgentFlowEventError> for ApiError {
    fn from(err: AgentFlowEventError) -> Self {
        match err {
            AgentFlowEventError::Database(e) => ApiError::Database(e),
            AgentFlowEventError::NotFound => ApiError::NotFound("Agent flow event not found".into()),
        }
    }
}

impl From<ArtifactReviewError> for ApiError {
    fn from(err: ArtifactReviewError) -> Self {
        match err {
            ArtifactReviewError::Database(e) => ApiError::Database(e),
            ArtifactReviewError::NotFound => ApiError::NotFound("Artifact review not found".into()),
        }
    }
}

impl From<TaskArtifactError> for ApiError {
    fn from(err: TaskArtifactError) -> Self {
        match err {
            TaskArtifactError::Database(e) => ApiError::Database(e),
            TaskArtifactError::NotFound => ApiError::NotFound("Task artifact link not found".into()),
            TaskArtifactError::AlreadyExists => ApiError::Conflict("Artifact already linked to task".into()),
        }
    }
}

impl From<WideResearchError> for ApiError {
    fn from(err: WideResearchError) -> Self {
        match err {
            WideResearchError::Database(e) => ApiError::Database(e),
            WideResearchError::SessionNotFound => ApiError::NotFound("Wide research session not found".into()),
            WideResearchError::SubagentNotFound => ApiError::NotFound("Research subagent not found".into()),
        }
    }
}

impl From<ExecutionArtifactError> for ApiError {
    fn from(err: ExecutionArtifactError) -> Self {
        match err {
            ExecutionArtifactError::Database(e) => ApiError::Database(e),
            ExecutionArtifactError::NotFound => ApiError::NotFound("Execution artifact not found".into()),
            ExecutionArtifactError::InvalidType(msg) => ApiError::BadRequest(msg),
        }
    }
}

impl From<TokenUsageError> for ApiError {
    fn from(err: TokenUsageError) -> Self {
        match err {
            TokenUsageError::Database(e) => ApiError::Database(e),
            TokenUsageError::NotFound => ApiError::NotFound("Token usage record not found".into()),
        }
    }
}

impl From<SocialAccountError> for ApiError {
    fn from(err: SocialAccountError) -> Self {
        match err {
            SocialAccountError::Database(e) => ApiError::Database(e),
            SocialAccountError::NotFound => ApiError::NotFound("Social account not found".into()),
            SocialAccountError::AlreadyExists => ApiError::Conflict("Social account already exists".into()),
        }
    }
}

impl From<SocialPostError> for ApiError {
    fn from(err: SocialPostError) -> Self {
        match err {
            SocialPostError::Database(e) => ApiError::Database(e),
            SocialPostError::NotFound => ApiError::NotFound("Social post not found".into()),
        }
    }
}

impl From<SocialMentionError> for ApiError {
    fn from(err: SocialMentionError) -> Self {
        match err {
            SocialMentionError::Database(e) => ApiError::Database(e),
            SocialMentionError::NotFound => ApiError::NotFound("Social mention not found".into()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status_code, error_type) = match &self {
            ApiError::Project(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ProjectError"),
            ApiError::TaskAttempt(_) => (StatusCode::INTERNAL_SERVER_ERROR, "TaskAttemptError"),
            ApiError::ExecutionProcess(err) => match err {
                ExecutionProcessError::ExecutionProcessNotFound => {
                    (StatusCode::NOT_FOUND, "ExecutionProcessError")
                }
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "ExecutionProcessError"),
            },
            // Promote certain GitService errors to conflict status with concise messages
            ApiError::GitService(git_err) => match git_err {
                services::services::git::GitServiceError::MergeConflicts(_) => {
                    (StatusCode::CONFLICT, "GitServiceError")
                }
                services::services::git::GitServiceError::RebaseInProgress => {
                    (StatusCode::CONFLICT, "GitServiceError")
                }
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "GitServiceError"),
            },
            ApiError::GitHubService(_) => (StatusCode::INTERNAL_SERVER_ERROR, "GitHubServiceError"),
            ApiError::Auth(_) => (StatusCode::INTERNAL_SERVER_ERROR, "AuthError"),
            ApiError::Deployment(_) => (StatusCode::INTERNAL_SERVER_ERROR, "DeploymentError"),
            ApiError::Container(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ContainerError"),
            ApiError::Executor(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ExecutorError"),
            ApiError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "DatabaseError"),
            ApiError::Worktree(_) => (StatusCode::INTERNAL_SERVER_ERROR, "WorktreeError"),
            ApiError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, "ConfigError"),
            ApiError::Image(img_err) => match img_err {
                ImageError::InvalidFormat => (StatusCode::BAD_REQUEST, "InvalidImageFormat"),
                ImageError::TooLarge(_, _) => (StatusCode::PAYLOAD_TOO_LARGE, "ImageTooLarge"),
                ImageError::NotFound => (StatusCode::NOT_FOUND, "ImageNotFound"),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "ImageError"),
            },
            ApiError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "IoError"),
            ApiError::Multipart(_) => (StatusCode::BAD_REQUEST, "MultipartError"),
            ApiError::Conflict(_) => (StatusCode::CONFLICT, "ConflictError"),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BadRequest"),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, "NotFound"),
            ApiError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            ApiError::Forbidden(_) => (StatusCode::FORBIDDEN, "Forbidden"),
            ApiError::TooManyRequests(_) => (StatusCode::TOO_MANY_REQUESTS, "TooManyRequests"),
            ApiError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "InternalError"),
            ApiError::PaymentRequired(_) => (StatusCode::PAYMENT_REQUIRED, "PaymentRequired"),
            ApiError::EmailAccount(e) => match e {
                EmailAccountError::NotFound => (StatusCode::NOT_FOUND, "EmailAccountNotFound"),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "EmailAccountError"),
            },
            ApiError::CrmContact(e) => match e {
                CrmContactError::NotFound => (StatusCode::NOT_FOUND, "CrmContactNotFound"),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "CrmContactError"),
            },
        };

        let error_message = match &self {
            ApiError::Image(img_err) => match img_err {
                ImageError::InvalidFormat => "This file type is not supported. Please upload an image file (PNG, JPG, GIF, WebP, or BMP).".to_string(),
                ImageError::TooLarge(size, max) => format!(
                    "This image is too large ({:.1} MB). Maximum file size is {:.1} MB.",
                    *size as f64 / 1_048_576.0,
                    *max as f64 / 1_048_576.0
                ),
                ImageError::NotFound => "Image not found.".to_string(),
                _ => {
                    "Failed to process image. Please try again.".to_string()
                }
            },
            ApiError::GitService(git_err) => match git_err {
                services::services::git::GitServiceError::MergeConflicts(msg) => msg.clone(),
                services::services::git::GitServiceError::RebaseInProgress => {
                    "A rebase is already in progress. Resolve conflicts or abort the rebase, then retry.".to_string()
                }
                _ => format!("{}: {}", error_type, self),
            },
            ApiError::Multipart(_) => "Failed to upload file. Please ensure the file is valid and try again.".to_string(),
            ApiError::Conflict(msg) => msg.clone(),
            ApiError::BadRequest(msg) => msg.clone(),
            ApiError::NotFound(msg) => msg.clone(),
            ApiError::Unauthorized(msg) => msg.clone(),
            ApiError::Forbidden(msg) => msg.clone(),
            ApiError::TooManyRequests(msg) => msg.clone(),
            ApiError::InternalError(msg) => msg.clone(),
            ApiError::PaymentRequired(msg) => msg.clone(),
            _ => format!("{}: {}", error_type, self),
        };
        let response = ApiResponse::<()>::error(&error_message);
        (status_code, Json(response)).into_response()
    }
}
