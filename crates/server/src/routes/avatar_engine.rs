//! Avatar Profile Engine — Stage 1 of the anchor setup pipeline.
//!
//! Takes a single reference image of a person (real or AI-generated), produces:
//!   1. A structured character bible (Claude vision pass over the reference)
//!   2. A 16-shot portrait set generated with OpenAI gpt-image-1 `images/edits`
//!      using the reference as the input and the bible as identity grounding.
//!
//! All shots land on local disk under `dev_assets/avatars/<id>/shots/` and are
//! served by the public route added in `video_gen.rs`.

use std::{path::PathBuf, sync::Arc, time::Duration};

use base64::Engine;
use db::models::avatar_profile::AvatarProfile;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;
use tokio::{fs, sync::Semaphore, task::JoinSet};
use uuid::Uuid;

// ─── Slot taxonomy ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct ShotSlot {
    pub key: &'static str,
    pub category: &'static str,
    pub prompt: &'static str,
}

/// 16 canonical slots. Prompts intentionally reference identity-locking
/// language so the bible block can be appended at the call site.
pub const SHOT_SLOTS: &[ShotSlot] = &[
    // Identity Sheet (8)
    ShotSlot {
        key: "front",
        category: "identity",
        prompt: "Studio headshot, the exact same person from the reference photographed straight-on at eye level. Neutral confident expression, soft natural lighting, plain grey backdrop, sharp focus on eyes. Editorial photography quality.",
    },
    ShotSlot {
        key: "three_quarter_left",
        category: "identity",
        prompt: "Studio headshot of the exact same person from the reference, head turned about 45 degrees to camera-left, eyes toward camera. Neutral expression, soft lighting, plain grey backdrop.",
    },
    ShotSlot {
        key: "three_quarter_right",
        category: "identity",
        prompt: "Studio headshot of the exact same person from the reference, head turned about 45 degrees to camera-right, eyes toward camera. Neutral expression, soft lighting, plain grey backdrop.",
    },
    ShotSlot {
        key: "profile_left",
        category: "identity",
        prompt: "Studio side-profile portrait of the exact same person from the reference, full left profile, looking forward (not at camera). Neutral expression, plain grey backdrop.",
    },
    ShotSlot {
        key: "profile_right",
        category: "identity",
        prompt: "Studio side-profile portrait of the exact same person from the reference, full right profile, looking forward (not at camera). Neutral expression, plain grey backdrop.",
    },
    ShotSlot {
        key: "full_body_front",
        category: "identity",
        prompt: "Full-body studio portrait of the exact same person from the reference, standing straight-on at eye level, hands relaxed at sides. Neutral pose, plain grey backdrop, editorial fashion photography.",
    },
    ShotSlot {
        key: "smile_warm",
        category: "identity",
        prompt: "Headshot of the exact same person from the reference, straight-on, genuine warm smile reaching the eyes. Soft lighting, plain grey backdrop.",
    },
    ShotSlot {
        key: "serious_neutral",
        category: "identity",
        prompt: "Headshot of the exact same person from the reference, straight-on, serious composed expression. Soft lighting, plain grey backdrop.",
    },
    // Wardrobe (4)
    ShotSlot {
        key: "studio_black",
        category: "wardrobe",
        prompt: "Editorial portrait of the exact same person from the reference wearing a tailored black outfit, on a black studio backdrop. Soft directional lighting, sharp focus, high-fashion magazine quality. Upper body framing.",
    },
    ShotSlot {
        key: "editorial_white",
        category: "wardrobe",
        prompt: "Editorial portrait of the exact same person from the reference wearing crisp white attire, on a clean white seamless backdrop. Soft even lighting, minimal aesthetic, upper body framing.",
    },
    ShotSlot {
        key: "casual_outdoor",
        category: "wardrobe",
        prompt: "Natural-light portrait of the exact same person from the reference in casual everyday clothing, outdoors during golden hour, slightly out-of-focus urban background. Three-quarter framing.",
    },
    ShotSlot {
        key: "evening_dressed",
        category: "wardrobe",
        prompt: "Editorial evening portrait of the exact same person from the reference in elegant evening wear, dim moody lighting, soft bokeh background. Upper body framing, refined and cinematic.",
    },
    // Action (4)
    ShotSlot {
        key: "laughing",
        category: "action",
        prompt: "Candid portrait of the exact same person from the reference caught mid-genuine-laugh, eyes crinkled, natural light. Three-quarter framing, shallow depth of field.",
    },
    ShotSlot {
        key: "looking_down",
        category: "action",
        prompt: "Contemplative portrait of the exact same person from the reference looking downward and slightly away from camera, soft thoughtful expression. Moody soft lighting, neutral background.",
    },
    ShotSlot {
        key: "walking_three_quarter",
        category: "action",
        prompt: "Three-quarter angle full-body shot of the exact same person from the reference walking toward camera, mid-stride, natural outdoor light, slight motion energy in the frame.",
    },
    ShotSlot {
        key: "thinking_pose",
        category: "action",
        prompt: "Portrait of the exact same person from the reference with one hand near the chin in a thoughtful pose, gaze just off-camera, warm directional light, neutral backdrop.",
    },
];

