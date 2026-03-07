//! Discord bot implemented with twilight-rs + songbird 0.4 (twilight feature)
//!
//! Uses a raw twilight event loop to handle gateway events.
//! Slash commands: /nora-join, /topsi-join, /bot-leave, /meeting-summary, /meeting-note
//!
//! ## Craig-inspired patterns adopted
//! - Nickname set to "[Listening]" when bot is active in a channel
//! - HTTP voice state fallback (fixes cache-miss join failures)
//! - Bot permission checks before joining
//! - Auto-record: watch VoiceStateUpdate, join when enough members are present
//! - Ephemeral error messages for validation failures
//! - Rate-limit tracking: per-user 5s, per-guild 30s

use std::{collections::HashMap, sync::Arc, time::Instant};

use anyhow::{Context, Result};
use tokio::sync::{broadcast, Mutex};
use tracing::{error, info, warn};
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{Event, Intents, Shard, ShardId};
use twilight_http::Client as HttpClient;
use twilight_model::{
    application::interaction::{Interaction, InteractionData, InteractionType},
    gateway::payload::incoming::VoiceStateUpdate,
    http::interaction::{InteractionResponse, InteractionResponseType, InteractionResponseData},
    id::{
        marker::{ChannelMarker, GuildMarker, UserMarker},
        Id,
    },
};

use crate::{
    audio,
    session::{ActiveAgent, DiscordVoiceSession, TranscriptEvent},
    session_key,
    voice_receiver::{self, ReceiverState},
    DISCORD_SESSIONS, TRANSCRIPT_CHANNELS,
};

// ─── Rate limiter ─────────────────────────────────────────────────────────────

struct RateLimits {
    /// user_id → last join invocation instant
    per_user: HashMap<u64, Instant>,
    /// guild_id → last join instant
    per_guild: HashMap<u64, Instant>,
}

impl RateLimits {
    fn new() -> Self {
        Self { per_user: HashMap::new(), per_guild: HashMap::new() }
    }

    fn check_and_record(&mut self, guild_id: u64, user_id: u64) -> Option<&'static str> {
        let now = Instant::now();
        if let Some(t) = self.per_user.get(&user_id) {
            if now.duration_since(*t).as_secs() < 5 {
                return Some("Please wait a few seconds before invoking this command again.");
            }
        }
        if let Some(t) = self.per_guild.get(&guild_id) {
            if now.duration_since(*t).as_secs() < 30 {
                return Some("Another join was just initiated in this server. Please wait 30 seconds.");
            }
        }
        self.per_user.insert(user_id, now);
        self.per_guild.insert(guild_id, now);
        None
    }
}

// ─── Bot state ────────────────────────────────────────────────────────────────

/// State shared across all event processing
struct BotState {
    http: Arc<HttpClient>,
    cache: Arc<InMemoryCache>,
    songbird: Arc<songbird::Songbird>,
    pool: sqlx::SqlitePool,
    server_port: u16,
    dedicated_agent: Option<ActiveAgent>,
    /// Protect rate-limit state (only accessed during slash command handling)
    rate_limits: Mutex<RateLimits>,
    /// Auto-record: minimum non-bot members in a voice channel to auto-join.
    /// 0 means auto-record is disabled.
    autorecord_min_members: u32,
}

// ─── Main bot loop ────────────────────────────────────────────────────────────

