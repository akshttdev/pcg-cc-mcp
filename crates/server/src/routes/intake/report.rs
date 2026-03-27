//! Stage 1 (Claude extraction), Stage 5a (company research passes),
//! and Stage 6 (comprehensive report generation).

use db::models::{
    business_report::{BusinessReport, CreateBusinessReport},
    call_intake_item::CallIntakeItem,
    person::Person,
};
use reqwest::Client;
use serde_json::Value;
use tracing::{info, warn};
use uuid::Uuid;

use super::{
    ExtractedBusiness, ExtractedIndividual, ExtractedIntake, GeneratedReport,
    pipeline::advance_deal_stage,
};

// ── Stage 1: Extract structure ────────────────────────────────────────────────

/// Email classification result — determines how an incoming email is handled.
#[derive(Debug, PartialEq)]
pub(crate) enum EmailClass {
    /// A new discovery call or new lead introduction — run full auto-pipeline
    DiscoveryCall,
    /// An update about an existing client relationship — log only, no new pipeline
    OngoingClient,
    /// Not actionable (marketing, notification, admin) — skip
    Other,
}

/// Classify an inbound email from a trusted sender (e.g. sirak@sirakstudios.com)
/// using a lightweight Claude call. Returns in ~1-2s.
pub(crate) async fn classify_email(subject: &str, body: &str) -> EmailClass {
    let api_key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(k) => k,
        Err(_) => return EmailClass::Other,
    };

    let prompt = format!(
        r#"Classify this email from a client into exactly one category.

Subject: {}
Body (first 2000 chars): {}

Reply with ONLY one of these words — nothing else:
- DISCOVERY  (a new prospective client or discovery call being shared)
- ONGOING    (an update, follow-up, or note about an existing client relationship)
- OTHER      (administrative, notifications, marketing, or unrelated)

Classification:"#,
        subject,
        &body[..body.len().min(2000)]
    );

    let client = reqwest::Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 10,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await;

    let label = match res {
        Ok(r) => r
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|b| b["content"][0]["text"].as_str().map(|s| s.trim().to_uppercase()))
            .unwrap_or_default(),
        Err(_) => return EmailClass::Other,
    };

    if label.contains("DISCOVERY") {
        EmailClass::DiscoveryCall
    } else if label.contains("ONGOING") {
        EmailClass::OngoingClient
    } else {
        EmailClass::Other
    }
}

pub(super) async fn extract_intake_structure(raw: &str) -> anyhow::Result<ExtractedIntake> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let system = "You are an expert business analyst specialising in extracting structured data \
        from call transcripts, meeting notes, and email summaries. Always respond with valid JSON only — \
        no markdown fences, no explanation.";

    let prompt = format!(
        r#"Analyse this call/meeting content and return a JSON object with these exact fields:

{{
  "participants": [
    {{
      "name": "string",
      "email": "string or null",
      "company": "string or null",
      "role": "string or null — their job title or role in the call",
      "is_prospect": true/false — true if they appear to be a client/prospect (not PCG/Powerclub team)
    }}
  ],
  "individuals": [
    {{
      "name": "full name of the individual",
      "email": "string or null",
      "company": "their current company or null",
      "role": "their job title or role or null",
      "linkedin": "LinkedIn URL if mentioned, else null",
      "is_prospect": true if client/prospect, false if PCG team member,
      "notes": "any specific notes about this person from the call or null"
    }}
  ],
  "businesses": [
    {{
      "name": "company/brand name",
      "website": "URL if mentioned, else null",
      "industry": "industry sector or null",
      "description": "1-2 sentence description of the business based on context",
      "size_estimate": "e.g. 'startup', 'SMB', 'enterprise', 'solopreneur' or null"
    }}
  ],
  "call_date": "ISO date string (YYYY-MM-DD) if mentioned, else null",
  "call_summary": "2-4 sentence summary of what was discussed",
  "topics": ["array", "of", "main", "topics"],
  "pain_points": ["specific pain points, challenges, or problems the prospect mentioned"],
  "action_items": [
    {{
      "action": "what needs to be done",
      "owner": "who is responsible (name or null)",
      "deadline": "deadline if mentioned, else null"
    }}
  ],
  "sentiment": "positive | neutral | negative",
  "opportunity_signals": ["signals that indicate business opportunity for PCG/Powerclub Global"]
}}

Important: The "individuals" array should include EVERY person mentioned (not just those on the call).
The "businesses" array should include EVERY company/brand discussed.
Fill in as much detail as you can infer from context.

CONTENT:
{}
"#,
        &raw[..raw.len().min(15000)]
    );

    let client = Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 4096,
            "system": system,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await?;

    let body: Value = res.json().await?;
    let text = body["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No text in Claude response: {:?}", body))?;

    // Strip markdown fences if present
    let json_str = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let extracted: ExtractedIntake = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse extraction JSON: {e}\nRaw: {json_str}"))?;

    Ok(extracted)
}

