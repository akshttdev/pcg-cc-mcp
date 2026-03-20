#![allow(clippy::uninlined_format_args)]

pub mod backend;
pub mod config;
pub mod nora_bot;
pub mod topsi_bot;
pub mod utils;
pub mod voice;

use std::sync::Arc;

use backend::BackendClient;
use config::BotConfig;
use nora_bot::NoraHandler;
use serenity::all::*;
use songbird::SerenityInit;
use topsi_bot::TopsiHandler;
use tracing::{error, info};
use voice::{VoiceManager, VoiceServiceConfig};

/// Spawn Discord bots (Nora + Topsi) as background tasks on the current tokio runtime.
/// Returns immediately. Bots run until the runtime shuts down.
/// Silently skips if no bot tokens are configured.
pub fn spawn_discord_bots() {
    let config = match BotConfig::from_env() {
        Ok(c) => Arc::new(c),
        Err(_) => {
            info!("[DISCORD] No bot tokens configured, skipping Discord bot startup");
            return;
        }
    };

    let backend = Arc::new(BackendClient::new(
        config.backend_url.clone(),
        config.backend_auth_token.clone(),
    ));
    let voice_config = VoiceServiceConfig::default();

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    // Spawn Nora bot
    if let Some(token) = &config.nora_token {
        let token = token.clone();
        let backend = backend.clone();
        let config = config.clone();
        let voice_config = voice_config.clone();

        tokio::spawn(async move {
            let nora_voice = Arc::new(VoiceManager::new(backend.clone(), "Nora", voice_config));
            let handler = NoraHandler::new(backend, config, nora_voice);

            let mut client = match Client::builder(&token, intents)
                .event_handler(handler)
                .register_songbird()
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    error!("[DISCORD] Failed to create Nora client: {}", e);
                    return;
                }
            };

            info!("[DISCORD] Starting Nora bot (text + voice)...");
            if let Err(e) = client.start().await {
                error!("[DISCORD] Nora bot error: {}", e);
            }
        });
    }

    // Spawn Topsi bot
    if let Some(token) = &config.topsi_token {
        let token = token.clone();
        let backend = backend.clone();
        let config = config.clone();
        let voice_config = voice_config.clone();

        tokio::spawn(async move {
            let topsi_voice = Arc::new(VoiceManager::new(backend.clone(), "Topsi", voice_config));
            let handler = TopsiHandler::new(backend, config, topsi_voice);

            let mut client = match Client::builder(&token, intents)
                .event_handler(handler)
                .register_songbird()
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    error!("[DISCORD] Failed to create Topsi client: {}", e);
                    return;
                }
            };

            info!("[DISCORD] Starting Topsi bot (text + voice)...");
            if let Err(e) = client.start().await {
                error!("[DISCORD] Topsi bot error: {}", e);
            }
        });
    }

    info!("[DISCORD] Discord bot agents spawned");
}