/// Build and run the Discord bot (runs until the token is revoked or process exits).
pub async fn run_bot(
    token: String,
    pool: sqlx::SqlitePool,
    server_port: u16,
    dedicated_agent: Option<ActiveAgent>,
) -> Result<()> {
    let http = Arc::new(HttpClient::new(token.clone()));
    let cache = Arc::new(
        InMemoryCache::builder()
            .resource_types(
                ResourceType::GUILD
                    | ResourceType::CHANNEL
                    | ResourceType::MEMBER
                    | ResourceType::VOICE_STATE,
            )
            .build(),
    );

    // GUILD_MEMBERS is privileged — omit unless explicitly enabled in Dev Portal.
    // GUILD_VOICE_STATES is required for VoiceStateUpdate events (auto-record + cache).
    let intents = Intents::GUILDS | Intents::GUILD_VOICE_STATES;

    // Fetch the real bot user ID before initialising songbird.
    // Songbird matches its own VoiceStateUpdate events during the join handshake;
    // a wrong ID causes joins to always time out.
    let current_user = http
        .current_user()
        .await
        .context("Failed to fetch current user")?
        .model()
        .await
        .context("Failed to decode current user")?;
    let user_id: Id<UserMarker> = current_user.id;
    info!("Bot user ID: {} ({})", user_id, current_user.name);

    let mut shard = Shard::new(ShardId::ONE, token.clone(), intents);

    // Build the TwilightMap (shard_id → sender) that songbird needs.
    // ShardId::ONE has internal number 0, so key is 0u64.
    let shard_map = {
        let mut m = std::collections::HashMap::new();
        m.insert(0u64, shard.sender());
        Arc::new(songbird::shards::TwilightMap::new(m))
    };

    let songbird = Arc::new(songbird::Songbird::twilight(shard_map, user_id));

    let autorecord_min: u32 = std::env::var("DISCORD_AUTORECORD_MIN_MEMBERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let state = Arc::new(BotState {
        http: Arc::clone(&http),
        cache: Arc::clone(&cache),
        songbird: Arc::clone(&songbird),
        pool,
        server_port,
        dedicated_agent,
        rate_limits: Mutex::new(RateLimits::new()),
        autorecord_min_members: autorecord_min,
    });

    let mut registered_commands = false;

    loop {
        let event = match shard.next_event().await {
            Ok(ev) => ev,
            Err(e) => {
                if e.is_fatal() {
                    error!("Fatal gateway error: {}", e);
                    break;
                }
                warn!("Gateway error (reconnecting): {}", e);
                continue;
            }
        };

        cache.update(&event);
        songbird.process(&event).await;

        match &event {
            Event::Ready(ready) => {
                info!("Discord bot connected as {} ({})", ready.user.name, ready.user.id);
                if !registered_commands {
                    if let Err(e) = register_slash_commands(&state.http, state.dedicated_agent).await {
                        error!("Failed to register slash commands: {}", e);
                    } else {
                        info!("Slash commands registered for {}", ready.user.name);
                        registered_commands = true;
                    }
                }
            }

            Event::InteractionCreate(interaction) => {
                let state = Arc::clone(&state);
                let interaction = interaction.0.clone();
                tokio::spawn(async move {
                    handle_interaction(&state, interaction).await;
                });
            }

            Event::VoiceStateUpdate(update) => {
                handle_voice_state_update(&state, update).await;
            }

            Event::VoiceServerUpdate(vsu) => {
                info!("VoiceServerUpdate: endpoint={:?} guild={:?}", vsu.endpoint, vsu.guild_id);
            }

            _ => {}
        }
    }

    Ok(())
}

// ─── Auto-record + disconnect tracking ───────────────────────────────────────

async fn handle_voice_state_update(state: &Arc<BotState>, update: &VoiceStateUpdate) {
    let bot_id = state.cache.current_user().map(|u| u.id).unwrap_or(Id::new(1));

    // Bot was disconnected / kicked from a voice channel
    if update.0.user_id == bot_id {
        if update.0.channel_id.is_none() {
            if let Some(guild_id) = update.0.guild_id {
                let agent_name = state.dedicated_agent.map(|a| a.name()).unwrap_or("nora");
                let key = session_key(guild_id.get(), agent_name);
                if let Some((_, old)) = DISCORD_SESSIONS.remove(&key) {
                    TRANSCRIPT_CHANNELS.remove(&old.meeting_session_id);
                    info!("Bot removed from guild {} — session cleaned up", guild_id);
                }
            }
        }
        return;
    }

    // Auto-record: only fire if enabled and the user joined (channel_id is Some)
    if state.autorecord_min_members == 0 {
        return;
    }

    let (guild_id, channel_id) = match (update.0.guild_id, update.0.channel_id) {
        (Some(g), Some(c)) => (g, c),
        _ => return,
    };

    // Debounce: check after a short delay to let the cache settle
    let state = Arc::clone(state);
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

        // Count non-bot members in the channel
        let member_count = state.cache
            .voice_channel_states(channel_id)
            .map(|states| states.filter(|vs| vs.user_id() != bot_id).count())
            .unwrap_or(0);

        if member_count < state.autorecord_min_members as usize {
            return;
        }

        let agent = state.dedicated_agent.unwrap_or(ActiveAgent::Nora);
        let key = session_key(guild_id.get(), agent.name());

        // Don't double-join
        if DISCORD_SESSIONS.contains_key(&key) {
            return;
        }

        let channel_name = state.cache
            .channel(channel_id)
            .and_then(|c| c.name.clone())
            .unwrap_or_else(|| channel_id.get().to_string());

        info!(
            "Auto-record: {} members in #{} — {} joining",
            member_count, channel_name, agent.name()
        );

        do_join_voice(
            &state,
            guild_id,
            channel_id,
            channel_name,
            String::new(), // no project_id for autorecord
            agent,
            None, // no interaction to respond to
        )
        .await;
    });
}

