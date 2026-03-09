use anyhow::Result;

#[derive(Debug, Clone)]
pub struct BotConfig {
    /// Nora Discord bot token
    pub nora_token: Option<String>,
    /// Topsi Discord bot token
    pub topsi_token: Option<String>,
    /// PCG backend API base URL (e.g., http://localhost:3002)
    pub backend_url: String,
    /// API auth token for backend requests
    pub backend_auth_token: Option<String>,
    /// Guild ID for slash command registration (None = global)
    pub guild_id: Option<u64>,
    /// Channel IDs where Nora is restricted to (empty = all channels)
    pub nora_channels: Vec<u64>,
    /// Channel IDs where Topsi is restricted to (empty = all channels)
    pub topsi_channels: Vec<u64>,
    /// Nora admin-only role ID (if set, only users with this role can talk to Nora)
    pub nora_admin_role: Option<u64>,
}

impl BotConfig {
    pub fn from_env() -> Result<Self> {
        let nora_token = std::env::var("DISCORD_NORA_BOT_TOKEN").ok();
        let topsi_token = std::env::var("DISCORD_TOPSI_BOT_TOKEN").ok();

        if nora_token.is_none() && topsi_token.is_none() {
            anyhow::bail!(
                "At least one bot token required. Set DISCORD_NORA_BOT_TOKEN and/or DISCORD_TOPSI_BOT_TOKEN"
            );
        }

        let backend_url = std::env::var("PCG_BACKEND_URL")
            .unwrap_or_else(|_| "http://localhost:3002".to_string());

        let backend_auth_token = std::env::var("PCG_BACKEND_AUTH_TOKEN").ok();

        let guild_id = std::env::var("DISCORD_GUILD_ID")
            .ok()
            .and_then(|v| v.parse().ok());

        let nora_channels = parse_id_list("DISCORD_NORA_CHANNELS");
        let topsi_channels = parse_id_list("DISCORD_TOPSI_CHANNELS");

        let nora_admin_role = std::env::var("DISCORD_NORA_ADMIN_ROLE")
            .ok()
            .and_then(|v| v.parse().ok());

        Ok(Self {
            nora_token,
            topsi_token,
            backend_url,
            backend_auth_token,
            guild_id,
            nora_channels,
            topsi_channels,
            nora_admin_role,
        })
    }
}

fn parse_id_list(env_var: &str) -> Vec<u64> {
    std::env::var(env_var)
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect()
}
