use std::{cmp, collections::HashMap, path::{Path, PathBuf}};

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use db::models::{
    cinematic_brief::{
        CinematicBrief,
        CinematicBriefStatus,
        CinematicShotPlan,
        CinematicShotPlanStatus,
        CreateCinematicBrief,
        CreateCinematicShotPlan,
        UpdateCinematicBriefStatus,
        UpdateCinematicShotPlanStatus,
    },
    project_asset::{CreateProjectAsset, ProjectAsset},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tokio::{fs, time::{sleep, Duration}};
use tracing::info;
use uuid::Uuid;

/// Runtime configuration for the cinematics orchestration stack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CinematicsConfig {
    pub comfy_base_url: String,
    pub workflow_template: Option<PathBuf>,
    pub output_dir: PathBuf,
    pub default_sampler: String,
    pub auto_render: bool,
}

impl Default for CinematicsConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let comfy_root = std::env::var("COMFYUI_ROOT").unwrap_or_else(|_| format!("{home}/ComfyUI"));
        let output_dir = std::env::var("COMFYUI_OUTPUT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Path::new(&comfy_root).join("output"));

        Self {
            comfy_base_url: std::env::var("COMFYUI_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8188".into()),
            workflow_template: std::env::var("COMFYUI_WORKFLOW_TEMPLATE")
                .ok()
                .map(PathBuf::from),
            output_dir,
            default_sampler: std::env::var("COMFYUI_DEFAULT_SAMPLER")
                .unwrap_or_else(|_| "euler".into()),
            auto_render: std::env::var("CINEMATICS_AUTO_RENDER")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
        }
    }
}

#[async_trait]
pub trait Cinematographer {
    async fn create_brief(&self, payload: CreateCinematicBrief) -> Result<CinematicBrief>;
    async fn ensure_shot_plan(&self, brief_id: Uuid) -> Result<Vec<CinematicShotPlan>>;
    async fn trigger_render(&self, brief_id: Uuid) -> Result<CinematicBrief>;
}

pub struct CinematicsService {
    pool: SqlitePool,
    client: Client,
    config: CinematicsConfig,
}

impl CinematicsService {
    pub fn new(pool: SqlitePool, config: CinematicsConfig) -> Self {
        Self {
            pool,
            client: Client::new(),
            config,
        }
    }

    pub fn auto_render_enabled(&self) -> bool {
        self.config.auto_render
    }

    fn workflow_template_path(&self) -> Option<PathBuf> {
        self.config.workflow_template.clone()
    }

    fn comfy_endpoint(&self, path: &str) -> String {
        format!("{}/{}", self.config.comfy_base_url.trim_end_matches('/'), path.trim_start_matches('/'))
    }

    fn build_default_workflow(&self, prompt: &str, negative_prompt: &str) -> Value {
        json!({
            "3": {
                "class_type": "KSampler",
                "inputs": {
                    "cfg": 8,
                    "denoise": 1,
                    "latent_image": ["5", 0],
                    "model": ["4", 0],
                    "negative": ["7", 0],
                    "positive": ["6", 0],
                    "sampler_name": self.config.default_sampler,
                    "scheduler": "normal",
                    "seed": (Utc::now().timestamp_nanos_opt().unwrap_or(0) & 0xFFFF_FFFF) as i64,
                    "steps": 22
                }
            },
            "4": { "class_type": "CheckpointLoaderSimple", "inputs": { "ckpt_name": "sd_xl_base_1.0.safetensors" } },
            "5": { "class_type": "EmptyLatentImage", "inputs": { "batch_size": 1, "height": 832, "width": 512 } },
            "6": { "class_type": "CLIPTextEncode", "inputs": { "clip": ["4", 1], "text": prompt } },
            "7": { "class_type": "CLIPTextEncode", "inputs": { "clip": ["4", 1], "text": negative_prompt } },
            "8": { "class_type": "VAEDecode", "inputs": { "samples": ["3", 0], "vae": ["4", 2] } },
            "9": { "class_type": "SaveImage", "inputs": { "filename_prefix": "Cinematic", "images": ["8", 0] } }
        })
    }

    async fn load_workflow_template(&self) -> Result<Value> {
        if let Some(path) = self.workflow_template_path() {
            let data = fs::read_to_string(&path)
                .await
                .with_context(|| format!("Failed to read workflow template at {}", path.display()))?;
            Ok(serde_json::from_str(&data)? )
        } else {
            Ok(self.build_default_workflow("breathtaking establishing shot", "low quality"))
        }
    }

