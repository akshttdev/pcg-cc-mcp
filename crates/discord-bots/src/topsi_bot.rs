use std::sync::Arc;

use serenity::all::*;
use tracing::{error, info};

use crate::backend::BackendClient;
use crate::config::BotConfig;
use crate::utils::{split_message, format_agent_embed};
use crate::voice::VoiceManager;

pub struct TopsiHandler {
    backend: Arc<BackendClient>,
    config: Arc<BotConfig>,
    voice: Arc<VoiceManager>,
}

impl TopsiHandler {
    pub fn new(backend: Arc<BackendClient>, config: Arc<BotConfig>, voice: Arc<VoiceManager>) -> Self {
        Self { backend, config, voice }
    }

    fn is_allowed_channel(&self, channel_id: ChannelId) -> bool {
        self.config.topsi_channels.is_empty()
            || self.config.topsi_channels.contains(&channel_id.get())
    }

    fn session_id(user_id: UserId, channel_id: ChannelId) -> String {
        format!("discord-topsi-{}-{}", user_id.get(), channel_id.get())
    }
}

#[async_trait]
impl EventHandler for TopsiHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("[TOPSI BOT] Connected as {}", ready.user.name);

        let guild_id = self.config.guild_id.map(GuildId::new);

        let commands = vec![
            CreateCommand::new("topsi")
                .description("Chat with Topsi, the Platform Orchestrator")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "message",
                        "Your message to Topsi",
                    )
                    .required(true),
                ),
            CreateCommand::new("topsi-status")
                .description("Check Topsi's status and topology health"),
            CreateCommand::new("topsi-task")
                .description("Create a task via Topsi")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "title",
                        "Task title",
                    )
                    .required(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "description",
                        "Task description",
                    )
                    .required(false),
                ),
            CreateCommand::new("topsi-join")
                .description("Topsi joins your voice channel"),
            CreateCommand::new("topsi-leave")
                .description("Topsi leaves the voice channel"),
        ];

        if let Some(gid) = guild_id {
            match gid.set_commands(&ctx.http, commands).await {
                Ok(cmds) => info!("[TOPSI BOT] Registered {} guild commands", cmds.len()),
                Err(e) => error!("[TOPSI BOT] Failed to register guild commands: {}", e),
            }
        } else {
            match Command::set_global_commands(&ctx.http, commands).await {
                Ok(cmds) => info!("[TOPSI BOT] Registered {} global commands", cmds.len()),
                Err(e) => error!("[TOPSI BOT] Failed to register global commands: {}", e),
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Interaction::Command(command) = interaction else {
            return;
        };

        if !self.is_allowed_channel(command.channel_id) {
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Topsi is not available in this channel.")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }

        match command.data.name.as_str() {
            "topsi" => self.handle_chat_command(&ctx, &command).await,
            "topsi-status" => self.handle_status_command(&ctx, &command).await,
            "topsi-task" => self.handle_task_command(&ctx, &command).await,
            "topsi-join" => self.handle_join_command(&ctx, &command).await,
            "topsi-leave" => self.handle_leave_command(&ctx, &command).await,
            _ => {}
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        if !self.is_allowed_channel(msg.channel_id) {
            return;
        }

        let current_user_id = ctx.cache.current_user().id;
        let mentioned = msg.mentions.iter().any(|u| u.id == current_user_id);
        if !mentioned {
            return;
        }

        let clean_message = msg
            .content
            .replace(&format!("<@{}>", current_user_id.get()), "")
            .replace(&format!("<@!{}>", current_user_id.get()), "")
            .trim()
            .to_string();

        if clean_message.is_empty() {
            let _ = msg
                .reply(&ctx.http, "Hey! I'm Topsi, the platform orchestrator. What can I help you with?")
                .await;
            return;
        }

        let typing = msg.channel_id.start_typing(&ctx.http);
        let session_id = Self::session_id(msg.author.id, msg.channel_id);

        match self.backend.chat_topsi(&clean_message, &session_id).await {
            Ok(response) => {
                drop(typing);
                let chunks = split_message(&response.content, 2000);
                for (i, chunk) in chunks.iter().enumerate() {
                    if i == 0 {
                        let mut embed = format_agent_embed(
                            "Topsi",
                            chunk,
                            0x8B5CF6,
                            response.input_tokens,
                            response.output_tokens,
                        );

                        if let Some(summary) = &response.topology_summary {
                            if let Some(health) = summary.get("healthScore").and_then(|h| h.as_f64()) {
                                embed = embed.field(
                                    "Topology Health",
                                    format!("{:.0}%", health * 100.0),
                                    true,
                                );
                            }
                        }

                        if !response.issues.is_empty() {
                            embed = embed.field(
                                "Issues Detected",
                                format!("{}", response.issues.len()),
                                true,
                            );
                        }

                        let _ = msg
                            .channel_id
                            .send_message(
                                &ctx.http,
                                CreateMessage::new()
                                    .embed(embed)
                                    .reference_message(&msg),
                            )
                            .await;
                    } else {
                        let _ = msg.channel_id.send_message(
                            &ctx.http,
                            CreateMessage::new().content(chunk),
                        ).await;
                    }
                }
            }
            Err(e) => {
                drop(typing);
                error!("[TOPSI BOT] Chat error: {}", e);
                let _ = msg
                    .reply(
                        &ctx.http,
                        format!("I hit an issue processing that: {}", e),
                    )
                    .await;
            }
        }
    }
}

impl TopsiHandler {
    async fn handle_chat_command(&self, ctx: &Context, command: &CommandInteraction) {
        let message = command
            .data
            .options
            .iter()
            .find(|o| o.name == "message")
            .and_then(|o| o.value.as_str())
            .unwrap_or("");

        if message.is_empty() {
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Please provide a message.")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }

        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Defer(
                    CreateInteractionResponseMessage::new(),
                ),
            )
            .await;

        let session_id = Self::session_id(command.user.id, command.channel_id);

        match self.backend.chat_topsi(message, &session_id).await {
            Ok(response) => {
                let embed = format_agent_embed(
                    "Topsi",
                    &response.content,
                    0x8B5CF6,
                    response.input_tokens,
                    response.output_tokens,
                );
                let _ = command
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content(format!("> {}", truncate(message, 200)))
                            .embed(embed),
                    )
                    .await;
            }
            Err(e) => {
                error!("[TOPSI BOT] Slash command error: {}", e);
                let _ = command
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content(format!("Error: {}", e)),
                    )
                    .await;
            }
        }
    }

    async fn handle_status_command(&self, ctx: &Context, command: &CommandInteraction) {
        let healthy = self.backend.health_check().await.unwrap_or(false);
        let status = if healthy { "Online" } else { "Offline" };
        let color = if healthy { 0x22C55E } else { 0xEF4444 };

        let embed = CreateEmbed::new()
            .title("Topsi - Topological Super Intelligence")
            .description(format!("**Status:** {}", status))
            .color(color)
            .field("Role", "Platform Orchestrator", true)
            .field("Backend", &self.config.backend_url, true)
            .timestamp(serenity::model::Timestamp::now());

        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .ephemeral(true),
                ),
            )
            .await;
    }

    async fn handle_task_command(&self, ctx: &Context, command: &CommandInteraction) {
        let title = command
            .data
            .options
            .iter()
            .find(|o| o.name == "title")
            .and_then(|o| o.value.as_str())
            .unwrap_or("");

        let description = command
            .data
            .options
            .iter()
            .find(|o| o.name == "description")
            .and_then(|o| o.value.as_str())
            .unwrap_or("");

        if title.is_empty() {
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Please provide a task title.")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }

        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Defer(
                    CreateInteractionResponseMessage::new(),
                ),
            )
            .await;

        let task_message = if description.is_empty() {
            format!("Create a new task titled: {}", title)
        } else {
            format!(
                "Create a new task titled: {}. Description: {}",
                title, description
            )
        };

        let session_id = Self::session_id(command.user.id, command.channel_id);

        match self.backend.chat_topsi(&task_message, &session_id).await {
            Ok(response) => {
                let embed = CreateEmbed::new()
                    .title("Task Created via Topsi")
                    .description(&response.content)
                    .color(0x22C55E)
                    .timestamp(serenity::model::Timestamp::now());

                let _ = command
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().embed(embed),
                    )
                    .await;
            }
            Err(e) => {
                error!("[TOPSI BOT] Task creation error: {}", e);
                let _ = command
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content(format!("Failed to create task: {}", e)),
                    )
                    .await;
            }
        }
    }

    async fn handle_join_command(&self, ctx: &Context, command: &CommandInteraction) {
        let guild_id = match command.guild_id {
            Some(id) => id,
            None => {
                let _ = command
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("This command only works in a server.")
                                .ephemeral(true),
                        ),
                    )
                    .await;
                return;
            }
        };

        let channel_id = guild_id
            .to_guild_cached(&ctx.cache)
            .and_then(|guild| {
                guild.voice_states.get(&command.user.id).and_then(|vs| vs.channel_id)
            });

        let channel_id = match channel_id {
            Some(id) => id,
            None => {
                let _ = command
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("You need to be in a voice channel first.")
                                .ephemeral(true),
                        ),
                    )
                    .await;
                return;
            }
        };

        match self.voice.join_channel(ctx, guild_id, channel_id).await {
            Ok(_) => {
                let embed = CreateEmbed::new()
                    .title("Topsi Voice Active")
                    .description("I've joined the voice channel. I'll listen and respond when spoken to.")
                    .color(0x8B5CF6)
                    .field("Channel", format!("<#{}>", channel_id), true)
                    .field("STT", &self.voice.config.stt_url, true)
                    .field("TTS", &self.voice.config.tts_url, true)
                    .timestamp(serenity::model::Timestamp::now());

                let _ = command
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().embed(embed),
                        ),
                    )
                    .await;
            }
            Err(e) => {
                let _ = command
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content(format!("Failed to join voice: {}", e))
                                .ephemeral(true),
                        ),
                    )
                    .await;
            }
        }
    }

    async fn handle_leave_command(&self, ctx: &Context, command: &CommandInteraction) {
        let guild_id = match command.guild_id {
            Some(id) => id,
            None => {
                let _ = command
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("This command only works in a server.")
                                .ephemeral(true),
                        ),
                    )
                    .await;
                return;
            }
        };

        match self.voice.leave_channel(ctx, guild_id).await {
            Ok(end_result) => {
                let embed = if let Some(meeting) = end_result {
                    CreateEmbed::new()
                        .title("Topsi - Meeting Ended")
                        .color(0x8B5CF6)
                        .field("Duration", format!("{}s", meeting.duration_seconds), true)
                        .field("Segments", format!("{}", meeting.segment_count), true)
                        .field("Participants", format!("{}", meeting.participant_count), true)
                        .description("Meeting transcript saved to dashboard. Notes generated.")
                        .timestamp(serenity::model::Timestamp::now())
                } else {
                    CreateEmbed::new()
                        .title("Topsi - Left Voice")
                        .description("Left the voice channel.")
                        .color(0x8B5CF6)
                };

                let _ = command
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().embed(embed),
                        ),
                    )
                    .await;
            }
            Err(e) => {
                let _ = command
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content(format!("Failed to leave: {}", e))
                                .ephemeral(true),
                        ),
                    )
                    .await;
            }
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
