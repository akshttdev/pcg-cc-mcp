use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "discord_bots=info,serenity=warn,songbird=warn".into()),
        )
        .init();

    info!("Starting Discord bot agents (standalone mode)...");
    discord_bots::spawn_discord_bots();

    // Keep the process alive until interrupted
    tokio::signal::ctrl_c().await?;
    info!("Shutting down Discord bots...");

    Ok(())
}
