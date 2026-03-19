use std::sync::Arc;

use serenity::all::*;
use tracing::{error, info};

use crate::{
    backend::BackendClient,
    config::BotConfig,
    utils::{format_agent_embed, split_message},
    voice::VoiceManager,
};

pub struct NoraHandler {
    backend: Arc<BackendClient>,
    config: Arc<BotConfig>,
    voice: Arc<VoiceManager>,
}

impl NoraHandler {
    pub fn new(
        backend: Arc<BackendClient>,
        config: Arc<BotConfig>,
        voice: Arc<VoiceManager>,
    ) -> Self {
        Self {
            backend,
            config,
            voice,
        }
    }

    fn is_allowed_channel(&self, channel_id: ChannelId) -> bool {
        self.config.nora_channels.is_empty()
            || self.config.nora_channels.contains(&channel_id.get())
    }

    fn has_admin_role(&self, member: &Option<Box<PartialMember>>) -> bool {
        let Some(role_id) = self.config.nora_admin_role else {
            return true;
        };
        match member {
            Some(m) => m.roles.iter().any(|r| r.get() == role_id),
            None => false,
        }
    }

    fn session_id(user_id: UserId, channel_id: ChannelId) -> String {
        format!("discord-nora-{}-{}", user_id.get(), channel_id.get())
    }
}

#[async_trait]
impl EventHandler for NoraHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("[NORA BOT] Connected as {}", ready.user.name);

        let guild_id = self.config.guild_id.map(GuildId::new);

        let commands = vec![
            CreateCommand::new("nora")
                .description("Chat with Nora, the Executive AI Assistant")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "message",
                        "Your message to Nora",
                    )
                    .required(true),
                ),
            CreateCommand::new("nora-status").description("Check Nora's current status"),
            CreateCommand::new("nora-join")
                .description("Nora joins your voice channel to listen and speak"),
            CreateCommand::new("nora-leave").description("Nora leaves the voice channel"),
        ];

        if let Some(gid) = guild_id {
            match gid.set_commands(&ctx.http, commands).await {
                Ok(cmds) => info!("[NORA BOT] Registered {} guild commands", cmds.len()),
                Err(e) => error!("[NORA BOT] Failed to register guild commands: {}", e),
            }
        } else {
            match Command::set_global_commands(&ctx.http, commands).await {
                Ok(cmds) => info!("[NORA BOT] Registered {} global commands", cmds.len()),
                Err(e) => error!("[NORA BOT] Failed to register global commands: {}", e),
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
                            .content("Nora is not available in this channel.")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }

        match command.data.name.as_str() {
            "nora" => self.handle_chat_command(&ctx, &command).await,
            "nora-status" => self.handle_status_command(&ctx, &command).await,
            "nora-join" => self.handle_join_command(&ctx, &command).await,
            "nora-leave" => self.handle_leave_command(&ctx, &command).await,
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

        if !self.has_admin_role(&msg.member) {
            let _ = msg
                .reply(&ctx.http, "You don't have permission to chat with Nora.")
                .await;
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
                .reply(&ctx.http, "Hello! How can I assist you today?")
                .await;
            return;
        }

        let typing = msg.channel_id.start_typing(&ctx.http);
        let session_id = Self::session_id(msg.author.id, msg.channel_id);

        match self.backend.chat_nora(&clean_message, &session_id).await {
            Ok(response) => {
                drop(typing);
                let chunks = split_message(&response.content, 2000);
                for (i, chunk) in chunks.iter().enumerate() {
                    if i == 0 {
                        let embed = format_agent_embed(
                            "Nora",
                            chunk,
                            0x3B82F6,
                            response.input_tokens,
                            response.output_tokens,
                        );
                        let _ = msg
                            .channel_id
                            .send_message(
                                &ctx.http,
                                CreateMessage::new().embed(embed).reference_message(&msg),
                            )
                            .await;
                    } else {
                        let _ = msg
                            .channel_id
                            .send_message(&ctx.http, CreateMessage::new().content(chunk))
                            .await;
                    }
                }
            }
            Err(e) => {
                drop(typing);
                error!("[NORA BOT] Chat error: {}", e);
                let _ = msg
                    .reply(
                        &ctx.http,
                        format!("I encountered an issue processing your request: {}", e),
                    )
                    .await;
            }
        }
    }
}

impl NoraHandler {
    fn check_admin_roles(&self, roles: &[RoleId]) -> bool {
        let Some(role_id) = self.config.nora_admin_role else {
            return true;
        };
        roles.iter().any(|r| r.get() == role_id)
    }

    async fn handle_chat_command(&self, ctx: &Context, command: &CommandInteraction) {
        let has_role = command
            .member
            .as_ref()
            .map(|m| self.check_admin_roles(&m.roles))
            .unwrap_or(self.config.nora_admin_role.is_none());
        if !has_role {
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("You don't have permission to chat with Nora.")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }

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
                CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
            )
            .await;

        let session_id = Self::session_id(command.user.id, command.channel_id);

        match self.backend.chat_nora(message, &session_id).await {
            Ok(response) => {
                let embed = format_agent_embed(
                    "Nora",
                    &response.content,
                    0x3B82F6,
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
                error!("[NORA BOT] Slash command error: {}", e);
                let _ = command
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content(format!("Error: {}", e)),
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
            .title("Nora - Executive AI Assistant")
            .description(format!("**Status:** {}", status))
            .color(color)
            .field("Role", "Private Admin Assistant", true)
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

        // Find the user's voice channel
        let channel_id = guild_id.to_guild_cached(&ctx.cache).and_then(|guild| {
            guild
                .voice_states
                .get(&command.user.id)
                .and_then(|vs| vs.channel_id)
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
                    .title("Nora Voice Active")
                    .description(
                        "I've joined the voice channel. I'll listen and respond when spoken to.",
                    )
                    .color(0x3B82F6)
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
                        .title("Nora - Meeting Ended")
                        .color(0x3B82F6)
                        .field("Duration", format!("{}s", meeting.duration_seconds), true)
                        .field("Segments", format!("{}", meeting.segment_count), true)
                        .field(
                            "Participants",
                            format!("{}", meeting.participant_count),
                            true,
                        )
                        .description("Meeting transcript saved to dashboard. Notes generated.")
                        .timestamp(serenity::model::Timestamp::now())
                } else {
                    CreateEmbed::new()
                        .title("Nora - Left Voice")
                        .description("Left the voice channel.")
                        .color(0x3B82F6)
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