    fn summary_topics(&self, brief: &CinematicBrief) -> Vec<String> {
        let mut topics = Vec::new();
        let mut source = brief.summary.clone();
        if !brief.script.is_empty() {
            source.push_str(" ");
            source.push_str(&brief.script);
        }
        for sentence in source.split(|c| c == '.' || c == '\n') {
            let trimmed = sentence.trim();
            if trimmed.len() > 8 {
                topics.push(trimmed.to_string());
            }
        }
        if topics.is_empty() {
            topics.push(brief.title.clone());
        }
        topics.truncate(4);
        topics
    }

    fn build_prompt(&self, brief: &CinematicBrief, topic: &str) -> (String, String) {
        let mut style_tags = Vec::new();
        if let Some(array) = brief.style_tags.0.as_array() {
            for entry in array.iter().filter_map(|v| v.as_str()) {
                style_tags.push(entry.to_string());
            }
        }
        if style_tags.is_empty() {
            style_tags.push("cinematic lighting".into());
            style_tags.push("film grain".into());
        }
        let joined_tags = style_tags.join(", ");
        let prompt = format!("{} -- {} -- shot on virtual cinema camera", topic, joined_tags);
        let negative = "lowres, blurry, text artifacts".to_string();
        (prompt, negative)
    }

    async fn ensure_shots_internal(
        &self,
        brief: &CinematicBrief,
    ) -> Result<Vec<CinematicShotPlan>> {
        let existing = CinematicShotPlan::list_by_brief(&self.pool, brief.id).await?;
        if !existing.is_empty() {
            return Ok(existing);
        }

        let mut shots = Vec::new();
        for (idx, topic) in self.summary_topics(brief).into_iter().enumerate() {
            let (prompt, negative) = self.build_prompt(brief, &topic);
            let per_shot = cmp::max(1, brief.duration_seconds.max(4) / 4);
            let shot = CinematicShotPlan::create(
                &self.pool,
                &CreateCinematicShotPlan {
                    brief_id: brief.id,
                    shot_index: idx as i64,
                    title: format!("Shot {}", idx + 1),
                    prompt,
                    negative_prompt: Some(negative),
                    camera_notes: Some("sweeping dolly move with parallax".into()),
                    duration_seconds: Some(per_shot),
                    metadata: Some(json!({ "topic": topic })),
                    status: Some(CinematicShotPlanStatus::Planned),
                },
                Uuid::new_v4(),
            )
            .await?;
            shots.push(shot);
        }

        CinematicBrief::update_status(
            &self.pool,
            brief.id,
            &UpdateCinematicBriefStatus {
                status: CinematicBriefStatus::Planning,
                llm_notes: Some("Auto-generated shot list".into()),
                render_payload: None,
                output_assets: None,
            },
        )
        .await?;

        Ok(shots)
    }

    async fn fetch_brief(&self, id: Uuid) -> Result<CinematicBrief> {
        CinematicBrief::find_by_id(&self.pool, id)
            .await?
            .ok_or_else(|| anyhow!("Cinematic brief {} not found", id))
    }

    async fn register_assets(
        &self,
        brief: &CinematicBrief,
        output_files: &[ComfyImageOutput],
    ) -> Result<Vec<ProjectAsset>> {
        let mut assets = Vec::new();
        for img in output_files {
            let disk_path = self.resolve_output_path(img);
            let storage_path = disk_path.to_string_lossy().to_string();
            let mime = Self::infer_mime(&img.filename);
            let asset = ProjectAsset::create(
                &self.pool,
                Uuid::new_v4(),
                &CreateProjectAsset {
                    project_id: brief.project_id,
                    pod_id: None,
                    board_id: None,
                    category: Some("ai_short".into()),
                    scope: Some("team".into()),
                    name: img.filename.clone(),
                    storage_path,
                    checksum: None,
                    byte_size: None,
                    mime_type: Some(mime.into()),
                    metadata: Some(json!({
                        "comfy_subfolder": img.subfolder,
                        "comfy_type": img.kind,
                    }).to_string()),
                    uploaded_by: Some("master_cinematographer".into()),
                },
            )
            .await?;
            assets.push(asset);
        }
        Ok(assets)
    }

    fn resolve_output_path(&self, img: &ComfyImageOutput) -> PathBuf {
        if img.subfolder.is_empty() {
            self.config.output_dir.join(&img.filename)
        } else {
            self.config
                .output_dir
                .join(&img.subfolder)
                .join(&img.filename)
        }
    }

