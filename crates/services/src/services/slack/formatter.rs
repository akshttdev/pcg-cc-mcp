//! Block Kit message templates for each `SlackEventType`.
//!
//! Each formatter takes the raw event payload (provided by the caller — a
//! `serde_json::Value` shaped per event type) and returns
//! `(text_fallback, blocks_json)`.
//!
//! Payload contracts (best-effort — missing fields gracefully degrade):
//!
//!   deal_stage_changed:   { deal_name, stage_name, deal_value_usd?, owner?, deal_url? }
//!   deal_won / deal_lost: { deal_name, deal_value_usd?, owner?, deal_url? }
//!   proposal_approved:    { client_name, deal_value_usd?, deliverables_count?, project_url? }
//!   proposal_rejected:    { client_name, reason? }
//!   task_assigned:        { task_title, assignee?, project?, task_url? }
//!   task_completed:       { task_title, project?, completed_by? }
//!   invoice_paid:         { invoice_number, amount_usd, client_name?, month_total_usd? }
//!   agent_escalation:     { agent_name, task_title, blocked_reason?, review_url? }

use db::models::slack_channel_route::SlackEventType;
use serde_json::{json, Value};

/// Pretty-print a USD amount: `$24,000` (no decimals if whole, else `$24,000.50`).
fn fmt_usd(amount: f64) -> String {
    if (amount - amount.round()).abs() < f64::EPSILON {
        format!(
            "${}",
            (amount as i64)
                .to_string()
                .as_bytes()
                .rchunks(3)
                .rev()
                .map(std::str::from_utf8)
                .collect::<Result<Vec<&str>, _>>()
                .map(|v| v.join(","))
                .unwrap_or_else(|_| (amount as i64).to_string()),
        )
    } else {
        format!("${amount:.2}")
    }
}

fn section(text: &str) -> Value {
    json!({
        "type": "section",
        "text": { "type": "mrkdwn", "text": text },
    })
}

fn link_actions(label: &str, url: &str) -> Value {
    json!({
        "type": "actions",
        "elements": [{
            "type": "button",
            "text": { "type": "plain_text", "text": label },
            "url": url,
        }],
    })
}

/// Top-level dispatch: pick the right template for the event type.
/// Returns `(text_fallback, blocks)` where `blocks` is a JSON array.
pub fn format_event(event_type: SlackEventType, payload: &Value) -> (String, Value) {
    match event_type {
        SlackEventType::DealStageChanged => deal_stage_changed(payload),
        SlackEventType::DealWon => deal_outcome(payload, "🏆", "won"),
        SlackEventType::DealLost => deal_outcome(payload, "❌", "lost"),
        SlackEventType::ProposalApproved => proposal_approved(payload),
        SlackEventType::ProposalRejected => proposal_rejected(payload),
        SlackEventType::TaskAssigned => task_assigned(payload),
        SlackEventType::TaskCompleted => task_completed(payload),
        SlackEventType::InvoicePaid => invoice_paid(payload),
        SlackEventType::AgentEscalation => agent_escalation(payload),
    }
}

// ─── Templates ──────────────────────────────────────────────────────────────

fn deal_stage_changed(p: &Value) -> (String, Value) {
    let deal_name = p["deal_name"].as_str().unwrap_or("Untitled deal");
    let stage_name = p["stage_name"].as_str().unwrap_or("(unknown stage)");
    let value = p["deal_value_usd"].as_f64();
    let owner = p["owner"].as_str();
    let url = p["deal_url"].as_str();

    let mut lines = vec![format!("🎯 *{deal_name}* moved to *{stage_name}*")];
    if let Some(v) = value {
        lines.push(format!("Deal value: {}", fmt_usd(v)));
    }
    if let Some(o) = owner {
        lines.push(format!("Owner: {o}"));
    }
    let body = lines.join("\n");
    let text = format!("{deal_name} → {stage_name}");

    let mut blocks = vec![section(&body)];
    if let Some(u) = url {
        blocks.push(link_actions("View deal →", u));
    }
    (text, Value::Array(blocks))
}

fn deal_outcome(p: &Value, emoji: &str, verb: &str) -> (String, Value) {
    let deal_name = p["deal_name"].as_str().unwrap_or("Untitled deal");
    let value = p["deal_value_usd"].as_f64();
    let owner = p["owner"].as_str();
    let url = p["deal_url"].as_str();

    let mut lines = vec![format!("{emoji} *{deal_name}* — deal *{verb}*")];
    if let Some(v) = value {
        lines.push(format!("Value: {}", fmt_usd(v)));
    }
    if let Some(o) = owner {
        lines.push(format!("Owner: {o}"));
    }
    let body = lines.join("\n");
    let text = format!("{deal_name} {verb}");

    let mut blocks = vec![section(&body)];
    if let Some(u) = url {
        blocks.push(link_actions("View deal →", u));
    }
    (text, Value::Array(blocks))
}

