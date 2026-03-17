use super::*;

pub async fn get_org_brand_profile(
    State(deployment): State<DeploymentImpl>,
    Extension(access_context): Extension<AccessContext>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Option<OrgBrandProfile>>>, ApiError> {
    let _ = access_context;
    let profile = OrgBrandProfile::find_by_org(&deployment.db().pool, org_id).await?;
    Ok(Json(ApiResponse::success(profile)))
}

/// PUT /api/organizations/:id/brand-profile
pub async fn upsert_org_brand_profile(
    State(deployment): State<DeploymentImpl>,
    Extension(access_context): Extension<AccessContext>,
    Path(org_id): Path<Uuid>,
    Json(body): Json<UpsertOrgBrandProfile>,
) -> Result<Json<ApiResponse<OrgBrandProfile>>, ApiError> {
    let _ = access_context;
    let profile = OrgBrandProfile::upsert(&deployment.db().pool, org_id, &body).await?;
    Ok(Json(ApiResponse::success(profile)))
}

// ── Brand Research ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BrandResearchJobResponse {
    pub org_id: Uuid,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct BrandResearchStatusResponse {
    pub org_id: Uuid,
    pub status: String,
    pub summary: Option<String>,
    pub ran_at: Option<String>,
}

/// POST /api/organizations/:id/brand-research
/// Queues an async Exa-powered brand research task.
pub async fn trigger_brand_research(
    State(deployment): State<DeploymentImpl>,
    Extension(access_context): Extension<AccessContext>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ApiResponse<BrandResearchJobResponse>>, ApiError> {
    let _ = access_context;
    let pool = &deployment.db().pool;

    // Fetch org name + existing brand profile
    let org: Option<Organization> = Organization::find_by_id(pool, &org_id.to_string()).await
        .map_err(|e| ApiError::InternalError(format!("DB error: {e}")))?;

    let org = org.ok_or_else(|| ApiError::NotFound("Organization not found".into()))?;

    // Get existing brand profile for context
    let profile = OrgBrandProfile::find_by_org(pool, org_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("{e}")))?;

    // Mark status = queued
    sqlx::query(
        "UPDATE organization_brand_profiles SET research_status = 'queued', updated_at = datetime('now','subsec') WHERE organization_id = ?",
    )
    .bind(org_id.as_bytes().as_slice())
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("{e}")))?;

    // Build research context
    let org_name = org.name.clone();
    let website = profile.as_ref().and_then(|p| p.website_url.clone()).unwrap_or_default();
    let instagram = profile.as_ref().and_then(|p| p.social_instagram.clone()).unwrap_or_default();
    let linkedin = profile.as_ref().and_then(|p| p.social_linkedin.clone()).unwrap_or_default();
    let twitter = profile.as_ref().and_then(|p| p.social_twitter.clone()).unwrap_or_default();
    let competitors = profile.as_ref().and_then(|p| p.competitor_brands.clone()).unwrap_or_default();
    let pool_clone = pool.clone();

    tokio::spawn(async move {
        if let Err(e) = run_brand_research(
            &pool_clone, org_id, &org_name, &website, &instagram, &linkedin, &twitter, &competitors
        ).await {
            tracing::error!("[BRAND_RESEARCH] Failed for org {}: {}", org_id, e);
            let _ = sqlx::query(
                "UPDATE organization_brand_profiles SET research_status = 'failed', updated_at = datetime('now','subsec') WHERE organization_id = ?",
            )
            .bind(org_id.as_bytes().as_slice())
            .execute(&pool_clone)
            .await;
        }
    });

    Ok(Json(ApiResponse::success(BrandResearchJobResponse {
        org_id,
        status: "queued".into(),
        message: format!("Brand research queued for {} — Scout is gathering online presence data via Exa.", org.name),
    })))
}

/// GET /api/organizations/:id/brand-research/status
pub async fn get_brand_research_status(
    State(deployment): State<DeploymentImpl>,
    Extension(access_context): Extension<AccessContext>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ApiResponse<BrandResearchStatusResponse>>, ApiError> {
    let _ = access_context;

    #[derive(sqlx::FromRow)]
    struct Row {
        research_status: String,
        research_summary: Option<String>,
        research_ran_at: Option<String>,
    }

    let row: Option<Row> = sqlx::query_as(
        "SELECT research_status, research_summary, research_ran_at FROM organization_brand_profiles WHERE organization_id = ?",
    )
    .bind(org_id.as_bytes().as_slice())
    .fetch_optional(&deployment.db().pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("{e}")))?;

    let (status, summary, ran_at) = row
        .map(|r| (r.research_status, r.research_summary, r.research_ran_at))
        .unwrap_or_else(|| ("idle".into(), None, None));

    Ok(Json(ApiResponse::success(BrandResearchStatusResponse {
        org_id,
        status,
        summary,
        ran_at,
    })))
}

/// POST /api/organizations/:id/seed-brand-project
/// Creates a brand project + deliverable set for this org if not already present.
pub async fn seed_brand_project(
    Path(org_id): Path<Uuid>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    use serde_json::json;

    let pool = &deployment.db().pool;

    let org = Organization::find_by_id(pool, &org_id.to_string())
        .await?
        .ok_or_else(|| ApiError::NotFound("Organization not found".into()))?;

    if !access_context.is_admin {
        let role = Organization::get_user_role(pool, &org_id.to_string(), access_context.user_id).await?;
        if role.is_none() {
            return Err(ApiError::Forbidden("Not a member of this organization".into()));
        }
    }

    // Find or create the brand project for this org
    #[derive(sqlx::FromRow)]
    struct ProjRow { id: Uuid, name: String }

    let existing: Option<ProjRow> = sqlx::query_as(
        "SELECT id, name FROM projects WHERE organization_id = ? AND project_status != 'archived' ORDER BY created_at ASC LIMIT 1"
    )
    .bind(org_id)
    .fetch_optional(pool)
    .await?;

    let project_id = if let Some(p) = existing {
        tracing::info!("[BRAND_PROJECT] Using existing project '{}' for {}", p.name, org.name);
        p.id
    } else {
        // Create brand project
        let proj_id = Uuid::new_v4();
        let proj_name = format!("{} — Brand Identity", org.name);
        sqlx::query(
            "INSERT INTO projects (id, name, organization_id, owner_id, slug, created_at, updated_at) VALUES (?, ?, ?, ?, ?, datetime('now','subsec'), datetime('now','subsec'))"
        )
        .bind(proj_id)
        .bind(&proj_name)
        .bind(org_id)
        .bind(access_context.user_id)
        .bind(org.slug.to_lowercase().replace(' ', "-") + "-brand")
        .execute(pool)
        .await?;

        // Add caller as project owner
        let mem_id = Uuid::new_v4();
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO project_members (id, project_id, user_id, role) VALUES (?, ?, ?, 'owner')"
        )
        .bind(mem_id)
        .bind(proj_id)
        .bind(access_context.user_id)
        .execute(pool)
        .await;

        tracing::info!("[BRAND_PROJECT] Created project '{}' for {}", proj_name, org.name);
        proj_id
    };

    // Deliverable specs for brand guide production workflow
    let deliverable_specs: &[(&str, &str, &str, i64)] = &[
        ("document", "Brand Strategy & Research Report",
         "Comprehensive brand research: market positioning, ICP, competitive landscape, and strategic direction. Generated by Scout research pipeline.",
         1),
        ("graphic",  "Visual Identity System",
         "Complete visual identity: logo suite (primary, reversed, monochrome, icon), colour palette system with tints/shades, typography scale, iconography style guide, and design tokens.",
         3),
        ("document", "Brand Guide Document",
         "Full brand standards document: foundation, visual identity, voice and tone, brand applications (business card, letterhead, social, email), usage rules, and do/don't examples.",
         2),
        ("graphic",  "Social Media Template Pack",
         "10 branded templates for Instagram, LinkedIn, Twitter/X, and Facebook. Includes post, story, cover, and ad formats. Editable in Canva with brand colours and typography pre-loaded.",
         2),
        ("graphic",  "Merchandise Specification Sheet",
         "Print-ready brand spec sheet for all Vistaprint Corporate Store products: business cards, letterhead, apparel, mugs, totes, banners. Includes production-ready colour codes (Pantone, CMYK, RGB, HEX).",
         1),
        ("document", "Corporate Store Configuration",
         "Vistaprint Corporate Store setup: branded product catalogue, pricing tiers, order fulfilment workflow, and partner rate sheet. Enables direct client ordering at PCG partner pricing.",
         1),
    ];

    // Create deliverables — skip if title already exists in this project
    let mut created: Vec<serde_json::Value> = vec![];
    for (dtype, title, description, revisions) in deliverable_specs {
        let exists: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM deliverables WHERE project_id = ? AND title = ? LIMIT 1"
        )
        .bind(project_id)
        .bind(*title)
        .fetch_optional(pool)
        .await?;

        if exists.is_some() {
            continue; // already seeded
        }

        let del_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO deliverables (id, project_id, deliverable_type, title, description, revision_rounds_allowed, status) VALUES (?, ?, ?, ?, ?, ?, 'working')"
        )
        .bind(del_id)
        .bind(project_id)
        .bind(*dtype)
        .bind(*title)
        .bind(*description)
        .bind(*revisions)
        .execute(pool)
        .await?;

        created.push(json!({ "id": del_id, "title": title, "type": dtype }));
    }

    // Mark the Research Report as done — we already have research
    let _ = sqlx::query(
        "UPDATE deliverables SET status = 'done', delivered_at = datetime('now','subsec') WHERE project_id = ? AND title = 'Brand Strategy & Research Report' AND status = 'working'"
    )
    .bind(project_id)
    .execute(pool)
    .await;

    Ok(Json(ApiResponse::success(json!({
        "project_id": project_id,
        "org_name": org.name,
        "deliverables_created": created.len(),
        "deliverables": created,
        "message": format!("Brand project seeded for {}. {} deliverables created.", org.name, created.len()),
    }))))
}

