use async_nats;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to NATS relay
    let client = async_nats::connect("nats://nonlocal.info:4222").await?;

    println!("Connected to NATS relay");

    // Send direct message to Omega 1 (apn_9c47c2fb)
    let omega_dm_subject = "apn.dm.apn_9c47c2fb";

    let message = json!({
        "from": "apn_09465b95",
        "type": "greeting",
        "content": "Hello Omega 1! This is a test message from apn_09465b95. Can you read me?"
    });

    let payload = serde_json::to_vec(&message)?;

    println!("Sending message to Omega 1 at {}", omega_dm_subject);
    client.publish(omega_dm_subject.to_string(), payload.into()).await?;

    // Also broadcast on discovery channel
    println!("Broadcasting on apn.discovery");
    client.publish("apn.discovery".to_string(), serde_json::to_vec(&json!({
        "node_id": "apn_09465b95",
        "wallet_address": "0x09465b9572fb354fdf4e34040386f180d1ff0c2a3a668333bedee17b266a4b74",
        "capabilities": ["compute", "relay", "storage"],
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "message": "Hello from Pythia's node!"
    }))?.into()).await?;

    println!("✅ Messages sent successfully!");

    Ok(())
}