// ── Stage 5a: Company research passes ────────────────────────────────────────

/// Run one research pass on a company using Claude with web search.
pub(super) async fn run_company_research_pass(
    pool: &sqlx::SqlitePool,
    company_name: &str,
    intake_item_id: Uuid,
    pass_number: u32,
) -> anyhow::Result<()> {
    let focus = match pass_number {
        1 => "identity_and_overview",
        2 => "market_position_and_competitors",
        _ => "digital_presence_and_opportunities",
    };

    info!(
        "Company research pass {} ({}) for '{}'",
        pass_number, focus, company_name
    );

    let pass_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO company_research_passes
         (id, company_name, intake_item_id, pass_number, research_focus, status)
         VALUES (?, ?, ?, ?, ?, 'running')",
    )
    .bind(pass_id)
    .bind(company_name)
    .bind(intake_item_id)
    .bind(pass_number as i64)
    .bind(focus)
    .execute(pool)
    .await?;

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let focus_prompt = match focus {
        "identity_and_overview" => format!(
            "Research '{company_name}': What do they do? What industry are they in? Who founded it? \
             What is their size, location, and key offerings? Find their website and social media."
        ),
        "market_position_and_competitors" => format!(
            "Research '{company_name}' market position: Who are their main competitors? \
             What is their unique value proposition? How do they differentiate? \
             What market segment do they serve? What are their strengths and weaknesses?"
        ),
        _ => format!(
            "Research '{company_name}' digital presence and business opportunities: \
             What is their web presence like? Social media activity? Content strategy? \
             What business challenges might they face where a creative agency could help? \
             What are potential growth opportunities?"
        ),
    };

    let messages = vec![serde_json::json!({
        "role": "user",
        "content": format!(
            "{focus_prompt}\n\nReturn a JSON object: \
            {{\"summary\": \"3-5 sentence overview\", \
              \"key_findings\": {{\"relevant_details\": \"...\"}}, \
              \"sources\": [{{\"title\": \"...\", \"url\": \"...\"}}], \
              \"confidence_score\": 0.0-1.0}}"
        )
    })];

    let client = Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "web-search-2025-03-05")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 4096,
            "tools": [{
                "type": "web_search_20250305",
                "name": "web_search",
                "max_uses": 4
            }],
            "messages": messages
        }))
        .send()
        .await?;

    let body: serde_json::Value = res.json().await?;

    // Extract text content from response (may include tool use blocks)
    let text = body["content"]
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|b| b["type"].as_str() == Some("text"))
                .and_then(|b| b["text"].as_str())
        })
        .unwrap_or("{}");

    let json_str = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap_or_else(|_| {
        serde_json::json!({"summary": text, "key_findings": {}, "sources": [], "confidence_score": 0.3})
    });

    let summary = parsed["summary"].as_str().unwrap_or("").to_string();
    let key_findings =
        serde_json::to_string(&parsed["key_findings"]).unwrap_or_else(|_| "{}".into());
    let sources = serde_json::to_string(&parsed["sources"]).unwrap_or_else(|_| "[]".into());
    let confidence = parsed["confidence_score"].as_f64().unwrap_or(0.5);

    sqlx::query(
        "UPDATE company_research_passes SET
         status = 'done', summary = ?, key_findings = ?, sources = ?,
         confidence_score = ?, agent_used = 'claude-sonnet',
         updated_at = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(&summary)
    .bind(&key_findings)
    .bind(&sources)
    .bind(confidence)
    .bind(pass_id)
    .execute(pool)
    .await?;

    info!(
        "Company research pass {} complete for '{}'",
        pass_number, company_name
    );
    Ok(())
}