// ── Merge two JSON objects — values from b override a where non-null/non-empty ─
fn merge_json(a: &serde_json::Value, b: &serde_json::Value) -> serde_json::Value {
    let mut result = a.clone();
    if let (Some(_a_obj), Some(b_obj)) = (a.as_object(), b.as_object()) {
        let r_obj = result.as_object_mut().unwrap();
        for (key, b_val) in b_obj {
            let should_override = match b_val {
                serde_json::Value::Null => false,
                serde_json::Value::String(s) => !s.is_empty(),
                serde_json::Value::Array(arr) => !arr.is_empty(),
                serde_json::Value::Number(_) | serde_json::Value::Bool(_) | serde_json::Value::Object(_) => true,
            };
            if should_override {
                r_obj.insert(key.clone(), b_val.clone());
            }
        }
    }
    result
}

// ── Upsert a single knowledge graph entry ─────────────────────────────────────
async fn upsert_knowledge_entry(
    pool: &sqlx::SqlitePool,
    proj_id_bytes: &[u8],
    org_id_hex: &str,
    source_id: &str,
    title: &str,
    summary: &str,
    score: f64,
) {
    let _ = sqlx::query(
        r#"INSERT INTO project_knowledge_sources
           (id, project_id, owner_type, owner_id, source_type, source_id, source_title, source_summary, coverage_score, auto_registered)
           VALUES (?, ?, 'organization', ?, 'entity', ?, ?, ?, ?, 1)
           ON CONFLICT(project_id, source_type, source_id) DO UPDATE SET
             source_title=excluded.source_title, source_summary=excluded.source_summary,
             coverage_score=excluded.coverage_score, is_stale=0,
             last_refreshed_at=datetime('now','subsec'), updated_at=datetime('now','subsec')"#
    )
    .bind(Uuid::new_v4().as_bytes().to_vec())
    .bind(proj_id_bytes)
    .bind(org_id_hex)
    .bind(source_id)
    .bind(title)
    .bind(summary)
    .bind(score)
    .execute(pool).await;
}

