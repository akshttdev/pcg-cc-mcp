use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use chrono::{DateTime, Utc};
use db::models::merge::{MergeStatus, PullRequestInfo};
use octocrab::{models::IssueState, Octocrab, OctocrabBuilder};
use regex::Regex;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;
use ts_rs::TS;

use crate::services::{git::GitServiceError, git_cli::GitCliError};

#[derive(Debug, Error, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(use_ts_enum)]
pub enum GitHubServiceError {
    #[ts(skip)]
    #[serde(skip)]
    #[error(transparent)]
    Client(octocrab::Error),
    #[ts(skip)]
    #[error("Repository error: {0}")]
    Repository(String),
    #[ts(skip)]
    #[error("Pull request error: {0}")]
    PullRequest(String),
    #[ts(skip)]
    #[error("Branch error: {0}")]
    Branch(String),
    #[error("GitHub token is invalid or expired.")]
    TokenInvalid,
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    #[error("GitHub repository not found or no access")]
    RepoNotFoundOrNoAccess,
    #[ts(skip)]
    #[serde(skip)]
    #[error(transparent)]
    GitService(GitServiceError),
}

impl From<octocrab::Error> for GitHubServiceError {
    fn from(err: octocrab::Error) -> Self {
        match &err {
            octocrab::Error::GitHub { source, .. } => {
                let status = source.status_code.as_u16();
                let msg = source.message.to_ascii_lowercase();
                if status == 401 || msg.contains("bad credentials") || msg.contains("token expired")
                {
                    GitHubServiceError::TokenInvalid
                } else if status == 403 {
                    GitHubServiceError::InsufficientPermissions
                } else {
                    GitHubServiceError::Client(err)
                }
            }
            _ => GitHubServiceError::Client(err),
        }
    }
}
impl From<GitServiceError> for GitHubServiceError {
    fn from(error: GitServiceError) -> Self {
        match error {
            GitServiceError::GitCLI(GitCliError::AuthFailed(_)) => Self::TokenInvalid,
            GitServiceError::GitCLI(GitCliError::CommandFailed(msg)) => {
                let lower = msg.to_ascii_lowercase();
                if lower.contains("the requested url returned error: 403") {
                    Self::InsufficientPermissions
                } else if lower.contains("the requested url returned error: 404") {
                    Self::RepoNotFoundOrNoAccess
                } else {
                    Self::GitService(GitServiceError::GitCLI(GitCliError::CommandFailed(msg)))
                }
            }
            other => Self::GitService(other),
        }
    }
}

impl GitHubServiceError {
    pub fn is_api_data(&self) -> bool {
        matches!(
            self,
            GitHubServiceError::TokenInvalid
                | GitHubServiceError::InsufficientPermissions
                | GitHubServiceError::RepoNotFoundOrNoAccess
        )
    }

    pub fn should_retry(&self) -> bool {
        !self.is_api_data()
    }
}