// ── Stage 6: Report generation ────────────────────────────────────────────────

pub async fn run_report_generation(
    pool: sqlx::SqlitePool,
    person_id: Uuid,
    report_type: String,
    businesses: Vec<ExtractedBusiness>,
    individuals: Vec<ExtractedIndividual>,
    primary_intake_id: Uuid,
    crm_deal_id: Option<Uuid>,
) -> anyhow::Result<()> {
    info!("Generating {} report for person {}", report_type, person_id);

    // Load person
    let person = Person::find_by_id(&pool, person_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Person not found"))?;

    // Load all intake items for this person
    let intake_items = CallIntakeItem::list_by_person(&pool, person_id).await?;

    if intake_items.is_empty() {
        warn!("No intake items for person {} — skipping report", person_id);
        return Ok(());
    }

    // Gather call transcripts / summaries
    let mut call_context = String::new();
    let mut intake_ids: Vec<String> = Vec::new();

    for item in &intake_items {
        intake_ids.push(item.id.to_string());
        let summary = item.call_summary.as_deref().unwrap_or("");
        let topics: Vec<String> = serde_json::from_str(&item.extracted_topics).unwrap_or_default();
        let pain_points: Vec<String> =
            serde_json::from_str(&item.extracted_pain_points).unwrap_or_default();

        call_context.push_str(&format!(
            "\n--- Call/Email ({}) ---\nSummary: {}\nTopics: {}\nPain Points: {}\n",
            item.call_date.as_deref().unwrap_or("unknown date"),
            summary,
            topics.join(", "),
            pain_points.join("; "),
        ));

        // Also include raw content excerpt
        if let Some(raw) = &item.raw_content {
            call_context.push_str(&format!(
                "Transcript excerpt:\n{}\n",
                &raw[..raw.len().min(2000)]
            ));
        }
    }

    // Load company research results for context
    let mut company_research_context = String::new();
    for biz in &businesses {
        let passes: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
            "SELECT research_focus, summary FROM company_research_passes
             WHERE company_name = ? AND status = 'done'
             ORDER BY pass_number ASC LIMIT 5",
        )
        .bind(&biz.name)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        if !passes.is_empty() {
            company_research_context
                .push_str(&format!("\n\n=== Company Research: {} ===\n", biz.name));
            for (focus, summary) in &passes {
                company_research_context.push_str(&format!("[{}] {}\n", focus, summary));
            }
        }

        // Gather sources from research passes
        let sources_json: Vec<serde_json::Value> = sqlx::query_as::<_, (String,)>(
            "SELECT sources FROM company_research_passes
             WHERE company_name = ? AND status = 'done'",
        )
        .bind(&biz.name)
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .flat_map(|(s,)| serde_json::from_str::<Vec<serde_json::Value>>(&s).unwrap_or_default())
        .collect();

        if !sources_json.is_empty() {
            company_research_context.push_str(&format!(
                "Sources: {}\n",
                sources_json
                    .iter()
                    .filter_map(|s| s["url"].as_str().map(|u| u.to_string()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    // Intelligence summary
    let intel_summary = person
        .intelligence_summary
        .as_deref()
        .unwrap_or("No prior research available.");
    let person_name = &person.full_name;
    let company = person.company_name.as_deref().unwrap_or(
        businesses
            .first()
            .map(|b| b.name.as_str())
            .unwrap_or("Unknown company"),
    );

    let biz_list = businesses
        .iter()
        .map(|b| {
            format!(
                "- {} ({}): {}",
                b.name,
                b.website.as_deref().unwrap_or("no website"),
                b.description.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let ind_list = individuals
        .iter()
        .map(|i| {
            format!(
                "- {} | {} at {} | {}",
                i.name,
                i.role.as_deref().unwrap_or("unknown role"),
                i.company.as_deref().unwrap_or("unknown company"),
                i.notes.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let generated = generate_report_with_claude(
        person_name,
        company,
        intel_summary,
        &call_context,
        &company_research_context,
        &biz_list,
        &ind_list,
        &report_type,
    )
    .await?;

    // Collect all sources from company research passes
    let all_sources: Vec<serde_json::Value> = sqlx::query_as::<_, (String,)>(
        "SELECT sources FROM company_research_passes WHERE intake_item_id = ? AND status = 'done'",
    )
    .bind(primary_intake_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .flat_map(|(s,)| serde_json::from_str::<Vec<serde_json::Value>>(&s).unwrap_or_default())
    .collect();

    let mut merged_sources = generated.sources.clone();
    merged_sources.extend(all_sources);

    // Resolve the person's primary company for the report
    #[derive(sqlx::FromRow)]
    struct CompanyIdRow {
        company_id: Option<Uuid>,
    }
    let company_id: Option<Uuid> = sqlx::query_as::<_, CompanyIdRow>(
        "SELECT company_id FROM person_company_roles WHERE person_id = ? AND is_primary = 1 LIMIT 1",
    )
    .bind(person_id)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten()
    .and_then(|r| r.company_id);

    // Store report
    let report = BusinessReport::create(
        &pool,
        CreateBusinessReport {
            person_id: Some(person_id),
            company_id,
            report_type: Some(report_type.clone()),
            title: generated.title.clone(),
            executive_summary: Some(generated.executive_summary),
            company_overview: Some(generated.company_overview),
            pain_points: Some(
                serde_json::to_string(&generated.pain_points).unwrap_or_else(|_| "[]".into()),
            ),
            opportunities: Some(
                serde_json::to_string(&generated.opportunities).unwrap_or_else(|_| "[]".into()),
            ),
            recommended_services: Some(
                serde_json::to_string(&generated.recommended_services)
                    .unwrap_or_else(|_| "[]".into()),
            ),
            next_steps: Some(
                serde_json::to_string(&generated.next_steps).unwrap_or_else(|_| "[]".into()),
            ),
            full_report_md: Some(generated.full_report_md),
            individual_profiles: Some(
                serde_json::to_string(&generated.individual_profiles)
                    .unwrap_or_else(|_| "[]".into()),
            ),
            market_analysis: Some(generated.market_analysis),
            competitor_analysis: Some(
                serde_json::to_string(&generated.competitor_analysis)
                    .unwrap_or_else(|_| "[]".into()),
            ),
            target_clients: Some(generated.target_clients),
            brand_positioning: Some(generated.brand_positioning),
            digital_presence: Some(generated.digital_presence),
            sources: Some(serde_json::to_string(&merged_sources).unwrap_or_else(|_| "[]".into())),
            intake_item_ids: Some(
                serde_json::to_string(&intake_ids).unwrap_or_else(|_| "[]".into()),
            ),
            call_log_ids: Some("[]".into()),
            created_by: None,
        },
    )
    .await?;

    // Link crm_deal_id to the report
    if let Some(deal_id) = crm_deal_id {
        let _ = sqlx::query("UPDATE business_reports SET crm_deal_id = ? WHERE id = ?")
            .bind(deal_id)
            .bind(report.id.as_str())
            .execute(&pool)
            .await;
    }

    let report_id_str = report.id.to_string();
    BusinessReport::mark_ready(
        &pool,
        uuid::Uuid::parse_str(&report_id_str).unwrap_or_default(),
    )
    .await?;

    // Link report back to intake items
    for intake_id in &intake_ids {
        if let Ok(iid) = Uuid::parse_str(intake_id) {
            let _ = sqlx::query(
                "UPDATE call_intake_items SET report_id = ?, updated_at = datetime('now','subsec') WHERE id = ?",
            )
            .bind(&report_id_str)
            .bind(iid)
            .execute(&pool)
            .await;
        }
    }

    // Advance CRM deal to "Analysis Done" now that report is complete
    if let Some(deal_id) = crm_deal_id {
        advance_deal_stage(&pool, deal_id, "Analysis Done").await;
    }

    // Ingest report sources into org-scoped knowledge graph
    ingest_sources_into_kg(&pool, person_id, &merged_sources).await;

    // Create human-review task: "Review Business Report: [person]"
    {
        let person_name = sqlx::query_scalar::<_, String>("SELECT full_name FROM persons WHERE id = ?")
            .bind(person_id)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "Unknown".into());

        let title = format!("Review Business Report: {}", person_name);
        let desc = format!(
            "Business report is ready for review.\nReport ID: {}\n\
             Review the executive summary, market analysis, and recommended services.\n\
             Then advance the deal stage and create a proposal if appropriate.",
            report_id_str
        );
        let project_id = super::pipeline::SIRAK_CONFIG.project_id;
        let exists: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM tasks WHERE project_id = ? AND title = ?",
        )
        .bind(project_id)
        .bind(&title)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(false);

        if !exists {
            let _ = sqlx::query(
                "INSERT INTO tasks (id, project_id, title, description, status, priority,
                 created_by, tags, workflow_type, entity_type, entity_id,
                 created_at, updated_at)
                 SELECT ?, ?, ?, ?, 'todo', 'high', id, '[\"report-review\",\"intel\"]',
                 'report_review', 'person', ?
                 FROM users WHERE username = 'Sirak' LIMIT 1",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(project_id)
            .bind(&title)
            .bind(&desc)
            .bind(&report_id_str)
            .execute(&pool)
            .await;
            tracing::info!("Created report review task: {}", title);
        }
    }

    info!("Report {} created for person {}", report_id_str, person_id);
    Ok(())
}

/// Register research source URLs in the org-scoped knowledge graph.
async fn ingest_sources_into_kg(
    pool: &sqlx::SqlitePool,
    person_id: Uuid,
    sources: &[serde_json::Value],
) {
    // Find which org this person belongs to
    #[derive(sqlx::FromRow)]
    struct Row {
        organization_id: Uuid,
    }

    let org_id = sqlx::query_as::<_, Row>(
        "SELECT organization_id FROM person_organization_contacts WHERE person_id = ? LIMIT 1",
    )
    .bind(person_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|r| r.organization_id);

    if let Some(org_id) = org_id {
        for src in sources {
            let url = match src["url"].as_str() {
                Some(u) if u.starts_with("http") => u,
                _ => continue,
            };
            let title = src["title"].as_str().unwrap_or(url);
            let excerpt = src["excerpt"].as_str().unwrap_or("");
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO project_knowledge_sources
                 (id, owner_type, owner_id, source_type, source_id, source_title,
                  source_summary, coverage_score, is_active, is_stale,
                  created_at, updated_at)
                 VALUES (?, 'organization', ?, 'web_page', ?, ?, ?, 0.6, 1, 0,
                  datetime('now','subsec'), datetime('now','subsec'))",
            )
            .bind(Uuid::new_v4())
            .bind(org_id)
            .bind(url)
            .bind(title)
            .bind(excerpt)
            .execute(pool)
            .await;
        }
    }
}

async fn generate_report_with_claude(
    person_name: &str,
    company: &str,
    intel_summary: &str,
    call_context: &str,
    company_research_context: &str,
    biz_list: &str,
    ind_list: &str,
    report_type: &str,
) -> anyhow::Result<GeneratedReport> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let system = "You are a senior business development analyst at Powerclub Global, \
        a creative agency specialising in media production, brand strategy, talent management, \
        and digital content. Your job is to analyse prospect/client intelligence and generate \
        comprehensive, actionable business audit reports with deep analytics. \
        Always respond with valid JSON only — no markdown fences, no explanation outside the JSON.";

    let prompt = format!(
        r#"Generate a comprehensive {report_type} analytics report for:
Primary Contact: {person_name}
Primary Company: {company}

--- IDENTIFIED INDIVIDUALS ---
{ind_list}

--- IDENTIFIED BUSINESSES ---
{biz_list}

--- INTELLIGENCE RESEARCH ---
{intel_summary}

--- COMPANY RESEARCH (multi-pass) ---
{company_research_context}

--- CALL/MEETING CONTEXT ---
{call_context}

Return a JSON object with these EXACT fields (all required):
{{
  "title": "string — concise report title e.g. 'Business Analytics: Acme Corp'",
  "executive_summary": "string — 3-5 sentences summarising key findings and opportunity for PCG",
  "company_overview": "string — 3-5 sentences about the prospect company based on all research",
  "individual_profiles": [
    {{
      "name": "person's full name",
      "role": "their title/role",
      "company": "their company",
      "linkedin": "LinkedIn URL or null",
      "summary": "2-3 sentence profile of this individual",
      "key_insights": "what is most relevant about this person for our engagement"
    }}
  ],
  "market_analysis": "string — 3-6 sentence markdown prose on their market position, size, trends, growth trajectory",
  "competitor_analysis": [
    {{
      "name": "competitor name",
      "website": "URL or null",
      "strengths": "what they do well",
      "weaknesses": "where they fall short",
      "threat_level": "high|medium|low"
    }}
  ],
  "target_clients": "string — 2-4 sentence markdown prose on who THEIR ideal customers are",
  "brand_positioning": "string — 2-4 sentence markdown prose on how they position themselves in the market",
  "digital_presence": "string — 2-4 sentence markdown prose on their web, social, SEO, content presence",
  "pain_points": [
    {{"point": "specific pain point", "severity": "high|medium|low"}}
  ],
  "opportunities": [
    {{
      "title": "opportunity title",
      "description": "2-3 sentences explaining the opportunity for PCG",
      "priority": "high|medium|low",
      "estimated_value": "e.g. '$5k-15k project' or 'ongoing retainer'"
    }}
  ],
  "recommended_services": [
    {{
      "name": "PCG service name",
      "rationale": "why this service fits this prospect",
      "timeline": "e.g. 'Q2 2026' or '30-60 days'"
    }}
  ],
  "next_steps": [
    {{
      "action": "specific action",
      "owner": "PCG team member or role",
      "deadline": "timeline string"
    }}
  ],
  "sources": [
    {{
      "title": "page/article title",
      "url": "full URL",
      "excerpt": "1-2 sentence relevant excerpt or null"
    }}
  ],
  "full_report_md": "string — complete narrative report in markdown, 800+ words, with sections: \
    Executive Summary, Individual Profiles, Company Overview, Market Analysis, \
    Competitive Landscape, Brand Positioning, Digital Presence, Pain Points, \
    Opportunity Analysis, Recommended Services, Next Steps, Conclusion"
}}
"#
    );

    let client = Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-opus-4-6",
            "max_tokens": 8192,
            "system": system,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await?;

    let body: Value = res.json().await?;
    let text = body["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No text in Claude response: {:?}", body))?;

    let json_str = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let report: GeneratedReport = serde_json::from_str(json_str).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse report JSON: {e}\nRaw: {}",
            &json_str[..json_str.len().min(500)]
        )
    })?;

    Ok(report)
}

// ── Phase II: Deep research from company KG intel ────────────────────────────

/// Triggered after human review approval.
/// Loads company KG intel → runs Astra deep analysis → creates BusinessReport + deliverable.
pub async fn run_phase2_from_company_intel(
    pool: sqlx::SqlitePool,
    company_id: &str,
    client_id: Option<&str>,
    deal_id: Option<&str>,
    approved_by: &str,
) -> anyhow::Result<()> {
    #[derive(sqlx::FromRow)]
    struct CompanyRow {
        name: String,
        intelligence_summary: Option<String>,
        intelligence_raw: Option<String>,
        website: Option<String>,
        industry: Option<String>,
    }

    // companies.id is BLOB — query by hex match
    let company = sqlx::query_as::<_, CompanyRow>(
        r#"SELECT name, intelligence_summary, intelligence_raw, website, industry
           FROM companies
           WHERE lower(hex(substr(id,1,4)) || '-' || hex(substr(id,5,2)) || '-' ||
                 hex(substr(id,7,2)) || '-' || hex(substr(id,9,2)) || '-' || hex(substr(id,11,6))) = ?
              OR id = ?"#,
    )
    .bind(company_id)
    .bind(company_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("Company not found: {}", company_id))?;

    let raw: serde_json::Value = company
        .intelligence_raw
        .as_deref()
        .and_then(|r| serde_json::from_str(r).ok())
        .unwrap_or_default();

    // Build research context from Phase I intel
    let intel_summary = company.intelligence_summary.as_deref().unwrap_or("No prior research.");

    let company_research_ctx = format!(
        "Industry: {}\nWebsite: {}\n\nPhase I Scout Research:\n{}",
        company.industry.as_deref().unwrap_or("Unknown"),
        company.website.as_deref().unwrap_or("Unknown"),
        serde_json::to_string_pretty(&raw).unwrap_or_default(),
    );

    let biz_list = format!(
        "- {} ({}): {}",
        company.name,
        company.website.as_deref().unwrap_or("no website"),
        company.intelligence_summary.as_deref().unwrap_or("")
    );

    // Determine primary contact name from linked client/person if available
    let person_name = if let Some(cid) = client_id {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT p.full_name FROM clients cl JOIN persons p ON p.id = cl.primary_person_id WHERE cl.id = ?"
        )
        .bind(cid)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten()
        .flatten()
        .unwrap_or_else(|| company.name.clone())
    } else {
        company.name.clone()
    };

    info!("[Phase II] Generating Astra deep research report for {}", company.name);

    let generated = generate_report_with_claude(
        &person_name,
        &company.name,
        intel_summary,
        "No call context — report triggered from KG intel approval.",
        &company_research_ctx,
        &biz_list,
        "",
        "business_audit",
    )
    .await?;

    // Create BusinessReport record
    let report = BusinessReport::create(
        &pool,
        CreateBusinessReport {
            title: generated.title.clone(),
            report_type: Some("business_audit".into()),
            executive_summary: Some(generated.executive_summary),
            company_overview: Some(generated.company_overview),
            pain_points: Some(serde_json::to_string(&generated.pain_points).unwrap_or_else(|_| "[]".into())),
            opportunities: Some(serde_json::to_string(&generated.opportunities).unwrap_or_else(|_| "[]".into())),
            recommended_services: Some(serde_json::to_string(&generated.recommended_services).unwrap_or_else(|_| "[]".into())),
            next_steps: Some(serde_json::to_string(&generated.next_steps).unwrap_or_else(|_| "[]".into())),
            full_report_md: Some(generated.full_report_md),
            individual_profiles: Some(serde_json::to_string(&generated.individual_profiles).unwrap_or_else(|_| "[]".into())),
            market_analysis: Some(generated.market_analysis),
            competitor_analysis: Some(serde_json::to_string(&generated.competitor_analysis).unwrap_or_else(|_| "[]".into())),
            target_clients: Some(generated.target_clients),
            brand_positioning: Some(generated.brand_positioning),
            digital_presence: Some(generated.digital_presence),
            sources: Some(serde_json::to_string(&generated.sources).unwrap_or_else(|_| "[]".into())),
            intake_item_ids: Some("[]".into()),
            call_log_ids: Some("[]".into()),
            company_id: None,
            person_id: None,
            created_by: None,
        },
    )
    .await?;

    let report_id_str = report.id.to_string();

    // Link to client, deal, approved_by
    let _ = sqlx::query(
        "UPDATE business_reports SET
         client_id = COALESCE(?, client_id),
         crm_deal_id = COALESCE(?, crm_deal_id),
         reviewed_by = ?,
         reviewed_at = datetime('now','subsec'),
         review_status = 'approved'
         WHERE id = ?",
    )
    .bind(client_id)
    .bind(deal_id)
    .bind(approved_by)
    .bind(&report_id_str)
    .execute(&pool)
    .await;

    let report_uuid = Uuid::parse_str(&report_id_str).unwrap_or_default();
    BusinessReport::mark_ready(&pool, report_uuid).await?;

    // Create PDF deliverable on deal's project board if deal_id is set
    if let Some(did) = deal_id {
        let project_id: Option<String> = sqlx::query_scalar(
            "SELECT project_id FROM crm_deals WHERE id = ?"
        )
        .bind(did)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten()
        .flatten();

        if let Some(pid) = project_id {
            let del_id = Uuid::new_v4().to_string();
            let pdf_url = format!("/api/business-reports/{}/pdf", report_id_str);
            let _ = sqlx::query(
                "INSERT INTO deliverables (id, project_id, title, description, status,
                 final_link, business_report_id, deliverable_type, created_at, updated_at)
                 VALUES (?, ?, ?, ?, 'done', ?, ?, 'report', datetime('now','subsec'), datetime('now','subsec'))",
            )
            .bind(&del_id)
            .bind(&pid)
            .bind(format!("Business Analysis Report: {}", company.name))
            .bind("Phase II Astra deep research — Fortune 100-level business analysis report.")
            .bind(&pdf_url)
            .bind(&report_id_str)
            .execute(&pool)
            .await;

            info!("[Phase II] Created PDF deliverable on project {}", pid);
        }
    }

    // ── Auto-pipeline task transitions ────────────────────────────────────────
    use crate::routes::intake::pipeline::SIRAK_CONFIG;

    // Mark Phase II (Astra Report) task done
    let _ = sqlx::query(
        "UPDATE tasks SET status = 'done', updated_at = datetime('now','subsec')
         WHERE workflow_type = 'phase2_astra' AND entity_id = ? AND status != 'done'",
    )
    .bind(company_id)
    .execute(&pool)
    .await;

    // Create Phase III review task assigned to the human operator (Sirak)
    let task3_id = Uuid::new_v4().to_string();
    let task3_title = format!("Review: {} Business Report", company.name);
    let task3_desc = format!(
        "Phase III — Human review of Astra deep research business analysis.\n\
         Report ID: {}\n\
         Company: {}\n\
         Action: Review the report, make edits if needed, then mark approved to close the pipeline.",
        report_id_str, company.name
    );
    let _ = sqlx::query(
        "INSERT INTO tasks (id, project_id, title, description, status, priority,
         assignee_id, created_by, tags, workflow_type, entity_type, entity_id,
         crm_deal_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, 'todo', 'high', ?, 'auto-pipeline',
         '[\"phase3\",\"review\",\"auto-pipeline\"]', 'phase3_review', 'business_report', ?,
         ?, datetime('now','subsec'), datetime('now','subsec'))",
    )
    .bind(&task3_id)
    .bind(SIRAK_CONFIG.project_id)
    .bind(&task3_title)
    .bind(&task3_desc)
    .bind(SIRAK_CONFIG.sirak_user_id)
    .bind(&report_id_str)
    .bind(deal_id)
    .execute(&pool)
    .await;

    info!(
        "[Phase II] Complete for {} — report {} — Phase III review task created",
        company.name, report_id_str
    );
    Ok(())
}