// ─── Persisted shapes ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortraitShot {
    pub slot: String,
    pub category: String,
    pub url: String,
    pub prompt: String,
    pub locked: bool,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterBible {
    /// Estimated age range, e.g. "mid 20s".
    pub age_range: Option<String>,
    /// Apparent gender presentation (purely descriptive, e.g. "feminine").
    pub gender_presentation: Option<String>,
    /// Hair description — color, length, style, texture.
    pub hair: Option<String>,
    /// Eye color and shape.
    pub eyes: Option<String>,
    /// Skin tone and complexion notes.
    pub skin: Option<String>,
    /// Body build / silhouette descriptors.
    pub build: Option<String>,
    /// Distinguishing features (jewelry, marks, accessories worn in reference).
    pub distinguishing_features: Option<Vec<String>>,
    /// Wardrobe defaults seen in the reference.
    pub wardrobe_defaults: Option<String>,
    /// Overall vibe / styling notes.
    pub style_vibe: Option<String>,
    /// Things the engine must NOT do (e.g. "do not add glasses, do not lighten skin").
    pub do_not_do: Option<Vec<String>>,
}

// ─── Public API ──────────────────────────────────────────────────────────────

pub struct PipelineConfig {
    pub avatar_id: Uuid,
    pub reference_path: PathBuf,
    pub shots_dir: PathBuf,
    pub public_base_path: String,
}

/// Run the full profile-generation pipeline. Updates `profile_status` along
/// the way and persists `bible_json` + `portrait_set` on success.
pub async fn run_pipeline(pool: SqlitePool, cfg: PipelineConfig) {
    if let Err(e) = run_pipeline_inner(&pool, &cfg).await {
        tracing::error!(
            "avatar profile pipeline failed for {}: {:#}",
            cfg.avatar_id,
            e
        );
        let _ = AvatarProfile::update_profile_status(
            &pool,
            cfg.avatar_id,
            "failed",
            Some(&format!("{e:#}")),
        )
        .await;
    }
}