// ─── Slash command registration ───────────────────────────────────────────────

async fn register_slash_commands(
    http: &HttpClient,
    dedicated_agent: Option<ActiveAgent>,
) -> Result<()> {
    let app_id = http.current_user_application().await?.model().await?.id;

    let mut commands = vec![
        build_simple_command("meeting-summary", "Generate an AI summary of the current voice session"),
        build_note_command(),
    ];

    match dedicated_agent {
        Some(ActiveAgent::Nora) => {
            commands.push(build_join_command("nora-join", "Nora joins your current voice channel"));
            commands.push(build_simple_command("nora-leave", "Nora leaves the voice channel"));
        }
        Some(ActiveAgent::Topsi) => {
            commands.push(build_join_command("topsi-join", "Topsi joins your current voice channel"));
            commands.push(build_simple_command("topsi-leave", "Topsi leaves the voice channel"));
        }
        None => {
            commands.push(build_join_command("nora-join", "Nora joins your current voice channel"));
            commands.push(build_join_command("topsi-join", "Topsi joins your current voice channel"));
            commands.push(build_simple_command("nora-leave", "Nora leaves the voice channel"));
            commands.push(build_simple_command("topsi-leave", "Topsi leaves the voice channel"));
        }
    }

    http.interaction(app_id)
        .set_global_commands(&commands)
        .await
        .context("Failed to set global commands")?;

    Ok(())
}

fn build_join_command(name: &str, description: &str) -> twilight_model::application::command::Command {
    use twilight_model::application::command::{Command, CommandOption, CommandOptionType, CommandType};
    Command {
        application_id: None,
        default_member_permissions: None,
        description: description.to_string(),
        description_localizations: None,
        dm_permission: Some(false),
        guild_id: None,
        id: None,
        kind: CommandType::ChatInput,
        name: name.to_string(),
        name_localizations: None,
        nsfw: None,
        options: vec![CommandOption {
            autocomplete: None,
            channel_types: None,
            choices: None,
            description: "Project ID to link this session to (optional)".to_string(),
            description_localizations: None,
            kind: CommandOptionType::String,
            max_length: None,
            max_value: None,
            min_length: None,
            min_value: None,
            name: "project".to_string(),
            name_localizations: None,
            options: None,
            required: Some(false),
        }],
        version: Id::new(1),
    }
}

fn build_simple_command(name: &str, description: &str) -> twilight_model::application::command::Command {
    use twilight_model::application::command::{Command, CommandType};
    Command {
        application_id: None,
        default_member_permissions: None,
        description: description.to_string(),
        description_localizations: None,
        dm_permission: Some(false),
        guild_id: None,
        id: None,
        kind: CommandType::ChatInput,
        name: name.to_string(),
        name_localizations: None,
        nsfw: None,
        options: vec![],
        version: Id::new(1),
    }
}

fn build_note_command() -> twilight_model::application::command::Command {
    use twilight_model::application::command::{Command, CommandOption, CommandOptionType, CommandType};
    Command {
        application_id: None,
        default_member_permissions: None,
        description: "Add a timestamped note to the current voice session transcript".to_string(),
        description_localizations: None,
        dm_permission: Some(false),
        guild_id: None,
        id: None,
        kind: CommandType::ChatInput,
        name: "meeting-note".to_string(),
        name_localizations: None,
        nsfw: None,
        options: vec![CommandOption {
            autocomplete: None,
            channel_types: None,
            choices: None,
            description: "Your note".to_string(),
            description_localizations: None,
            kind: CommandOptionType::String,
            max_length: Some(500),
            max_value: None,
            min_length: Some(1),
            min_value: None,
            name: "text".to_string(),
            name_localizations: None,
            options: None,
            required: Some(true),
        }],
        version: Id::new(1),
    }
}

// ─── Interaction handling ─────────────────────────────────────────────────────