    fn infer_mime(filename: &str) -> &'static str {
        if filename.ends_with(".mp4") {
            "video/mp4"
        } else if filename.ends_with(".gif") {
            "image/gif"
        } else if filename.ends_with(".webm") {
            "video/webm"
        } else {
            "image/png"
        }
    }

    async fn queue_prompt(&self, workflow: Value) -> Result<String> {
        let resp = self
            .client
            .post(self.comfy_endpoint("/prompt"))
            .json(&json!({ "prompt": workflow }))
            .send()
            .await?
            .json::<QueueResponse>()
            .await?;
        Ok(resp.prompt_id)
    }

    async fn poll_history(&self, prompt_id: &str) -> Result<Vec<ComfyImageOutput>> {
        let mut attempts = 0;
        loop {
            attempts += 1;
            let history = self
                .client
                .get(self.comfy_endpoint(&format!("/history/{prompt_id}")))
                .send()
                .await?
                .json::<HistoryResponse>()
                .await?;

            if let Some(entry) = history.history.get(prompt_id) {
                if let Some(outputs) = entry.outputs.values().next() {
                    if let Some(images) = &outputs.images {
                        return Ok(images.clone());
                    }
                }
            }

            if attempts > 90 {
                return Err(anyhow!("Timed out waiting for ComfyUI render"));
            }

            sleep(Duration::from_secs(1)).await;
        }
    }

    async fn render_with_comfy(&self, workflow: Value) -> Result<Vec<ComfyImageOutput>> {
        let prompt_id = self.queue_prompt(workflow).await?;
        self.poll_history(&prompt_id).await
    }
}

#[async_trait]
impl Cinematographer for CinematicsService {
    async fn create_brief(&self, payload: CreateCinematicBrief) -> Result<CinematicBrief> {
        let brief_id = Uuid::new_v4();
        let brief = CinematicBrief::create(&self.pool, &payload, brief_id).await?;
        self.ensure_shot_plan(brief.id).await?;
        Ok(brief)
    }

    async fn ensure_shot_plan(&self, brief_id: Uuid) -> Result<Vec<CinematicShotPlan>> {
        let brief = self.fetch_brief(brief_id).await?;
        self.ensure_shots_internal(&brief).await
    }

    async fn trigger_render(&self, brief_id: Uuid) -> Result<CinematicBrief> {
        let brief = self.fetch_brief(brief_id).await?;
        let shots = self.ensure_shots_internal(&brief).await?;
        let shot_count = shots.len();

        CinematicBrief::update_status(
            &self.pool,
            brief.id,
            &UpdateCinematicBriefStatus {
                status: CinematicBriefStatus::Rendering,
                llm_notes: Some("Rendering via ComfyUI".into()),
                render_payload: None,
                output_assets: None,
            },
        )
        .await?;

        let mut combined_outputs = Vec::new();
        let template = self.load_workflow_template().await?;
        for shot in &shots {
            let mut workflow = template.clone();
            if let Some(node) = workflow.get_mut("6") {
                if let Some(inputs) = node.get_mut("inputs") {
                    inputs["text"] = Value::String(shot.prompt.clone());
                }
            }
            if let Some(node) = workflow.get_mut("7") {
                if let Some(inputs) = node.get_mut("inputs") {
                    inputs["text"] = Value::String(shot.negative_prompt.clone());
                }
            }

            let images = self.render_with_comfy(workflow.clone()).await?;
            combined_outputs.extend(images.clone());
            let rendered_files: Vec<String> = images.iter().map(|img| img.filename.clone()).collect();

            CinematicShotPlan::update_status(
                &self.pool,
                shot.id,
                &UpdateCinematicShotPlanStatus {
                    status: CinematicShotPlanStatus::Completed,
                    metadata: Some(json!({
                        "rendered_files": rendered_files,
                    })),
                },
            )
            .await?;
        }

        let assets = self.register_assets(&brief, &combined_outputs).await?;
        let asset_ids: Vec<String> = assets.iter().map(|asset| asset.id.to_string()).collect();

        let final_brief = CinematicBrief::update_status(
            &self.pool,
            brief.id,
            &UpdateCinematicBriefStatus {
                status: CinematicBriefStatus::Delivered,
                llm_notes: Some("Render complete".into()),
                render_payload: Some(json!({ "shots": shot_count })),
                output_assets: Some(asset_ids.clone()),
            },
        )
        .await?;

        info!("Cinematic brief {} rendered {} clips", brief.id, asset_ids.len());

        Ok(final_brief)
    }
}

#[derive(Debug, Deserialize)]
struct QueueResponse {
    prompt_id: String,
}

#[derive(Debug, Deserialize)]
struct HistoryResponse {
    #[serde(default)]
    history: HashMap<String, PromptHistoryEntry>,
}

#[derive(Debug, Deserialize)]
struct PromptHistoryEntry {
    #[serde(default)]
    outputs: HashMap<String, NodeOutput>,
}

#[derive(Debug, Deserialize)]
struct NodeOutput {
    #[serde(default)]
    images: Option<Vec<ComfyImageOutput>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ComfyImageOutput {
    pub filename: String,
    #[serde(default)]
    pub subfolder: String,
    #[serde(rename = "type")]
    pub kind: String,
}