// ── Core brand research execution ─────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn run_brand_research(
    pool: &sqlx::SqlitePool,
    org_id: Uuid,
    org_name: &str,
    website: &str,
    instagram: &str,
    linkedin: &str,
    twitter: &str,
    competitors: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use serde_json::json;

    // Mark running, increment iteration counter
    sqlx::query(
        "UPDATE organization_brand_profiles SET research_status = 'running', research_iterations = research_iterations + 1, updated_at = datetime('now','subsec') WHERE organization_id = ?",
    )
    .bind(org_id.as_bytes().as_slice())
    .execute(pool)
    .await?;

    let exa_key = std::env::var("EXA_API_KEY").unwrap_or_default();
    let anthropic_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("NORA_ANTHROPIC_API_KEY"))
        .unwrap_or_default();
    let client = reqwest::Client::new();

    // Read current iteration count + previous knowledge after the increment above
    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct IterRow {
        research_iterations: i64,
        research_depth: i64,
        research_summary: Option<String>,
        brand_gap_notes: Option<String>,
        founder_name: Option<String>,
        founding_year: Option<String>,
        estimated_team_size: Option<String>,
        geographic_focus: Option<String>,
        key_clients: Option<String>,
        tech_stack: Option<String>,
    }
    let iter_row = sqlx::query_as::<_, IterRow>(
        "SELECT research_iterations, research_depth, research_summary, brand_gap_notes, founder_name, founding_year, estimated_team_size, geographic_focus, key_clients, tech_stack FROM organization_brand_profiles WHERE organization_id = ?"
    )
    .bind(org_id.as_bytes().as_slice())
    .fetch_optional(pool)
    .await
    .ok().flatten();

    let iteration: i64 = iter_row.as_ref().map(|r| r.research_iterations).unwrap_or(1);
    let prev_summary = iter_row.as_ref().and_then(|r| r.research_summary.clone()).unwrap_or_default();
    let prev_gaps = iter_row.as_ref().and_then(|r| r.brand_gap_notes.clone()).unwrap_or_default();
    let prev_founder = iter_row.as_ref().and_then(|r| r.founder_name.clone()).unwrap_or_default();
    let prev_geo = iter_row.as_ref().and_then(|r| r.geographic_focus.clone()).unwrap_or_default();
    let prev_clients = iter_row.as_ref().and_then(|r| r.key_clients.clone()).unwrap_or_default();

    tracing::info!("[BRAND_RESEARCH] Starting iteration {} for {}", iteration, org_name);

    // Extract domain root (used by Clearbit and Exa searches)
    let domain = if website.starts_with("http") {
        website.trim_start_matches("https://").trim_start_matches("http://")
            .split('/').next().unwrap_or("").to_string()
    } else { String::new() };

    // ── 0. Clearbit logo auto-discovery ──────────────────────────────────────
    if !domain.is_empty() {
        let cb_url = format!("https://logo.clearbit.com/{}", domain);
        if let Ok(resp) = client.get(&cb_url).timeout(std::time::Duration::from_secs(8)).send().await {
            if resp.status().is_success() {
                tracing::info!("[BRAND_RESEARCH] Clearbit logo found for {}", domain);
                let _ = sqlx::query(
                    "UPDATE organization_brand_profiles SET clearbit_logo_url = ?, logo_url = COALESCE(logo_url, ?) WHERE organization_id = ?"
                )
                .bind(&cb_url).bind(&cb_url)
                .bind(org_id.as_bytes().as_slice())
                .execute(pool).await;
            }
        }
    }

    // ── 1. PHASE 1: Broad intelligence gathering — 14 parallel Exa searches ──
    let mut exa_context = String::new();

    if !exa_key.is_empty() {
        let short_name = org_name.split_whitespace().next().unwrap_or(org_name);
        let search_queries: Vec<String> = vec![
            // Core identity
            format!("{} agency overview services capabilities what they do clients", org_name),
            format!("\"{}\" founder CEO executive team leadership bio background", org_name),
            // Press & authority
            format!("\"{}\" press coverage news article interview feature announcement", org_name),
            format!("\"{}\" award recognition achievement industry best", org_name),
            // Events & community
            format!("\"{}\" conference event speaking appearance sponsor 2024 2025", org_name),
            // Competitive
            format!("{} vs {} competitors alternative comparison web3 marketing agency", org_name, competitors),
            format!("{} market positioning differentiation value proposition unique", org_name),
            // Social proof
            format!("\"{}\" client testimonial case study portfolio work results", org_name),
            format!("\"{}\" review reputation rating glassdoor clutch g2", org_name),
            // Content & thought leadership
            format!("{} blog article thought leadership insights content strategy", org_name),
            // Business intelligence
            format!("\"{}\" funding investment revenue growth team size hiring jobs", org_name),
            format!("site:linkedin.com \"{}\" company employees about", short_name),
            // Technical & digital
            format!("{} technology stack infrastructure platform tools", org_name),
            // Geographic & market
            format!("{} global offices locations market regions clients geography", org_name),
        ];

        // On iterations 2+, add gap-targeted queries replacing the last 2 generic ones
        let mut search_queries = search_queries;
        if iteration >= 2 && !prev_founder.is_empty() {
            search_queries.pop(); // drop last generic query
            search_queries.push(format!("\"{}\" {} biography background career history portfolio", prev_founder, org_name));
        }
        if iteration >= 2 && !prev_geo.is_empty() {
            search_queries.pop(); // drop second-to-last
            search_queries.push(format!("{} office address headquarters city country operations", org_name));
        }
        if iteration >= 3 {
            // Third pass: deep-dive on clients and events
            search_queries.push(format!("{} event production conference summit attendees participants sponsorship", org_name));
            search_queries.push(format!("{} client success story blockchain nft web3 startup result outcome", org_name));
        }

        tracing::info!("[BRAND_RESEARCH] Running {} parallel Exa searches (iteration {})", search_queries.len(), iteration);

        // First 3 queries (core identity) use full text; rest use highlights
        let search_futures: Vec<_> = search_queries.iter().enumerate().map(|(i, query)| {
            let client = client.clone();
            let key = exa_key.clone();
            let q = query.clone();
            let deep = i < 3;
            async move {
                client
                    .post("https://api.exa.ai/search")
                    .header("Authorization", format!("Bearer {}", key))
                    .header("Content-Type", "application/json")
                    .timeout(std::time::Duration::from_secs(22))
                    .json(&json!({
                        "query": q,
                        "num_results": if deep { 4 } else { 3 },
                        "use_autoprompt": true,
                        "type": "neural",
                        "contents": if deep {
                            json!({ "text": { "maxCharacters": 1500 } })
                        } else {
                            json!({ "highlights": { "numSentences": 3, "highlightsPerUrl": 2 } })
                        }
                    }))
                    .send()
                    .await
                    .ok()
            }
        }).collect();

        let results = futures::future::join_all(search_futures).await;

        for resp_opt in results {
            if let Some(resp) = resp_opt {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if let Some(items) = data["results"].as_array() {
                        for r in items {
                            let title = r["title"].as_str().unwrap_or("");
                            let url = r["url"].as_str().unwrap_or("");
                            let snippet = r["text"].as_str()
                                .or_else(|| r["highlights"].as_array()
                                    .and_then(|h| h.first())
                                    .and_then(|h| h.as_str()))
                                .unwrap_or("");
                            exa_context.push_str(&format!(
                                "\n---\nTitle: {}\nURL: {}\nContent: {}\n",
                                title, url, &snippet[..snippet.len().min(600)]
                            ));
                        }
                    }
                }
            }
        }

        // findSimilar for competitor discovery (8 results)
        if !website.is_empty() {
            if let Ok(resp) = client
                .post("https://api.exa.ai/findSimilar")
                .header("Authorization", format!("Bearer {}", exa_key))
                .header("Content-Type", "application/json")
                .timeout(std::time::Duration::from_secs(20))
                .json(&json!({
                    "url": website,
                    "num_results": 8,
                    "exclude_source_domain": true,
                    "contents": { "highlights": { "numSentences": 2 } }
                }))
                .send()
                .await
            {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if let Some(items) = data["results"].as_array() {
                        exa_context.push_str("\n\n## SIMILAR BRANDS / COMPETITORS (Exa findSimilar):\n");
                        for r in items {
                            let title = r["title"].as_str().unwrap_or("");
                            let url = r["url"].as_str().unwrap_or("");
                            let snip = r["highlights"].as_array()
                                .and_then(|h| h.first()).and_then(|h| h.as_str()).unwrap_or("");
                            exa_context.push_str(&format!("- {} ({}): {}\n", title, url, &snip[..snip.len().min(200)]));
                        }
                    }
                }
            }
        }

        tracing::info!("[BRAND_RESEARCH] Exa Phase 1 complete — {} chars", exa_context.len());
    } else {
        tracing::warn!("[BRAND_RESEARCH] EXA_API_KEY not set — skipping Exa");
    }

    // ── 2. Deep website scrape (8 pages) + PageSpeed ─────────────────────────

    fn strip_html(html: &str, max_chars: usize) -> String {
        let mut out = String::with_capacity(html.len().min(max_chars));
        let mut in_tag = false;
        let mut in_script = false;
        let mut in_style = false;
        let mut tag_buf = String::new();
        for c in html.chars() {
            if out.len() >= max_chars { break; }
            match c {
                '<' => { in_tag = true; tag_buf.clear(); }
                '>' => {
                    let t = tag_buf.trim().to_lowercase();
                    if t.starts_with("script") { in_script = true; }
                    else if t.starts_with("/script") { in_script = false; }
                    else if t.starts_with("style") { in_style = true; }
                    else if t.starts_with("/style") { in_style = false; }
                    in_tag = false; tag_buf.clear();
                    if !in_script && !in_style { out.push(' '); }
                }
                _ => {
                    if in_tag { tag_buf.push(c); }
                    else if !in_script && !in_style { out.push(c); }
                }
            }
        }
        out.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn extract_tech_stack(html: &str, headers: &[String]) -> Vec<String> {
        let mut stack: std::collections::HashSet<String> = std::collections::HashSet::new();
        let lhtml = html.to_lowercase();
        if lhtml.contains("__next") || lhtml.contains("/_next/") { stack.insert("Next.js".into()); }
        if lhtml.contains("nuxt") { stack.insert("Nuxt.js".into()); }
        if lhtml.contains("gatsby") { stack.insert("Gatsby".into()); }
        if lhtml.contains("wordpress") || lhtml.contains("wp-content") { stack.insert("WordPress".into()); }
        if lhtml.contains("webflow") { stack.insert("Webflow".into()); }
        if lhtml.contains("shopify") { stack.insert("Shopify".into()); }
        if lhtml.contains("react") { stack.insert("React".into()); }
        if lhtml.contains("vue") { stack.insert("Vue.js".into()); }
        if lhtml.contains("tailwind") { stack.insert("Tailwind CSS".into()); }
        if lhtml.contains("framer") { stack.insert("Framer".into()); }
        if lhtml.contains("hubspot") { stack.insert("HubSpot".into()); }
        if lhtml.contains("intercom") { stack.insert("Intercom".into()); }
        if lhtml.contains("google-analytics") || lhtml.contains("gtag") { stack.insert("Google Analytics".into()); }
        if lhtml.contains("hotjar") { stack.insert("Hotjar".into()); }
        if lhtml.contains("crisp.chat") { stack.insert("Crisp Chat".into()); }
        if lhtml.contains("typeform") { stack.insert("Typeform".into()); }
        for h in headers {
            let hl = h.to_lowercase();
            if hl.contains("vercel") { stack.insert("Vercel".into()); }
            if hl.contains("netlify") { stack.insert("Netlify".into()); }
            if hl.contains("cloudflare") { stack.insert("Cloudflare".into()); }
            if hl.contains("aws") || hl.contains("amazon") { stack.insert("AWS".into()); }
        }
        let mut out: Vec<String> = stack.into_iter().collect();
        out.sort();
        out
    }

    fn extract_social_links(html: &str) -> Vec<(String, String)> {
        let patterns = [
            ("instagram", "instagram.com/"),
            ("linkedin", "linkedin.com/company/"),
            ("twitter", "twitter.com/"),
            ("twitter", "x.com/"),
            ("facebook", "facebook.com/"),
            ("youtube", "youtube.com/@"),
            ("youtube", "youtube.com/channel/"),
            ("tiktok", "tiktok.com/@"),
            ("telegram", "t.me/"),
        ];
        let mut found: Vec<(String, String)> = vec![];
        let mut seen = std::collections::HashSet::new();
        for (platform, pattern) in &patterns {
            let lower = html.to_lowercase();
            let mut search_start = 0;
            while let Some(pos) = lower[search_start..].find(pattern) {
                let abs = search_start + pos;
                let after = &html[abs + pattern.len()..];
                let handle: String = after.chars()
                    .take_while(|c| !matches!(c, '"' | '\'' | '?' | ' ' | '>' | '/'))
                    .collect();
                if !handle.is_empty() && !seen.contains(&handle) {
                    seen.insert(handle.clone());
                    found.push((platform.to_string(), handle));
                }
                search_start = abs + 1;
                if search_start >= lower.len() { break; }
            }
        }
        found
    }

    let mut website_content = String::new();
    let mut pagespeed_context = String::new();
    let mut detected_tech_stack: Vec<String> = vec![];

    if !website.is_empty() {
        let base = website.trim_end_matches('/');
        let sub_pages = vec![
            website.to_string(),
            format!("{}/about", base),
            format!("{}/services", base),
            format!("{}/team", base),
            format!("{}/work", base),
            format!("{}/blog", base),
            format!("{}/case-studies", base),
            format!("{}/events", base),
        ];

        let page_futures: Vec<_> = sub_pages.iter().map(|url| {
            let c = client.clone();
            let u = url.clone();
            async move {
                let resp = c.get(&u)
                    .header("User-Agent", "Mozilla/5.0 (compatible; BrandScout/2.0; +https://powerclubglobal.com/bot)")
                    .timeout(std::time::Duration::from_secs(15))
                    .send().await;
                match resp {
                    Ok(r) if r.status().is_success() => {
                        let headers: Vec<String> = r.headers().iter()
                            .map(|(k, v)| format!("{}: {}", k, v.to_str().unwrap_or("")))
                            .collect();
                        let text = r.text().await.unwrap_or_default();
                        Some((u, text, headers))
                    }
                    _ => None,
                }
            }
        }).collect();

        let ps_future = client
            .get("https://www.googleapis.com/pagespeedonline/v5/runPagespeed")
            .query(&[("url", website), ("strategy", "mobile"), ("category", "performance"), ("category", "seo")])
            .timeout(std::time::Duration::from_secs(25))
            .send();

        let (page_results, ps_result) = futures::future::join(
            futures::future::join_all(page_futures),
            ps_future,
        ).await;

        let mut social_links_extracted = false;
        for (i, result) in page_results.into_iter().enumerate() {
            if let Some((_url, html, headers)) = result {
                if i == 0 {
                    if !social_links_extracted {
                        let social_links = extract_social_links(&html);
                        if !social_links.is_empty() {
                            website_content.push_str("\n\n## SOCIAL LINKS EXTRACTED FROM WEBSITE:\n");
                            for (platform, handle) in &social_links {
                                website_content.push_str(&format!("- {}: {}\n", platform, handle));
                            }
                            social_links_extracted = true;
                        }
                    }
                    detected_tech_stack = extract_tech_stack(&html, &headers);
                    if !detected_tech_stack.is_empty() {
                        website_content.push_str(&format!("\n\n## DETECTED TECH STACK: {}\n", detected_tech_stack.join(", ")));
                    }
                }
                let stripped = strip_html(&html, 2500);
                if !stripped.is_empty() {
                    let page_label = sub_pages.get(i).map(|u| u.as_str()).unwrap_or("page");
                    let page_label = page_label.trim_start_matches("https://").trim_start_matches("http://");
                    website_content.push_str(&format!("\n\n=== PAGE: {} ===\n{}", page_label, stripped));
                }
            }
        }

        if let Ok(ps_resp) = ps_result {
            if let Ok(ps_json) = ps_resp.json::<serde_json::Value>().await {
                let perf = ps_json["lighthouseResult"]["categories"]["performance"]["score"].as_f64().unwrap_or(0.0) * 100.0;
                let seo  = ps_json["lighthouseResult"]["categories"]["seo"]["score"].as_f64().unwrap_or(0.0) * 100.0;
                let fcp  = ps_json["lighthouseResult"]["audits"]["first-contentful-paint"]["displayValue"].as_str().unwrap_or("?");
                pagespeed_context = format!("PageSpeed (mobile): Performance={:.0}/100, SEO={:.0}/100, FCP={}", perf, seo, fcp);
                tracing::info!("[BRAND_RESEARCH] {}", pagespeed_context);
            }
        }

        tracing::info!("[BRAND_RESEARCH] Website scrape complete — {} chars, tech: {:?}", website_content.len(), detected_tech_stack);
    }

    // ── 3. PASS 1: Claude synthesis — extract all facts ──────────────────────
    let known_handles = format!("Instagram: {}, LinkedIn: {}, Twitter: {}", instagram, linkedin, twitter);

    let synthesis_prompt = format!(
        r#"You are an elite brand intelligence analyst with deep expertise in creative agencies, marketing technology, and startup ecosystems.

Analyse EVERYTHING provided about "{name}" and extract a comprehensive, structured intelligence profile.

INSTRUCTIONS:
- Extract VERBATIM text (taglines, headlines, copy) — do not paraphrase
- Infer only what is clearly supported by evidence; mark uncertain fields with low confidence
- social_profiles: use ONLY handles extracted from the website (ground-truth) — do not invent
- Identify the actual founder/CEO name from any bio, press, or about page content
- Detect tech stack from website signals
- Estimate team size from LinkedIn data, job listings, or "our team" pages
- Note any clients, case studies, or portfolio work mentioned by name
- Identify geographic focus (markets served, office locations, events attended)
- Assess content strategy from blog topics, social posting patterns

Return ONLY valid JSON with this structure:
{{
  "tagline": "exact hero headline from website or null",
  "brand_voice_observed": "authoritative|bold|casual|playful|formal|sophisticated",
  "market_position_observed": "luxury|premium|mid-market|budget",
  "industry_observed": "specific industry string",
  "target_audience": "who they serve — be specific",
  "icp_description": "ideal customer profile — detailed",
  "icp_company_size": "solo|startup|smb|mid-market|enterprise",
  "icp_industries": ["industry1", "industry2"],
  "competitors_found": ["competitor1", "competitor2"],
  "key_differentiators": ["differentiator1", "differentiator2"],
  "mission_inferred": "mission statement if found or inferable",
  "vision_inferred": "vision statement if found or inferable",
  "social_profiles": [
    {{"platform": "instagram", "handle": "handle_without_@", "confidence": 0.95}},
    {{"platform": "linkedin", "handle": "slug", "confidence": 0.95}},
    {{"platform": "twitter", "handle": "handle", "confidence": 0.95}}
  ],
  "founder_name": "Full Name or null",
  "founder_background": "brief bio if found or null",
  "founding_year": "YYYY or null",
  "estimated_team_size": "1-5|5-15|15-50|50-200|200+ or null",
  "key_clients_mentioned": ["client1", "client2"],
  "case_study_topics": ["topic1", "topic2"],
  "tech_stack_detected": ["tech1", "tech2"],
  "geographic_focus": "primary markets/regions",
  "funding_stage": "bootstrapped|pre-seed|seed|series-a|series-b|public|unknown",
  "content_pillars_observed": ["pillar1", "pillar2"],
  "thought_leadership_topics": ["topic1", "topic2"],
  "awards_recognition": ["award1", "award2"],
  "brand_voice_examples": ["exact quote 1", "exact quote 2"],
  "online_summary": "2-3 sentence comprehensive summary of the entity",
  "research_gaps": ["gap1: what we still don't know", "gap2"],
  "confidence": 0.0
}}

WEBSITE DATA (8 pages scraped):
{website}
{pagespeed}

TECH STACK DETECTED FROM HTML:
{tech}

EXA RESEARCH (14 searches):
{exa}

KNOWN HANDLES (current DB):
{handles}
{prior}"#,
        name = org_name,
        website = if website_content.is_empty() { "Not fetched".into() } else { website_content.chars().take(9000).collect::<String>() },
        pagespeed = if pagespeed_context.is_empty() { String::new() } else { format!("\nPAGESPEED: {}", pagespeed_context) },
        tech = if detected_tech_stack.is_empty() { "None detected".into() } else { detected_tech_stack.join(", ") },
        exa = if exa_context.is_empty() { "No Exa results".into() } else { exa_context.chars().take(11000).collect::<String>() },
        handles = known_handles,
        prior = if iteration <= 1 { String::new() } else {
            format!(
                "\n\nPRIOR KNOWLEDGE (from {} previous iterations — build on this, do not lose it):\nSummary: {}\nFounder: {}\nGeo: {}\nClients: {}\nKnown gaps to resolve:\n{}",
                iteration - 1,
                if prev_summary.is_empty() { "None yet".to_string() } else { prev_summary.chars().take(800).collect() },
                if prev_founder.is_empty() { "Unknown".to_string() } else { prev_founder },
                if prev_geo.is_empty() { "Unknown".to_string() } else { prev_geo },
                if prev_clients.is_empty() { "[]".to_string() } else { prev_clients },
                if prev_gaps.is_empty() { "None identified".to_string() } else { prev_gaps.chars().take(1500).collect() },
            )
        },
    );

    let pass1_body = json!({
        "model": "claude-sonnet-4-6",
        "max_tokens": 4096,
        "messages": [{"role": "user", "content": synthesis_prompt}]
    });

    tracing::info!("[BRAND_RESEARCH] Pass 1 synthesis for {} (iteration {})", org_name, iteration);
    let pass1_resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &anthropic_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .timeout(std::time::Duration::from_secs(90))
        .json(&pass1_body)
        .send()
        .await?;

    let pass1_json: serde_json::Value = pass1_resp.json().await?;
    let pass1_text = pass1_json["content"]
        .as_array()
        .and_then(|arr| arr.iter().find(|c| c["type"] == "text"))
        .and_then(|c| c["text"].as_str())
        .unwrap_or("");

    let parsed: serde_json::Value = serde_json::from_str(pass1_text)
        .or_else(|_| {
            if let Some(start) = pass1_text.find('{') {
                if let Some(end) = pass1_text.rfind('}') {
                    return serde_json::from_str(&pass1_text[start..=end]);
                }
            }
            Err(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, "no JSON")))
        })
        .unwrap_or_default();

    tracing::info!("[BRAND_RESEARCH] Pass 1 complete — confidence: {:.2}", parsed["confidence"].as_f64().unwrap_or(0.0));

    // ── 4. PASS 2: Gap analysis — targeted searches + refinement ─────────────
    let confidence = parsed["confidence"].as_f64().unwrap_or(0.0);
    let gaps: Vec<String> = parsed["research_gaps"].as_array()
        .map(|g| g.iter().filter_map(|v| v.as_str()).map(String::from).collect())
        .unwrap_or_default();

    let mut final_data = parsed.clone();

    if !exa_key.is_empty() && (confidence < 0.8 || !gaps.is_empty() || iteration > 1) {
        tracing::info!("[BRAND_RESEARCH] Pass 2 gap analysis — {} gaps, confidence {:.2}", gaps.len(), confidence);

        let mut gap_queries: Vec<String> = vec![];

        if iteration >= 2 || parsed["founder_name"].is_null() {
            gap_queries.push(format!("\"{}\" CEO founder president director leadership team executive", org_name));
        }
        if iteration >= 2 || parsed["key_clients_mentioned"].as_array().map(|a| a.len()).unwrap_or(0) == 0 {
            gap_queries.push(format!("\"{}\" client project portfolio case study work brand campaign", org_name));
        }
        if iteration >= 2 || parsed["awards_recognition"].as_array().map(|a| a.len()).unwrap_or(0) == 0 {
            gap_queries.push(format!("\"{}\" award recognition achievement winner best agency 2023 2024", org_name));
        }
        for gap in gaps.iter().take(4) {
            gap_queries.push(format!("{} {} site evidence", org_name, gap.chars().take(60).collect::<String>()));
        }
        if let Some(li_handle) = parsed["social_profiles"].as_array()
            .and_then(|arr| arr.iter().find(|p| p["platform"].as_str() == Some("linkedin")))
            .and_then(|p| p["handle"].as_str())
        {
            gap_queries.push(format!("site:linkedin.com/company/{} employees size about overview", li_handle));
        }

        if !gap_queries.is_empty() {
            let gap_futures: Vec<_> = gap_queries.iter().map(|q| {
                let client = client.clone();
                let key = exa_key.clone();
                let query = q.clone();
                async move {
                    client
                        .post("https://api.exa.ai/search")
                        .header("Authorization", format!("Bearer {}", key))
                        .header("Content-Type", "application/json")
                        .timeout(std::time::Duration::from_secs(20))
                        .json(&json!({
                            "query": query,
                            "num_results": 3,
                            "use_autoprompt": true,
                            "type": "neural",
                            "contents": { "text": { "maxCharacters": 1000 } }
                        }))
                        .send().await.ok()
                }
            }).collect();

            let gap_results = futures::future::join_all(gap_futures).await;

            let mut gap_context = String::new();
            for resp_opt in gap_results {
                if let Some(resp) = resp_opt {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if let Some(items) = data["results"].as_array() {
                            for r in items {
                                let title = r["title"].as_str().unwrap_or("");
                                let url = r["url"].as_str().unwrap_or("");
                                let content = r["text"].as_str().unwrap_or("");
                                gap_context.push_str(&format!(
                                    "\n---\nTitle: {}\nURL: {}\nContent: {}\n",
                                    title, url, &content[..content.len().min(500)]
                                ));
                            }
                        }
                    }
                }
            }

            tracing::info!("[BRAND_RESEARCH] Gap pass complete — {} chars", gap_context.len());

            if !gap_context.is_empty() {
                let pass2_prompt = format!(
                    r#"You are completing a brand intelligence profile for "{}".

EXISTING PASS 1 DATA:
{}

NEW GAP RESEARCH:
{}

Using the new research, extend and correct the existing data. Return a JSON object with ONLY the fields that have new or improved data. Use the same schema as the original. Focus on: founder_name, founding_year, key_clients_mentioned, awards_recognition, estimated_team_size, geographic_focus, funding_stage, thought_leadership_topics, confidence (update to reflect total knowledge)."#,
                    org_name,
                    serde_json::to_string_pretty(&parsed).unwrap_or_default().chars().take(3000).collect::<String>(),
                    gap_context.chars().take(6000).collect::<String>(),
                );

                let pass2_body = json!({
                    "model": "claude-sonnet-4-6",
                    "max_tokens": 2048,
                    "messages": [{"role": "user", "content": pass2_prompt}]
                });

                tracing::info!("[BRAND_RESEARCH] Pass 2 refinement for {}", org_name);
                if let Ok(pass2_resp) = client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", &anthropic_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .timeout(std::time::Duration::from_secs(60))
                    .json(&pass2_body)
                    .send()
                    .await
                {
                    if let Ok(pass2_json) = pass2_resp.json::<serde_json::Value>().await {
                        let pass2_text = pass2_json["content"]
                            .as_array()
                            .and_then(|arr| arr.iter().find(|c| c["type"] == "text"))
                            .and_then(|c| c["text"].as_str())
                            .unwrap_or("");

                        if let Ok(pass2_data) = serde_json::from_str::<serde_json::Value>(pass2_text)
                            .or_else(|_| {
                                if let Some(s) = pass2_text.find('{') {
                                    if let Some(e) = pass2_text.rfind('}') {
                                        return serde_json::from_str(&pass2_text[s..=e]);
                                    }
                                }
                                Err(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, "no JSON")))
                            })
                        {
                            final_data = merge_json(&parsed, &pass2_data);
                            tracing::info!("[BRAND_RESEARCH] Pass 2 merged — final confidence: {:.2}", final_data["confidence"].as_f64().unwrap_or(confidence));
                        }
                    }
                }
            }
        }
    }

    // ── 5. Write results back to brand profile ────────────────────────────────
    // Resolve confidence robustly — Claude may return it as number or string
    let final_confidence = final_data["confidence"].as_f64()
        .or_else(|| final_data["confidence"].as_str().and_then(|s| s.parse::<f64>().ok()))
        .unwrap_or(confidence); // fall back to Pass 1 confidence
    let depth_this_run = if final_confidence >= 0.85 { 3i64 } else if final_confidence >= 0.65 { 2 } else { 1 };
    // Preserve the highest depth ever achieved across iterations
    let prev_depth = iter_row.as_ref().map(|r| r.research_depth).unwrap_or(0);
    let depth = depth_this_run.max(prev_depth);
    let summary = final_data["online_summary"].as_str().unwrap_or("Research complete").to_string();

    let mut updates: Vec<(&str, String)> = vec![];

    macro_rules! push_str {
        ($field:expr, $key:expr) => {
            if let Some(v) = final_data[$key].as_str() {
                if !v.is_empty() && v != "unknown" && v != "null" {
                    updates.push(($field, v.to_string()));
                }
            }
        };
    }
    macro_rules! push_arr {
        ($field:expr, $key:expr) => {
            if let Some(arr) = final_data[$key].as_array() {
                let v: Vec<String> = arr.iter().filter_map(|x| x.as_str()).map(String::from).collect();
                if !v.is_empty() {
                    updates.push(($field, serde_json::to_string(&v).unwrap_or_default()));
                }
            }
        };
    }

    push_str!("brand_voice", "brand_voice_observed");
    push_str!("market_position", "market_position_observed");
    push_str!("industry", "industry_observed");
    push_str!("tagline", "tagline");
    push_str!("target_audience", "target_audience");
    push_str!("icp_description", "icp_description");
    push_str!("icp_company_size", "icp_company_size");
    push_str!("mission_statement", "mission_inferred");
    push_str!("vision_statement", "vision_inferred");
    // Extended fields
    push_str!("founder_name", "founder_name");
    push_str!("founding_year", "founding_year");
    push_str!("estimated_team_size", "estimated_team_size");
    push_str!("geographic_focus", "geographic_focus");
    push_str!("funding_stage", "funding_stage");
    push_str!("content_strategy_notes", "content_strategy_notes");
    // Arrays
    push_arr!("icp_industries", "icp_industries");
    push_arr!("competitor_brands", "competitors_found");
    push_arr!("differentiators", "key_differentiators");
    push_arr!("content_pillars", "content_pillars_observed");
    push_arr!("key_clients", "key_clients_mentioned");
    push_arr!("awards_and_recognition", "awards_recognition");

    // Tech stack (combine detected + Claude-found)
    {
        let mut tech: std::collections::HashSet<String> = detected_tech_stack.iter().cloned().collect();
        if let Some(arr) = final_data["tech_stack_detected"].as_array() {
            for v in arr { if let Some(s) = v.as_str() { tech.insert(s.to_string()); } }
        }
        if !tech.is_empty() {
            let mut tv: Vec<String> = tech.into_iter().collect();
            tv.sort();
            updates.push(("tech_stack", serde_json::to_string(&tv).unwrap_or_default()));
        }
    }

    // Gap notes
    if let Some(gaps_arr) = final_data["research_gaps"].as_array() {
        let gap_list: Vec<String> = gaps_arr.iter().filter_map(|v| v.as_str()).map(String::from).collect();
        if !gap_list.is_empty() {
            updates.push(("brand_gap_notes", gap_list.join("; ")));
        }
    }

    // Social profiles — authoritative direct write
    let mut social_updates: Vec<(&str, String)> = vec![];
    if let Some(profiles) = final_data["social_profiles"].as_array() {
        for p in profiles {
            let platform = p["platform"].as_str().unwrap_or("");
            let handle = p["handle"].as_str().unwrap_or("");
            if handle.is_empty() || handle == "unknown" { continue; }
            let col: Option<&'static str> = match platform {
                "instagram" => Some("social_instagram"),
                "linkedin"  => Some("social_linkedin"),
                "twitter"   => Some("social_twitter"),
                "facebook"  => Some("social_facebook"),
                "youtube"   => Some("social_youtube"),
                "tiktok"    => Some("social_tiktok"),
                _ => None,
            };
            if let Some(col) = col { social_updates.push((col, handle.to_string())); }
        }
    }
    updates.extend(social_updates.into_iter().map(|(col, val)| (col, val)));

    if !updates.is_empty() {
        let social_cols = ["social_instagram","social_linkedin","social_twitter","social_facebook","social_youtube","social_tiktok"];
        let array_cols  = ["icp_industries","competitor_brands","differentiators","content_pillars","brand_values","key_clients","awards_and_recognition"];
        let direct_cols = ["brand_voice","market_position","industry","tagline","target_audience","icp_description","icp_company_size",
                           "founder_name","founding_year","estimated_team_size","geographic_focus","funding_stage","content_strategy_notes",
                           "mission_statement","vision_statement","tech_stack","brand_gap_notes"];

        let set_clauses: Vec<String> = updates.iter().map(|(col, _)| {
            if social_cols.contains(col) {
                format!("{} = ?", col)
            } else if array_cols.contains(col) {
                format!("{col} = CASE WHEN {col} IS NULL OR {col} = '[]' OR {col} = '' THEN ? ELSE {col} END")
            } else if direct_cols.contains(col) {
                format!("{} = COALESCE({}, ?)", col, col)
            } else {
                format!("{} = ?", col)
            }
        }).collect();

        let sql = format!(
            "UPDATE organization_brand_profiles SET {}, research_status='done', research_summary=?, research_ran_at=datetime('now','subsec'), research_depth=?, updated_at=datetime('now','subsec') WHERE organization_id=?",
            set_clauses.join(", ")
        );
        let mut q = sqlx::query(&sql);
        for (_, val) in &updates { q = q.bind(val); }
        q = q.bind(&summary).bind(depth).bind(org_id.as_bytes().as_slice());
        q.execute(pool).await?;
    } else {
        sqlx::query(
            "UPDATE organization_brand_profiles SET research_status='done', research_summary=?, research_ran_at=datetime('now','subsec'), research_depth=?, updated_at=datetime('now','subsec') WHERE organization_id=?"
        )
        .bind(&summary).bind(depth).bind(org_id.as_bytes().as_slice())
        .execute(pool).await?;
    }

    // ── 5b. DALL-E 3 mood board generation ───────────────────────────────────
    let openai_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    if !openai_key.is_empty() {
        #[derive(sqlx::FromRow)]
        struct ProfileSnap {
            primary_color: String,
            accent_color: Option<String>,
            brand_archetype: Option<String>,
            brand_voice: Option<String>,
            industry: Option<String>,
            market_position: Option<String>,
            tagline: Option<String>,
        }
        if let Ok(Some(snap)) = sqlx::query_as::<_, ProfileSnap>(
            "SELECT primary_color, accent_color, brand_archetype, brand_voice, industry, market_position, tagline FROM organization_brand_profiles WHERE organization_id = ?"
        ).bind(org_id.as_bytes().as_slice()).fetch_optional(pool).await {
            let archetype = snap.brand_archetype.as_deref().unwrap_or("Ruler");
            let voice = snap.brand_voice.as_deref().unwrap_or("authoritative");
            let industry = snap.industry.as_deref().unwrap_or("creative agency");
            let position = snap.market_position.as_deref().unwrap_or("premium");
            let tagline_hint = snap.tagline.as_deref().unwrap_or("");
            let primary = &snap.primary_color;
            let accent = snap.accent_color.as_deref().unwrap_or("#C0A050");
            let founder = final_data["founder_name"].as_str().unwrap_or("");
            let clients = final_data["key_clients_mentioned"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).take(3).collect::<Vec<_>>().join(", "))
                .unwrap_or_default();

            let prompt1 = format!(
                "Ultra-premium editorial brand mood board for '{name}', a {position} {industry}. Archetype: {archetype}. Voice: {voice}. \
                Colors: {primary} and {accent} gold metallic. \
                Visual: sophisticated dark luxury aesthetic, editorial typography, high-end creative production studio, \
                international events and galas, elite professionals at work, geometric brand patterns. \
                Photographic collage, 16:9, ultra high-end art direction. No text.",
                name=org_name, position=position, industry=industry, archetype=archetype,
                voice=voice, primary=primary, accent=accent
            );
            let prompt2 = format!(
                "Premium brand collateral showcase for '{name}'. '{tagline}' \
                Colors: {primary} and {accent}. \
                Business cards with embossed foil, premium letterhead, branded notebooks, roll-up banners at a conference, \
                branded merchandise on dark surface. {clients_hint} \
                {voice} {position} energy. Dark moody studio photography, selective lighting.",
                name=org_name, tagline=tagline_hint, primary=primary, accent=accent,
                clients_hint=if clients.is_empty() { String::new() } else { format!("Clients like {}.", clients) },
                voice=voice, position=position
            );

            let mut mood_urls: Vec<String> = vec![];
            for prompt in [prompt1, prompt2] {
                if let Ok(r) = client
                    .post("https://api.openai.com/v1/images/generations")
                    .bearer_auth(&openai_key)
                    .json(&json!({"model":"dall-e-3","prompt":prompt,"quality":"hd","size":"1792x1024","style":"vivid","n":1}))
                    .timeout(std::time::Duration::from_secs(60))
                    .send().await
                {
                    if let Ok(j) = r.json::<serde_json::Value>().await {
                        if let Some(url) = j["data"][0]["url"].as_str() {
                            mood_urls.push(url.to_string());
                        } else if let Some(err) = j["error"]["message"].as_str() {
                            tracing::warn!("[BRAND_RESEARCH] DALL-E: {}", err);
                        }
                    }
                }
            }
            if !mood_urls.is_empty() {
                let photo_notes = format!(
                    "{} and {} gold palette. {} archetype. {} market. Founder: {}. Photography: selective lighting, editorial composition, luxury texture.",
                    primary, accent, archetype, position,
                    if founder.is_empty() { "Unknown" } else { founder }
                );
                let _ = sqlx::query(
                    "UPDATE organization_brand_profiles SET mood_board_urls=?, brand_photography_notes=? WHERE organization_id=?"
                )
                .bind(serde_json::to_string(&mood_urls).unwrap_or_default())
                .bind(photo_notes)
                .bind(org_id.as_bytes().as_slice())
                .execute(pool).await;
                tracing::info!("[BRAND_RESEARCH] {} mood board images generated", mood_urls.len());
            }
        }
    } else {
        tracing::info!("[BRAND_RESEARCH] OPENAI_API_KEY not set — skipping mood board generation");
    }

    // ── 6. Store research as deduplicated data source ─────────────────────────
    let report_title = format!("Brand Intelligence Report — {}", org_name);
    let pagespeed_section = if pagespeed_context.is_empty() { String::new() } else {
        format!("\n\n## Website Performance\n{}", pagespeed_context)
    };
    let research_doc = format!(
        "# Brand Intelligence Report: {}\n\nIteration: {}\nGenerated: {}\nConfidence: {:.0}%\nDepth: {}/3\n\n## Summary\n{}{}\n\n## Full Research Data\n```json\n{}\n```",
        org_name, iteration,
        chrono::Utc::now().format("%Y-%m-%d"),
        final_confidence * 100.0, depth,
        summary, pagespeed_section,
        serde_json::to_string_pretty(&final_data).unwrap_or_default()
    );

    let org_id_bytes = org_id.as_bytes().to_vec();
    sqlx::query("DELETE FROM data_sources WHERE organization_id = ? AND title = ?")
        .bind(&org_id_bytes).bind(&report_title).execute(pool).await?;

    sqlx::query(
        "INSERT INTO data_sources (id, organization_id, title, description, data_type, source_type, content, status, metadata, created_at, updated_at) VALUES (?, ?, ?, ?, 'report', 'text', ?, 'ready', '{}', datetime('now','subsec'), datetime('now','subsec'))"
    )
    .bind(Uuid::new_v4().as_bytes().to_vec())
    .bind(&org_id_bytes)
    .bind(&report_title)
    .bind(format!("Iteration {}. Confidence {:.0}%. Depth {}/3. Covers: identity, competitive, audience, social, founder, clients, tech stack.", iteration, final_confidence * 100.0, depth))
    .bind(&research_doc)
    .execute(pool).await?;

    // ── 7. CRM: create/update founder person record ───────────────────────────
    if let Some(founder) = final_data["founder_name"].as_str() {
        if !founder.is_empty() {
            let name_parts: Vec<&str> = founder.splitn(2, ' ').collect();
            let first = name_parts.first().copied().unwrap_or(founder);
            let last = name_parts.get(1).copied().unwrap_or("");
            let bio = final_data["founder_background"].as_str().unwrap_or("");

            let exists: Option<(Vec<u8>,)> = sqlx::query_as(
                "SELECT id FROM persons WHERE first_name = ? AND (last_name = ? OR last_name IS NULL) LIMIT 1"
            )
            .bind(first).bind(last)
            .fetch_optional(pool).await.ok().flatten();

            if exists.is_none() {
                let person_id = Uuid::new_v4();
                let _ = sqlx::query(
                    "INSERT INTO persons (id, first_name, last_name, role, notes, created_at, updated_at) VALUES (?, ?, ?, 'Founder / CEO', ?, datetime('now','subsec'), datetime('now','subsec'))"
                )
                .bind(person_id.as_bytes().as_slice())
                .bind(first)
                .bind(last)
                .bind(if bio.is_empty() { None } else { Some(bio) })
                .execute(pool).await;
                tracing::info!("[BRAND_RESEARCH] Created CRM person record for founder: {}", founder);
            } else {
                tracing::info!("[BRAND_RESEARCH] Founder {} already in CRM", founder);
            }
        }
    }

    // ── 8. Knowledge graph — 8 entity types ──────────────────────────────────
    #[derive(sqlx::FromRow)]
    struct ProjIdRow { id: Uuid }
    if let Ok(Some(proj)) = sqlx::query_as::<_, ProjIdRow>(
        "SELECT id FROM projects WHERE organization_id = ? AND project_status != 'archived' ORDER BY created_at ASC LIMIT 1"
    )
    .bind(org_id)
    .fetch_optional(pool).await {
        let proj_id_bytes = proj.id.as_bytes().to_vec();
        let org_id_hex = hex::encode(org_id.as_bytes());

        // 1. Brand Intelligence (core)
        upsert_knowledge_entry(pool, &proj_id_bytes, &org_id_hex, "brand_intelligence",
            &format!("Brand Intelligence — {}", org_name), &summary, final_confidence).await;

        // 2. Competitive Landscape
        if let Some(comps) = final_data["competitors_found"].as_array() {
            let comp_str: Vec<&str> = comps.iter().filter_map(|v| v.as_str()).collect();
            if !comp_str.is_empty() {
                let diffs: Vec<&str> = final_data["key_differentiators"].as_array()
                    .map(|d| d.iter().filter_map(|v| v.as_str()).take(3).collect())
                    .unwrap_or_default();
                upsert_knowledge_entry(pool, &proj_id_bytes, &org_id_hex, "competitive_landscape",
                    &format!("Competitive Landscape — {}", org_name),
                    &format!("Competitors: {}. Differentiators: {}.", comp_str.join(", "), diffs.join("; ")),
                    0.82).await;
            }
        }

        // 3. Audience Intelligence
        if let Some(icp) = final_data["icp_description"].as_str() {
            let industries: Vec<&str> = final_data["icp_industries"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
            upsert_knowledge_entry(pool, &proj_id_bytes, &org_id_hex, "audience_intelligence",
                &format!("Audience Intelligence — {}", org_name),
                &format!("ICP: {}. Industries: {}. Size: {}.",
                    icp,
                    if industries.is_empty() { "Not specified".into() } else { industries.join(", ") },
                    final_data["icp_company_size"].as_str().unwrap_or("Unknown")),
                0.85).await;
        }

        // 4. Online Presence
        {
            let socials: Vec<String> = final_data["social_profiles"].as_array()
                .map(|arr| arr.iter().filter_map(|p| {
                    let pl = p["platform"].as_str()?;
                    let h = p["handle"].as_str()?;
                    if h == "unknown" || h.is_empty() { return None; }
                    Some(format!("{}: {}", pl, h))
                }).collect())
                .unwrap_or_default();
            upsert_knowledge_entry(pool, &proj_id_bytes, &org_id_hex, "online_presence",
                &format!("Online Presence — {}", org_name),
                &format!("Website: {}. Socials: {}. {}",
                    website,
                    if socials.is_empty() { "Unknown".into() } else { socials.join(", ") },
                    pagespeed_context),
                0.90).await;
        }

        // 5. Brand Identity
        upsert_knowledge_entry(pool, &proj_id_bytes, &org_id_hex, "brand_identity",
            &format!("Brand Identity — {}", org_name),
            &format!("Voice: {}. Market position: {}. Tagline: {}",
                final_data["brand_voice_observed"].as_str().unwrap_or("Unknown"),
                final_data["market_position_observed"].as_str().unwrap_or("Unknown"),
                final_data["tagline"].as_str().unwrap_or("Not found")),
            0.88).await;

        // 6. People Intelligence (founder + team)
        if let Some(founder) = final_data["founder_name"].as_str() {
            if !founder.is_empty() {
                upsert_knowledge_entry(pool, &proj_id_bytes, &org_id_hex, "people_intelligence",
                    &format!("People Intelligence — {}", org_name),
                    &format!("Founder/CEO: {}. {}. Team size: {}. Founded: {}.",
                        founder,
                        final_data["founder_background"].as_str().unwrap_or(""),
                        final_data["estimated_team_size"].as_str().unwrap_or("Unknown"),
                        final_data["founding_year"].as_str().unwrap_or("Unknown")),
                    0.75).await;
            }
        }

        // 7. Client & Portfolio Intelligence
        if let Some(clients) = final_data["key_clients_mentioned"].as_array() {
            let client_list: Vec<&str> = clients.iter().filter_map(|v| v.as_str()).collect();
            if !client_list.is_empty() {
                let case_topics: Vec<&str> = final_data["case_study_topics"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
                upsert_knowledge_entry(pool, &proj_id_bytes, &org_id_hex, "client_portfolio",
                    &format!("Client & Portfolio Intelligence — {}", org_name),
                    &format!("Known clients: {}. Case study topics: {}.",
                        client_list.join(", "),
                        if case_topics.is_empty() { "Not found".into() } else { case_topics.join(", ") }),
                    0.70).await;
            }
        }

        // 8. Technology & Operations
        {
            let mut tech_all: Vec<String> = detected_tech_stack.clone();
            if let Some(arr) = final_data["tech_stack_detected"].as_array() {
                for v in arr { if let Some(s) = v.as_str() { tech_all.push(s.to_string()); } }
            }
            tech_all.sort();
            tech_all.dedup();
            if !tech_all.is_empty() {
                upsert_knowledge_entry(pool, &proj_id_bytes, &org_id_hex, "tech_operations",
                    &format!("Technology & Operations — {}", org_name),
                    &format!("Tech stack: {}. Geographic focus: {}. Funding: {}.",
                        tech_all.join(", "),
                        final_data["geographic_focus"].as_str().unwrap_or("Unknown"),
                        final_data["funding_stage"].as_str().unwrap_or("Unknown")),
                    0.65).await;
            }
        }

        tracing::info!("[BRAND_RESEARCH] Knowledge graph updated — iteration {}, depth {}/3", iteration, depth);
    } else {
        tracing::info!("[BRAND_RESEARCH] No project found for org {} — knowledge stored in data_sources only", org_name);
    }

    tracing::info!("[BRAND_RESEARCH] Completed {} — iteration {}, confidence {:.2}", org_name, iteration, final_confidence);
    Ok(())
}