async fn run_pipeline_inner(pool: &SqlitePool, cfg: &PipelineConfig) -> anyhow::Result<()> {
    AvatarProfile::update_profile_status(pool, cfg.avatar_id, "generating", None).await?;

    let reference_bytes = fs::read(&cfg.reference_path).await?;
    let reference_b64 = base64::engine::general_purpose::STANDARD.encode(&reference_bytes);

    // 1. Character bible via Claude vision
    let anthropic_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;
    let bible = generate_bible(&anthropic_key, &reference_b64).await?;
    let bible_json = serde_json::to_string(&bible)?;
    AvatarProfile::set_bible(pool, cfg.avatar_id, &bible_json).await?;
    tracing::info!("avatar {} bible generated", cfg.avatar_id);

    // 2. 16 shots in parallel (pool of 4)
    let fal_key =
        std::env::var("FAL_API_KEY").map_err(|_| anyhow::anyhow!("FAL_API_KEY not set"))?;
    fs::create_dir_all(&cfg.shots_dir).await?;

    let limiter = Arc::new(Semaphore::new(4));
    let bible_block = format_bible_block(&bible);
    let fal_key = Arc::new(fal_key);
    let reference_data_uri = Arc::new(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(reference_bytes.as_slice())
    ));
    let shots_dir = Arc::new(cfg.shots_dir.clone());
    let public_base = Arc::new(cfg.public_base_path.clone());

    let mut joinset: JoinSet<anyhow::Result<PortraitShot>> = JoinSet::new();

    for slot in SHOT_SLOTS {
        let limiter = Arc::clone(&limiter);
        let fal_key = Arc::clone(&fal_key);
        let reference_data_uri = Arc::clone(&reference_data_uri);
        let shots_dir = Arc::clone(&shots_dir);
        let public_base = Arc::clone(&public_base);
        let bible_block = bible_block.clone();
        let slot = *slot;

        joinset.spawn(async move {
            let _permit = limiter.acquire_owned().await?;
            generate_one_shot(
                &fal_key,
                &reference_data_uri,
                &shots_dir,
                &public_base,
                &bible_block,
                slot,
            )
            .await
        });
    }

    let mut shots: Vec<PortraitShot> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    while let Some(joined) = joinset.join_next().await {
        match joined {
            Ok(Ok(shot)) => shots.push(shot),
            Ok(Err(e)) => errors.push(format!("{e:#}")),
            Err(e) => errors.push(format!("join error: {e}")),
        }
    }

    if shots.is_empty() {
        anyhow::bail!("all 16 shot generations failed: {}", errors.join(" | "));
    }

    // Sort shots by slot order (matches SHOT_SLOTS order) so the UI is stable.
    shots.sort_by_key(|s| {
        SHOT_SLOTS
            .iter()
            .position(|slot| slot.key == s.slot)
            .unwrap_or(usize::MAX)
    });

    let portrait_set_json = serde_json::to_string(&shots)?;
    AvatarProfile::set_portrait_set(pool, cfg.avatar_id, &portrait_set_json).await?;

    if let Some(front) = shots.iter().find(|s| s.slot == "front") {
        AvatarProfile::set_thumbnail(pool, cfg.avatar_id, &front.url).await?;
    }

    let final_status = if errors.is_empty() {
        "ready"
    } else {
        "ready" // partial success — keep ready, surface errors in profile_error
    };
    let err_text = if errors.is_empty() {
        None
    } else {
        Some(format!(
            "{} of 16 shots failed: {}",
            errors.len(),
            errors.join(" | ")
        ))
    };
    AvatarProfile::update_profile_status(pool, cfg.avatar_id, final_status, err_text.as_deref())
        .await?;
    tracing::info!(
        "avatar {} profile ready ({} shots, {} errors)",
        cfg.avatar_id,
        shots.len(),
        errors.len()
    );
    Ok(())
}

// ─── Claude bible pass ───────────────────────────────────────────────────────

const BIBLE_SYSTEM: &str = "You are a casting director and visual-effects supervisor. \
You will be shown one reference photograph of a person. Produce a structured \
JSON character bible that captures the visual identity precisely enough that \
another image-generation model could produce 16 consistent portraits of the \
same individual. Respond ONLY with valid JSON matching the schema. No prose.";

const BIBLE_PROMPT: &str = r#"Return JSON exactly matching this schema:
{
  "age_range": string,
  "gender_presentation": string,
  "hair": string,
  "eyes": string,
  "skin": string,
  "build": string,
  "distinguishing_features": [string],
  "wardrobe_defaults": string,
  "style_vibe": string,
  "do_not_do": [string]
}
Be specific and concrete. Use neutral descriptive language. No prose outside the JSON."#;

async fn generate_bible(api_key: &str, reference_b64: &str) -> anyhow::Result<CharacterBible> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    let body = json!({
        "model": "claude-sonnet-4-6",
        "max_tokens": 1500,
        "system": BIBLE_SYSTEM,
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/png",
                        "data": reference_b64
                    }
                },
                { "type": "text", "text": BIBLE_PROMPT }
            ]
        }]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Anthropic bible call failed {status}: {text}");
    }

    #[derive(Deserialize)]
    struct AnthropicResp {
        content: Vec<AnthropicContent>,
    }
    #[derive(Deserialize)]
    struct AnthropicContent {
        text: Option<String>,
    }
    let data: AnthropicResp = resp.json().await?;
    let raw_text = data
        .content
        .into_iter()
        .find_map(|c| c.text)
        .ok_or_else(|| anyhow::anyhow!("Anthropic returned no text content"))?;

    let cleaned = strip_code_fence(&raw_text);
    let bible: CharacterBible = serde_json::from_str(cleaned).map_err(|e| {
        anyhow::anyhow!("failed to parse character bible JSON: {e}; raw text: {raw_text}")
    })?;
    Ok(bible)
}