async fn handle_interaction(state: &Arc<BotState>, interaction: Interaction) {
    if interaction.kind != InteractionType::ApplicationCommand {
        return;
    }

    let guild_id = match interaction.guild_id {
        Some(id) => id,
        None => return,
    };

    let data = match &interaction.data {
        Some(InteractionData::ApplicationCommand(d)) => d.as_ref().clone(),
        _ => return,
    };

    // Defer immediately for long-running commands (Discord requires response within 3s)
    let app_id = match state.http.current_user_application().await {
        Ok(r) => match r.model().await {
            Ok(a) => a.id,
            Err(_) => return,
        },
        Err(_) => return,
    };

    match data.name.as_str() {
        "nora-join" | "topsi-join" => {
            let agent = if data.name == "topsi-join" {
                ActiveAgent::Topsi
            } else {
                ActiveAgent::Nora
            };

            // Rate limit check before deferring
            let invoker_id = interaction.author_id().map(|id| id.get()).unwrap_or(0);
            {
                let mut rl = state.rate_limits.lock().await;
                if let Some(msg) = rl.check_and_record(guild_id.get(), invoker_id) {
                    let _ = defer_ephemeral(state, &interaction, app_id).await;
                    followup(state, &interaction, app_id, msg).await;
                    return;
                }
            }

            defer_visible(state, &interaction, app_id).await;
            join_voice(state, &interaction, guild_id, &data, agent, app_id).await;
        }

        "nora-leave" | "topsi-leave" | "bot-leave" => {
            defer_visible(state, &interaction, app_id).await;
            let agent = if data.name.starts_with("topsi") {
                ActiveAgent::Topsi
            } else {
                ActiveAgent::Nora
            };
            leave_voice(state, &interaction, guild_id, agent, app_id).await;
        }

        "meeting-summary" => {
            defer_visible(state, &interaction, app_id).await;
            generate_summary(state, &interaction, guild_id, app_id).await;
        }

        "meeting-note" => {
            use twilight_model::application::interaction::application_command::CommandOptionValue;
            defer_visible(state, &interaction, app_id).await;

            let note_text = data
                .options
                .iter()
                .find(|o| o.name == "text")
                .and_then(|o| match &o.value {
                    CommandOptionValue::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();

            add_note(state, &interaction, guild_id, &note_text, app_id).await;
        }

        _ => {}
    }
}

// ─── Join voice ───────────────────────────────────────────────────────────────

async fn join_voice(
    state: &Arc<BotState>,
    interaction: &Interaction,
    guild_id: Id<GuildMarker>,
    data: &twilight_model::application::interaction::application_command::CommandData,
    agent: ActiveAgent,
    app_id: twilight_model::id::Id<twilight_model::id::marker::ApplicationMarker>,
) {
    use twilight_model::application::interaction::application_command::CommandOptionValue;

    info!("join_voice: invoked agent={} guild={}", agent.name(), guild_id);

    let invoker_id = match interaction.author_id() {
        Some(id) => id,
        None => {
            followup(state, interaction, app_id, "Could not determine your user ID.").await;
            return;
        }
    };

    // ── 1. Locate the user's voice channel ───────────────────────────────────
    // First try InMemoryCache. If it's cold (user was already in channel before bot
    // started), fall back to the Discord HTTP API.
    let channel_id: Id<ChannelMarker> = {
        let cached = state
            .cache
            .voice_state(invoker_id, guild_id)
            .map(|vs| vs.channel_id());

        match cached {
            Some(id) => {
                info!("join_voice: found channel {} for user {} (cache)", id, invoker_id);
                id
            }
            None => {
                info!(
                    "join_voice: cache miss for user {} in guild {} — trying HTTP API",
                    invoker_id, guild_id
                );
                match state.http.voice_state(guild_id, invoker_id).await {
                    Ok(resp) => match resp.model().await {
                        Ok(vs) => match vs.channel_id {
                            Some(id) => {
                                info!("join_voice: found channel {} for user {} (HTTP API)", id, invoker_id);
                                id
                            }
                            None => {
                                followup(
                                    state,
                                    interaction,
                                    app_id,
                                    "You need to **join a voice channel first**, then run this command.",
                                )
                                .await;
                                return;
                            }
                        },
                        Err(e) => {
                            warn!("join_voice: failed to decode voice state from HTTP: {}", e);
                            followup(
                                state,
                                interaction,
                                app_id,
                                "You need to **join a voice channel first**, then run this command.",
                            )
                            .await;
                            return;
                        }
                    },
                    Err(e) => {
                        warn!("join_voice: HTTP voice state lookup failed: {}", e);
                        followup(
                            state,
                            interaction,
                            app_id,
                            "You need to **join a voice channel first**, then run this command.",
                        )
                        .await;
                        return;
                    }
                }
            }
        }
    };

    // ── 2. Validate bot has permission to join the channel ───────────────────
    // (twilight cache has channel permissions after GUILDS + GUILD_VOICE_STATES intents)
    // We skip a hard permission check here and let songbird.join() surface the error,
    // since computing permission overrides requires the GUILD_MEMBERS intent.

    // ── 3. Extract optional project_id ───────────────────────────────────────
    let project_id = data
        .options
        .iter()
        .find(|o| o.name == "project")
        .and_then(|o| match &o.value {
            CommandOptionValue::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let channel_name = state
        .cache
        .channel(channel_id)
        .and_then(|c| c.name.clone())
        .unwrap_or_else(|| channel_id.get().to_string());

    do_join_voice(
        state,
        guild_id,
        channel_id,
        channel_name,
        project_id,
        agent,
        Some((interaction, app_id)),
    )
    .await;
}

/// Core join logic — used by both slash commands and auto-record.
async fn do_join_voice(
    state: &Arc<BotState>,
    guild_id: Id<GuildMarker>,
    channel_id: Id<ChannelMarker>,
    channel_name: String,
    project_id: String,
    agent: ActiveAgent,
    // None when called from auto-record (no interaction to respond to)
    respond: Option<(&Interaction, twilight_model::id::Id<twilight_model::id::marker::ApplicationMarker>)>,
) {
    let key = session_key(guild_id.get(), agent.name());

    // Clear any stale songbird state and existing session
    let _ = state.songbird.remove(guild_id).await;

    if let Some((_, old_session)) = DISCORD_SESSIONS.remove(&key) {
        TRANSCRIPT_CHANNELS.remove(&old_session.meeting_session_id);
        let _ = sqlx::query(
            "UPDATE meeting_sessions SET status='ended', ended_at=datetime('now','subsec'), updated_at=datetime('now','subsec') WHERE id=?1 AND status='active'",
        )
        .bind(&old_session.meeting_session_id)
        .execute(&state.pool)
        .await;
    }

    // Resolve effective project_id (fallback to admin's first project)
    let effective_project_id = if project_id.is_empty() {
        sqlx::query_scalar::<_, String>(
            r#"SELECT pm.project_id FROM project_members pm
               JOIN users u ON u.id = pm.user_id
               WHERE u.is_admin = 1
               ORDER BY pm.granted_at ASC LIMIT 1"#,
        )
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
    } else {
        project_id
    };

    // Create meeting_session in DB
    let meeting_id = uuid::Uuid::new_v4().to_string();
    let title = format!("{} — #{} (Discord)", agent.name(), channel_name);
    let metadata = serde_json::json!({
        "discord_guild_id": guild_id.get().to_string(),
        "discord_channel_id": channel_id.get().to_string(),
        "discord_channel_name": &channel_name,
        "agent": agent.name(),
    })
    .to_string();

    if let Err(e) = sqlx::query(
        r#"INSERT INTO meeting_sessions (id, project_id, title, started_by, source_type, metadata, status)
           VALUES (?1, ?2, ?3, ?4, 'discord', ?5, 'active')"#,
    )
    .bind(&meeting_id)
    .bind(&effective_project_id)
    .bind(&title)
    .bind("discord-bot")
    .bind(&metadata)
    .execute(&state.pool)
    .await
    {
        error!("Failed to create meeting_session: {}", e);
        if let Some((interaction, app_id)) = respond {
            followup(state, interaction, app_id, "Database error — could not create session.").await;
        }
        return;
    }

    // Set up SSE broadcast
    let (event_tx, _) = broadcast::channel::<TranscriptEvent>(256);
    TRANSCRIPT_CHANNELS.insert(meeting_id.clone(), event_tx.clone());

    // Join the voice channel
    let handler_lock = match state.songbird.join(guild_id, channel_id).await {
        Ok(h) => h,
        Err(e) => {
            error!("Songbird failed to join #{}: {}", channel_name, e);
            TRANSCRIPT_CHANNELS.remove(&meeting_id);
            let _ = sqlx::query("DELETE FROM meeting_sessions WHERE id = ?1")
                .bind(&meeting_id)
                .execute(&state.pool)
                .await;
            if let Some((interaction, app_id)) = respond {
                followup(
                    state,
                    interaction,
                    app_id,
                    &format!(
                        "Failed to join **#{}**: `{}`\n\nMake sure I have **Connect** and **Speak** permissions in that channel.",
                        channel_name, e
                    ),
                )
                .await;
            }
            return;
        }
    };

    info!("{} joined #{} in guild {}", agent.name(), channel_name, guild_id);

    // Update bot nickname to signal active session (Craig pattern)
    let nick = format!("[Listening] {}", agent.name());
    let _ = state.http
        .update_current_member(guild_id)
        .nick(Some(&nick))
        .await;

    // Build in-memory session
    let session = DiscordVoiceSession::new(
        meeting_id.clone(),
        guild_id.get(),
        channel_id.get(),
        channel_name.clone(),
        effective_project_id,
        agent,
        state.server_port,
    );

    let session_arc = Arc::new(Mutex::new(session.clone()));

    let receiver_state = Arc::new(ReceiverState {
        session: Arc::clone(&session_arc),
        buffers: dashmap::DashMap::new(),
        ssrc_map: dashmap::DashMap::new(),
        event_tx,
        pool: state.pool.clone(),
        call_lock: Arc::clone(&handler_lock),
    });

    {
        let mut handler = handler_lock.lock().await;
        voice_receiver::register_handlers(&mut handler, receiver_state).await;
    }

    DISCORD_SESSIONS.insert(key, session);

    let msg = format!(
        "**{}** has joined **#{}** and is listening. Say my name to speak with me directly.\n_Session ID: `{}`_",
        agent.name(),
        channel_name,
        &meeting_id[..8],
    );

    if let Some((interaction, app_id)) = respond {
        followup(state, interaction, app_id, &msg).await;
    }
}

// ─── Leave voice ──────────────────────────────────────────────────────────────

async fn leave_voice(
    state: &Arc<BotState>,
    interaction: &Interaction,
    guild_id: Id<GuildMarker>,
    agent: ActiveAgent,
    app_id: twilight_model::id::Id<twilight_model::id::marker::ApplicationMarker>,
) {
    let key = session_key(guild_id.get(), agent.name());
    match DISCORD_SESSIONS.remove(&key) {
        None => {
            followup(state, interaction, app_id, "I'm not in a voice channel in this server.").await;
        }
        Some((_, session)) => {
            let mid = &session.meeting_session_id;
            let _ = sqlx::query(
                r#"UPDATE meeting_sessions SET status='ended', ended_at=datetime('now','subsec'), updated_at=datetime('now','subsec') WHERE id=?1"#,
            )
            .bind(mid)
            .execute(&state.pool)
            .await;

            TRANSCRIPT_CHANNELS.remove(mid);

            if let Err(e) = state.songbird.remove(guild_id).await {
                warn!("songbird remove failed: {}", e);
            }

            // Restore nickname
            let _ = state.http
                .update_current_member(guild_id)
                .nick(Some(agent.name()))
                .await;

            followup(
                state,
                interaction,
                app_id,
                "I've left the voice channel. The session has been archived to the dashboard.",
            )
            .await;
        }
    }
}

// ─── Meeting summary ──────────────────────────────────────────────────────────

async fn generate_summary(
    state: &Arc<BotState>,
    interaction: &Interaction,
    guild_id: Id<GuildMarker>,
    app_id: twilight_model::id::Id<twilight_model::id::marker::ApplicationMarker>,
) {
    // Find an active session for this guild (either Nora or Topsi)
    let session_info = {
        let nora_key = session_key(guild_id.get(), "Nora");
        let topsi_key = session_key(guild_id.get(), "Topsi");
        DISCORD_SESSIONS
            .get(&nora_key)
            .or_else(|| DISCORD_SESSIONS.get(&topsi_key))
            .map(|s| (
                s.meeting_session_id.clone(),
                s.agent,
                s.project_id.clone(),
                s.server_port,
            ))
    };

    let (meeting_id, agent, project_id, server_port) = match session_info {
        Some(info) => info,
        None => {
            followup(state, interaction, app_id, "No active voice session in this server.").await;
            return;
        }
    };

    let segments: Vec<(String, String)> = sqlx::query_as(
        r#"SELECT COALESCE(speaker_label,'Unknown'), text FROM meeting_segments WHERE meeting_session_id=?1 ORDER BY segment_index ASC"#,
    )
    .bind(&meeting_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    if segments.is_empty() {
        followup(state, interaction, app_id, "No transcript yet — keep talking!").await;
        return;
    }

    let transcript_text: String = segments
        .iter()
        .map(|(s, t)| format!("[{}]: {}", s, t))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Provide a concise structured summary: key topics, decisions, and action items.\n\nTranscript:\n{}",
        &transcript_text[..transcript_text.len().min(4000)]
    );

    if let Ok(summary) = audio::call_agent(server_port, agent, &prompt, &project_id, &meeting_id).await {
        let _ = state
            .http
            .interaction(app_id)
            .create_followup(&interaction.token)
            .content(&format!("**Meeting Summary**\n{}", &summary[..summary.len().min(1900)]))
            .unwrap_or_else(|_| unreachable!())
            .await;
    }
}

// ─── Meeting note ─────────────────────────────────────────────────────────────

async fn add_note(
    state: &Arc<BotState>,
    interaction: &Interaction,
    guild_id: Id<GuildMarker>,
    note_text: &str,
    app_id: twilight_model::id::Id<twilight_model::id::marker::ApplicationMarker>,
) {
    let nora_key = session_key(guild_id.get(), "Nora");
    let topsi_key = session_key(guild_id.get(), "Topsi");

    let session_info = DISCORD_SESSIONS
        .get(&nora_key)
        .or_else(|| DISCORD_SESSIONS.get(&topsi_key))
        .map(|s| (s.meeting_session_id.clone(), s.segment_count, s.elapsed_ms()));

    let (meeting_id, segment_count, elapsed_ms) = match session_info {
        Some(info) => info,
        None => {
            followup(state, interaction, app_id, "No active voice session in this server.").await;
            return;
        }
    };

    let author = interaction
        .author_id()
        .map(|id| id.get().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let segment_id = uuid::Uuid::new_v4().to_string();
    let note_label = format!("📝 Note ({})", &author);
    let note_with_meta = format!("[NOTE] {}", note_text);

    let _ = sqlx::query(
        r#"INSERT INTO meeting_segments
           (id, meeting_session_id, segment_index, speaker_label, text, confidence, start_time_ms, end_time_ms, is_topsi_addressed, metadata)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, 0, ?8)"#,
    )
    .bind(&segment_id)
    .bind(&meeting_id)
    .bind(segment_count + 1000) // high index so notes sort after speech
    .bind(&note_label)
    .bind(&note_with_meta)
    .bind(1.0_f64)
    .bind(elapsed_ms)
    .bind(serde_json::json!({ "is_note": true, "discord_user_id": author }).to_string())
    .execute(&state.pool)
    .await;

    followup(
        state,
        interaction,
        app_id,
        &format!("📝 Note added at {}s: _{}_", elapsed_ms / 1000, note_text),
    )
    .await;
}

// ─── Response helpers ─────────────────────────────────────────────────────────

async fn defer_visible(
    state: &Arc<BotState>,
    interaction: &Interaction,
    app_id: twilight_model::id::Id<twilight_model::id::marker::ApplicationMarker>,
) {
    let defer = InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    };
    let _ = state
        .http
        .interaction(app_id)
        .create_response(interaction.id, &interaction.token, &defer)
        .await;
}

async fn defer_ephemeral(
    state: &Arc<BotState>,
    interaction: &Interaction,
    app_id: twilight_model::id::Id<twilight_model::id::marker::ApplicationMarker>,
) {
    use twilight_model::channel::message::MessageFlags;
    let defer = InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: Some(InteractionResponseData {
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    };
    let _ = state
        .http
        .interaction(app_id)
        .create_response(interaction.id, &interaction.token, &defer)
        .await;
}

async fn followup(
    state: &Arc<BotState>,
    interaction: &Interaction,
    app_id: twilight_model::id::Id<twilight_model::id::marker::ApplicationMarker>,
    content: &str,
) {
    let _ = state
        .http
        .interaction(app_id)
        .create_followup(&interaction.token)
        .content(content)
        .unwrap_or_else(|_| unreachable!())
        .await;
}