fn proposal_approved(p: &Value) -> (String, Value) {
    let client = p["client_name"].as_str().unwrap_or("Client");
    let value = p["deal_value_usd"].as_f64();
    let deliverables = p["deliverables_count"].as_i64();
    let url = p["project_url"].as_str();

    let mut detail = String::new();
    if let Some(v) = value {
        detail.push_str(&fmt_usd(v));
    }
    if let Some(d) = deliverables {
        if !detail.is_empty() {
            detail.push_str(" · ");
        }
        detail.push_str(&format!("{d} deliverable{}", if d == 1 { "" } else { "s" }));
    }

    let mut lines = vec![format!("✅ *{client}* approved proposal")];
    if !detail.is_empty() {
        lines.push(detail);
    }
    lines.push("Project created automatically".into());
    let body = lines.join("\n");
    let text = format!("{client} approved proposal");

    let mut blocks = vec![section(&body)];
    if let Some(u) = url {
        blocks.push(link_actions("View project →", u));
    }
    (text, Value::Array(blocks))
}

fn proposal_rejected(p: &Value) -> (String, Value) {
    let client = p["client_name"].as_str().unwrap_or("Client");
    let reason = p["reason"].as_str();
    let body = match reason {
        Some(r) => format!("🛑 *{client}* rejected proposal\nReason: {r}"),
        None => format!("🛑 *{client}* rejected proposal"),
    };
    (
        format!("{client} rejected proposal"),
        Value::Array(vec![section(&body)]),
    )
}

fn task_assigned(p: &Value) -> (String, Value) {
    let title = p["task_title"].as_str().unwrap_or("(untitled task)");
    let assignee = p["assignee"].as_str();
    let project = p["project"].as_str();
    let url = p["task_url"].as_str();

    let mut lines = vec![format!("📋 New task: *{title}*")];
    if let Some(a) = assignee {
        lines.push(format!("Assigned to: {a}"));
    }
    if let Some(pr) = project {
        lines.push(format!("Project: {pr}"));
    }
    let body = lines.join("\n");
    let text = format!("Task assigned: {title}");

    let mut blocks = vec![section(&body)];
    if let Some(u) = url {
        blocks.push(link_actions("Open task →", u));
    }
    (text, Value::Array(blocks))
}

fn task_completed(p: &Value) -> (String, Value) {
    let title = p["task_title"].as_str().unwrap_or("(untitled task)");
    let project = p["project"].as_str();
    let by = p["completed_by"].as_str();

    let mut lines = vec![format!("✅ Completed: *{title}*")];
    if let Some(b) = by {
        lines.push(format!("By: {b}"));
    }
    if let Some(pr) = project {
        lines.push(format!("Project: {pr}"));
    }
    (
        format!("Task completed: {title}"),
        Value::Array(vec![section(&lines.join("\n"))]),
    )
}

fn invoice_paid(p: &Value) -> (String, Value) {
    let number = p["invoice_number"].as_str().unwrap_or("?");
    let amount = p["amount_usd"].as_f64().unwrap_or(0.0);
    let client = p["client_name"].as_str();
    let month_total = p["month_total_usd"].as_f64();

    let mut lines = vec![format!("💰 Invoice #{number} paid")];
    let mut second = fmt_usd(amount);
    if let Some(c) = client {
        second.push_str(" · ");
        second.push_str(c);
    }
    lines.push(second);
    if let Some(t) = month_total {
        lines.push(format!("Running total this month: {}", fmt_usd(t)));
    }
    (
        format!("Invoice #{number} paid ({})", fmt_usd(amount)),
        Value::Array(vec![section(&lines.join("\n"))]),
    )
}

fn agent_escalation(p: &Value) -> (String, Value) {
    let agent = p["agent_name"].as_str().unwrap_or("An agent");
    let task = p["task_title"].as_str().unwrap_or("(unspecified task)");
    let blocked = p["blocked_reason"].as_str();
    let url = p["review_url"].as_str();

    let mut lines = vec![format!("⚠️ *{agent}* needs help")];
    lines.push(format!("Task: {task}"));
    if let Some(b) = blocked {
        lines.push(format!("Blocked: {b}"));
    }
    let body = lines.join("\n");
    let text = format!("{agent} escalation: {task}");

    let mut blocks = vec![section(&body)];
    if let Some(u) = url {
        blocks.push(link_actions("Review →", u));
    }
    (text, Value::Array(blocks))
}
