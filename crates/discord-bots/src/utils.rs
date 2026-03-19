use serenity::all::CreateEmbed;

/// Split a message into chunks that fit within Discord's character limit
pub fn split_message(message: &str, max_len: usize) -> Vec<String> {
    if message.len() <= max_len {
        return vec![message.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = message;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }

        // Try to split at a newline or space boundary
        let split_at = remaining[..max_len]
            .rfind('\n')
            .or_else(|| remaining[..max_len].rfind(' '))
            .unwrap_or(max_len);

        chunks.push(remaining[..split_at].to_string());
        remaining = remaining[split_at..].trim_start();
    }

    chunks
}

/// Create a consistent embed for agent responses
pub fn format_agent_embed(
    agent_name: &str,
    message: &str,
    color: u32,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
) -> CreateEmbed {
    // Discord embed description limit is 4096
    let description = if message.len() > 4000 {
        format!("{}...", &message[..3997])
    } else {
        message.to_string()
    };

    let mut embed = CreateEmbed::new()
        .title(agent_name)
        .description(description)
        .color(color)
        .timestamp(serenity::model::Timestamp::now());

    if let (Some(input), Some(output)) = (input_tokens, output_tokens) {
        embed = embed.footer(serenity::all::CreateEmbedFooter::new(format!(
            "Tokens: {} in / {} out",
            input, output
        )));
    }

    embed
}
