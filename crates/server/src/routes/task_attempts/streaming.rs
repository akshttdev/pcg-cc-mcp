use super::*;

#[axum::debug_handler]
pub async fn stream_follow_up_draft_ws(
    ws: WebSocketUpgrade,
    Extension(task_attempt): Extension<TaskAttempt>,
    State(deployment): State<DeploymentImpl>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = handle_follow_up_draft_ws(socket, deployment, task_attempt.id).await {
            tracing::warn!("follow-up draft WS closed: {}", e);
        }
    })
}

async fn handle_follow_up_draft_ws(
    socket: WebSocket,
    deployment: DeploymentImpl,
    task_attempt_id: uuid::Uuid,
) -> anyhow::Result<()> {
    use futures_util::{SinkExt, StreamExt, TryStreamExt};

    let mut stream = deployment
        .events()
        .stream_follow_up_draft_for_attempt_raw(task_attempt_id)
        .await?
        .map_ok(|msg| msg.to_ws_message_unchecked());

    // Split socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Drain (and ignore) any client->server messages so pings/pongs work
    tokio::spawn(async move { while let Some(Ok(_)) = receiver.next().await {} });

    // Forward server messages
    while let Some(item) = stream.next().await {
        match item {
            Ok(msg) => {
                if sender.send(msg).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                tracing::error!("stream error: {}", e);
                break;
            }
        }
    }
    Ok(())
}