fn strip_code_fence(s: &str) -> &str {
    let trimmed = s.trim();
    let without_open = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed);
    let final_s = without_open.strip_suffix("```").unwrap_or(without_open);
    final_s.trim()
}

fn format_bible_block(bible: &CharacterBible) -> String {
    let mut parts: Vec<String> = Vec::new();
    let push = |parts: &mut Vec<String>, label: &str, value: Option<&str>| {
        if let Some(v) = value.filter(|v| !v.trim().is_empty()) {
            parts.push(format!("{label}: {v}"));
        }
    };
    push(&mut parts, "Age", bible.age_range.as_deref());
    push(
        &mut parts,
        "Presentation",
        bible.gender_presentation.as_deref(),
    );
    push(&mut parts, "Hair", bible.hair.as_deref());
    push(&mut parts, "Eyes", bible.eyes.as_deref());
    push(&mut parts, "Skin", bible.skin.as_deref());
    push(&mut parts, "Build", bible.build.as_deref());
    push(&mut parts, "Wardrobe", bible.wardrobe_defaults.as_deref());
    push(&mut parts, "Vibe", bible.style_vibe.as_deref());
    if let Some(features) = bible.distinguishing_features.as_ref() {
        if !features.is_empty() {
            parts.push(format!("Features: {}", features.join(", ")));
        }
    }
    if let Some(dont) = bible.do_not_do.as_ref() {
        if !dont.is_empty() {
            parts.push(format!("DO NOT: {}", dont.join("; ")));
        }
    }
    parts.join(" | ")
}

// ─── OpenAI gpt-image-1 shot generation ──────────────────────────────────────

/// Generate one shot via fal.ai FLUX.1 Kontext Pro
/// (`fal-ai/flux-pro/kontext`). Uses base64 data-uri for the reference so the
/// model doesn't need to fetch from a public URL.
async fn generate_one_shot(
    fal_key: &str,
    reference_data_uri: &str,
    shots_dir: &PathBuf,
    public_base: &str,
    bible_block: &str,
    slot: ShotSlot,
) -> anyhow::Result<PortraitShot> {
    let prompt = format!(
        "{}. Identity lock — preserve the exact same person from the reference: {}. \
        Maintain identical facial structure, hair, eye color and shape, skin tone, \
        and distinguishing features. Photorealistic.",
        slot.prompt, bible_block
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()?;

    let body = serde_json::json!({
        "image_url": reference_data_uri,
        "prompt": prompt,
        "num_images": 1,
        "guidance_scale": 3.5,
        "output_format": "png",
        "safety_tolerance": "6",
    });

    let resp = client
        .post("https://fal.run/fal-ai/flux-pro/kontext")
        .header("Authorization", format!("Key {fal_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "fal kontext failed for slot {} ({status}): {text}",
            slot.key
        );
    }

    #[derive(Deserialize)]
    struct FalResp {
        images: Vec<FalImage>,
    }
    #[derive(Deserialize)]
    struct FalImage {
        url: String,
    }
    let data: FalResp = resp.json().await?;
    let image_url = data
        .images
        .into_iter()
        .next()
        .map(|i| i.url)
        .ok_or_else(|| anyhow::anyhow!("fal returned no images for slot {}", slot.key))?;

    let png_bytes = reqwest::get(&image_url).await?.bytes().await?.to_vec();

    let out_path = shots_dir.join(format!("{}.png", slot.key));
    fs::write(&out_path, &png_bytes).await?;

    let url = format!("{public_base}/shots/{}", slot.key);
    Ok(PortraitShot {
        slot: slot.key.to_string(),
        category: slot.category.to_string(),
        url,
        prompt,
        locked: false,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}