#[derive(Debug, Clone)]
pub struct GitHubRepoInfo {
    pub owner: String,
    pub repo_name: String,
}
impl GitHubRepoInfo {
    pub fn from_remote_url(remote_url: &str) -> Result<Self, GitHubServiceError> {
        // Supports SSH, HTTPS and PR GitHub URLs. See tests for examples.
        let re = Regex::new(r"github\.com[:/](?P<owner>[^/]+)/(?P<repo>[^/]+?)(?:\.git)?(?:/|$)")
            .map_err(|e| {
            GitHubServiceError::Repository(format!("Failed to compile regex: {e}"))
        })?;

        let caps = re.captures(remote_url).ok_or_else(|| {
            GitHubServiceError::Repository(format!("Invalid GitHub URL format: {remote_url}"))
        })?;

        Ok(Self {
            owner: caps.name("owner").unwrap().as_str().to_string(),
            repo_name: caps.name("repo").unwrap().as_str().to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct CreatePrRequest {
    pub title: String,
    pub body: Option<String>,
    pub head_branch: String,
    pub base_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct RepositoryInfo {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub owner: String,
    pub description: Option<String>,
    pub clone_url: String,
    pub ssh_url: String,
    pub default_branch: String,
    pub private: bool,
}

#[derive(Debug, Clone)]
pub struct GitHubService {
    client: Octocrab,
}

impl GitHubService {
    /// Create a new GitHub service with authentication
    pub fn new(github_token: &str) -> Result<Self, GitHubServiceError> {
        let client = OctocrabBuilder::new()
            .personal_token(github_token.to_string())
            .build()?;

        Ok(Self { client })
    }

    pub async fn check_token(&self) -> Result<(), GitHubServiceError> {
        self.client.current().user().await?;
        Ok(())
    }

    /// Create a pull request on GitHub
    pub async fn create_pr(
        &self,
        repo_info: &GitHubRepoInfo,
        request: &CreatePrRequest,
    ) -> Result<PullRequestInfo, GitHubServiceError> {
        (|| async { self.create_pr_internal(repo_info, request).await })
            .retry(
                &ExponentialBuilder::default()
                    .with_min_delay(Duration::from_secs(1))
                    .with_max_delay(Duration::from_secs(30))
                    .with_max_times(3)
                    .with_jitter(),
            )
            .when(|e| e.should_retry())
            .notify(|err: &GitHubServiceError, dur: Duration| {
                tracing::warn!(
                    "GitHub API call failed, retrying after {:.2}s: {}",
                    dur.as_secs_f64(),
                    err
                );
            })
            .await
    }

    async fn create_pr_internal(
        &self,
        repo_info: &GitHubRepoInfo,
        request: &CreatePrRequest,
    ) -> Result<PullRequestInfo, GitHubServiceError> {
        // Verify repository access
        self.client
            .repos(&repo_info.owner, &repo_info.repo_name)
            .get()
            .await
            .map_err(|error| match GitHubServiceError::from(error) {
                GitHubServiceError::Client(source) => GitHubServiceError::Repository(format!(
                    "Cannot access repository {}/{}: {}",
                    repo_info.owner, repo_info.repo_name, source
                )),
                other => other,
            })?;

        // Check if the base branch exists
        self.client
            .repos(&repo_info.owner, &repo_info.repo_name)
            .get_ref(&octocrab::params::repos::Reference::Branch(
                request.base_branch.to_string(),
            ))
            .await
            .map_err(|err| match GitHubServiceError::from(err) {
                GitHubServiceError::Client(source) => {
                    let hint = if request.base_branch != "main" {
                        " Perhaps you meant to use main as your base branch instead?"
                    } else {
                        ""
                    };
                    GitHubServiceError::Branch(format!(
                        "Base branch '{}' does not exist: {}{}",
                        request.base_branch, source, hint
                    ))
                }
                other => other,
            })?;

        // Check if the head branch exists
        self.client
            .repos(&repo_info.owner, &repo_info.repo_name)
            .get_ref(&octocrab::params::repos::Reference::Branch(
                request.head_branch.to_string(),
            ))
            .await
            .map_err(|err| match GitHubServiceError::from(err) {
                GitHubServiceError::Client(source) => GitHubServiceError::Branch(format!(
                    "Head branch '{}' does not exist: {}",
                    request.head_branch, source
                )),
                other => other,
            })?;

        // Create the pull request
        let pr_info = self
            .client
            .pulls(&repo_info.owner, &repo_info.repo_name)
            .create(&request.title, &request.head_branch, &request.base_branch)
            .body(request.body.as_deref().unwrap_or(""))
            .send()
            .await
            .map(Self::map_pull_request)
            .map_err(|err| match GitHubServiceError::from(err) {
                GitHubServiceError::Client(source) => GitHubServiceError::PullRequest(format!(
                    "Failed to create PR for '{} -> {}': {}",
                    request.head_branch, request.base_branch, source
                )),
                other => other,
            })?;

        info!(
            "Created GitHub PR #{} for branch {} in {}/{}",
            pr_info.number, request.head_branch, repo_info.owner, repo_info.repo_name
        );

        Ok(pr_info)
    }

    /// Add a comment to a pull request
    pub async fn add_pr_comment(
        &self,
        repo_info: &GitHubRepoInfo,
        pr_number: i64,
        body: &str,
    ) -> Result<(), GitHubServiceError> {
        self.client
            .issues(&repo_info.owner, &repo_info.repo_name)
            .create_comment(u64::try_from(pr_number).unwrap_or(0), body)
            .await
            .map_err(|err| match GitHubServiceError::from(err) {
                GitHubServiceError::Client(source) => GitHubServiceError::PullRequest(format!(
                    "Failed to comment on PR #{pr_number}: {source}"
                )),
                other => other,
            })?;
        Ok(())
    }

    /// Update and get the status of a pull request
    pub async fn update_pr_status(
        &self,
        repo_info: &GitHubRepoInfo,
        pr_number: i64,
    ) -> Result<PullRequestInfo, GitHubServiceError> {
        (|| async {
            self.client
                .pulls(&repo_info.owner, &repo_info.repo_name)
                .get(u64::try_from(pr_number).unwrap_or(0))
                .await
                .map(Self::map_pull_request)
                .map_err(|err| match GitHubServiceError::from(err) {
                    GitHubServiceError::Client(source) => GitHubServiceError::PullRequest(format!(
                        "Failed to get PR #{pr_number}: {source}",
                    )),
                    other => other,
                })
        })
        .retry(
            &ExponentialBuilder::default()
                .with_min_delay(Duration::from_secs(1))
                .with_max_delay(Duration::from_secs(30))
                .with_max_times(3)
                .with_jitter(),
        )
        .when(|err| err.should_retry())
        .notify(|err: &GitHubServiceError, dur: Duration| {
            tracing::warn!(
                "GitHub API call failed, retrying after {:.2}s: {}",
                dur.as_secs_f64(),
                err
            );
        })
        .await
    }

    fn map_pull_request(pr: octocrab::models::pulls::PullRequest) -> PullRequestInfo {
        let state = match pr.state {
            Some(IssueState::Open) => MergeStatus::Open,
            Some(IssueState::Closed) => {
                if pr.merged_at.is_some() {
                    MergeStatus::Merged
                } else {
                    MergeStatus::Closed
                }
            }
            None => MergeStatus::Unknown,
            Some(_) => MergeStatus::Unknown,
        };

        PullRequestInfo {
            number: pr.number as i64,
            url: pr.html_url.map(|url| url.to_string()).unwrap_or_default(),
            status: state,
            merged_at: pr.merged_at.map(|dt| dt.naive_utc().and_utc()),
            merge_commit_sha: pr.merge_commit_sha,
        }
    }

    /// List repositories for the authenticated user with pagination
    pub async fn list_repositories(
        &self,
        page: u8,
    ) -> Result<Vec<RepositoryInfo>, GitHubServiceError> {
        (|| async { self.list_repositories_internal(page).await })
            .retry(
                &ExponentialBuilder::default()
                    .with_min_delay(Duration::from_secs(1))
                    .with_max_delay(Duration::from_secs(30))
                    .with_max_times(3)
                    .with_jitter(),
            )
            .when(|err| err.should_retry())
            .notify(|err: &GitHubServiceError, dur: Duration| {
                tracing::warn!(
                    "GitHub API call failed, retrying after {:.2}s: {}",
                    dur.as_secs_f64(),
                    err
                );
            })
            .await
    }

    async fn list_repositories_internal(
        &self,
        page: u8,
    ) -> Result<Vec<RepositoryInfo>, GitHubServiceError> {
        let repos_page = self
            .client
            .current()
            .list_repos_for_authenticated_user()
            .type_("all")
            .sort("updated")
            .direction("desc")
            .per_page(50)
            .page(page)
            .send()
            .await
            .map_err(|e| {
                GitHubServiceError::Repository(format!("Failed to list repositories: {}", e))
            })?;

        let repositories: Vec<RepositoryInfo> = repos_page
            .items
            .into_iter()
            .map(|repo| RepositoryInfo {
                id: repo.id.0 as i64,
                name: repo.name,
                full_name: repo.full_name.unwrap_or_default(),
                owner: repo.owner.map(|o| o.login).unwrap_or_default(),
                description: repo.description,
                clone_url: repo
                    .clone_url
                    .map(|url| url.to_string())
                    .unwrap_or_default(),
                ssh_url: repo.ssh_url.unwrap_or_default(),
                default_branch: repo.default_branch.unwrap_or_else(|| "main".to_string()),
                private: repo.private.unwrap_or(false),
            })
            .collect();

        tracing::info!(
            "Retrieved {} repositories from GitHub (page {})",
            repositories.len(),
            page
        );
        Ok(repositories)
    }

    /// Identity of the user who owns the access token. Used after OAuth
    /// callback to populate `provider_account_id` on the integration connection.
    pub async fn get_authenticated_user(&self) -> Result<AuthenticatedUser, GitHubServiceError> {
        let me = self.client.current().user().await?;
        Ok(AuthenticatedUser {
            id: me.id.0 as i64,
            login: me.login,
            name: me.name,
            avatar_url: Some(me.avatar_url.to_string()),
            email: me.email,
        })
    }

    /// Commits on `branch` (or default branch when None), newest first.
    /// `since` is an optional RFC3339 cutoff so callers can resume from their
    /// last sync cursor.
    pub async fn list_commits(
        &self,
        owner: &str,
        repo: &str,
        branch: Option<&str>,
        since: Option<DateTime<Utc>>,
        page: u8,
    ) -> Result<Vec<CommitSummary>, GitHubServiceError> {
        // Hold the RepoHandler in a binding so the borrow outlives the
        // chained builder calls below.
        let handler = self.client.repos(owner, repo);
        let mut builder = handler.list_commits();
        if let Some(b) = branch {
            builder = builder.branch(b);
        }
        if let Some(s) = since {
            builder = builder.since(s);
        }
        let commits_page = builder.per_page(50).page(page).send().await.map_err(|e| {
            GitHubServiceError::Repository(format!(
                "Failed to list commits for {owner}/{repo}: {e}"
            ))
        })?;

        let commits = commits_page
            .items
            .into_iter()
            .map(|c| CommitSummary {
                sha: c.sha,
                html_url: c.html_url.to_string(),
                message: c.commit.message,
                author_name: c.commit.author.as_ref().map(|a| a.name.clone()),
                author_email: c.commit.author.as_ref().map(|a| a.email.clone()),
                author_date: c.commit.author.and_then(|a| a.date),
                committer_login: c.author.map(|a| a.login),
            })
            .collect();
        Ok(commits)
    }
}

// ─── OAuth helpers (no token required) ─────────────────────────────────────

/// User-facing GitHub OAuth scopes. Override via `GITHUB_OAUTH_SCOPES`.
const DEFAULT_OAUTH_SCOPES: &str = "read:user user:email repo";

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AuthenticatedUser {
    pub id: i64,
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CommitSummary {
    pub sha: String,
    pub html_url: String,
    pub message: String,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    #[ts(type = "Date | null", optional)]
    pub author_date: Option<DateTime<Utc>>,
    pub committer_login: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    scope: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

fn oauth_scopes() -> String {
    std::env::var("GITHUB_OAUTH_SCOPES").unwrap_or_else(|_| DEFAULT_OAUTH_SCOPES.to_string())
}

/// Build the GitHub OAuth authorization URL.
///
/// Reads `GITHUB_CLIENT_ID` from the env. Callers must register the
/// `redirect_uri` on their GitHub OAuth app.
pub fn oauth_authorize_url(redirect_uri: &str, state: &str) -> Result<String, GitHubServiceError> {
    let client_id = std::env::var("GITHUB_CLIENT_ID").map_err(|_| {
        GitHubServiceError::Repository("GITHUB_CLIENT_ID environment variable not set".to_string())
    })?;
    let scopes = oauth_scopes();
    Ok(format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}&allow_signup=true",
        urlencoding::encode(&client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(&scopes),
        urlencoding::encode(state),
    ))
}

/// Exchange an authorization code for an access token.
///
/// Reads `GITHUB_CLIENT_ID` / `GITHUB_CLIENT_SECRET` from the env. The
/// returned tokens are *plaintext* — callers must encrypt before persisting.
pub async fn oauth_exchange_code(
    code: &str,
    redirect_uri: &str,
) -> Result<OAuthTokens, GitHubServiceError> {
    let client_id = std::env::var("GITHUB_CLIENT_ID").map_err(|_| {
        GitHubServiceError::Repository("GITHUB_CLIENT_ID environment variable not set".to_string())
    })?;
    let client_secret = std::env::var("GITHUB_CLIENT_SECRET").map_err(|_| {
        GitHubServiceError::Repository(
            "GITHUB_CLIENT_SECRET environment variable not set".to_string(),
        )
    })?;

    let http = HttpClient::new();
    let body: TokenResponse = http
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| GitHubServiceError::Repository(format!("Token exchange request failed: {e}")))?
        .json()
        .await
        .map_err(|e| {
            GitHubServiceError::Repository(format!("Token exchange response parse failed: {e}"))
        })?;

    if let Some(err) = body.error {
        return Err(GitHubServiceError::Repository(format!(
            "GitHub OAuth error: {} ({})",
            err,
            body.error_description.unwrap_or_default(),
        )));
    }

    let access_token = body.access_token.ok_or_else(|| {
        GitHubServiceError::Repository("GitHub token response missing access_token".to_string())
    })?;

    Ok(OAuthTokens {
        access_token,
        refresh_token: body.refresh_token,
        expires_at: body
            .expires_in
            .map(|s| Utc::now() + chrono::Duration::seconds(s)),
        scope: body.scope,
    })
}
