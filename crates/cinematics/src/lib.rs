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
    pub default_checkpoint: String,
    pub sdxl_checkpoint: String,
    pub svd_checkpoint: String,
    pub auto_render: bool,
    pub use_high_quality: bool,
}

impl Default for CinematicsConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());

        // Default to topos/comfy directory (symlinked from ~/ComfyUI/output)
        let topos_dir = Path::new(&home).join("topos");
        let default_output = topos_dir.join("comfy");

        let output_dir = std::env::var("COMFYUI_OUTPUT_DIR")
            .map(PathBuf::from)
            .unwrap_or(default_output);

        Self {
            comfy_base_url: std::env::var("COMFYUI_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8188".into()),
            workflow_template: std::env::var("COMFYUI_WORKFLOW_TEMPLATE")
                .ok()
                .map(PathBuf::from),
            output_dir,
            default_sampler: std::env::var("COMFYUI_DEFAULT_SAMPLER")
                .unwrap_or_else(|_| "euler".into()),
            default_checkpoint: std::env::var("COMFYUI_CHECKPOINT")
                .unwrap_or_else(|_| "v1-5-pruned-emaonly-fp16.safetensors".into()),
            sdxl_checkpoint: std::env::var("COMFYUI_SDXL_CHECKPOINT")
                .unwrap_or_else(|_| "sd_xl_base_1.0.safetensors".into()),
            svd_checkpoint: std::env::var("COMFYUI_SVD_CHECKPOINT")
                .unwrap_or_else(|_| "svd_xt.safetensors".into()),
            auto_render: std::env::var("CINEMATICS_AUTO_RENDER")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
            use_high_quality: std::env::var("CINEMATICS_HIGH_QUALITY")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),  // Default to high quality
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
        // Ensure output directory exists
        if !config.output_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&config.output_dir) {
                tracing::warn!(
                    "Failed to create ComfyUI output directory at {}: {}",
                    config.output_dir.display(),
                    e
                );
            } else {
                tracing::info!(
                    "Created ComfyUI output directory at {}",
                    config.output_dir.display()
                );
            }
        }

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
        if self.config.use_high_quality {
            // Use SDXL + SVD for high quality video
            self.build_sdxl_svd_workflow(prompt, negative_prompt)
        } else {
            // Fallback to AnimateDiff for lower VRAM systems
            self.build_animatediff_workflow(prompt, negative_prompt)
        }
    }

    /// High-quality workflow: SDXL generates keyframe, SVD animates to video
    fn build_sdxl_svd_workflow(&self, prompt: &str, negative_prompt: &str) -> Value {
        let seed = (Utc::now().timestamp_nanos_opt().unwrap_or(0) & 0xFFFF_FFFF) as i64;
        json!({
            // Load SDXL checkpoint
            "1": {
                "class_type": "CheckpointLoaderSimple",
                "inputs": { "ckpt_name": self.config.sdxl_checkpoint }
            },
            // SDXL positive prompt with quality boosters
            "2": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "clip": ["1", 1],
                    "text": format!("{}, masterpiece, best quality, highly detailed, cinematic lighting, 8k", prompt)
                }
            },
            // SDXL negative prompt
            "3": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "clip": ["1", 1],
                    "text": format!("{}, low quality, blurry, distorted, deformed, ugly, bad anatomy", negative_prompt)
                }
            },
            // SDXL latent (768x432 for 16:9 - optimized for 8GB VRAM)
            "4": {
                "class_type": "EmptyLatentImage",
                "inputs": { "batch_size": 1, "height": 432, "width": 768 }
            },
            // SDXL KSampler
            "5": {
                "class_type": "KSampler",
                "inputs": {
                    "cfg": 7,
                    "denoise": 1,
                    "latent_image": ["4", 0],
                    "model": ["1", 0],
                    "negative": ["3", 0],
                    "positive": ["2", 0],
                    "sampler_name": "dpmpp_2m",
                    "scheduler": "karras",
                    "seed": seed,
                    "steps": 25
                }
            },
            // Decode SDXL to image
            "6": {
                "class_type": "VAEDecode",
                "inputs": { "samples": ["5", 0], "vae": ["1", 2] }
            },
            // Load SVD model for video
            "7": {
                "class_type": "ImageOnlyCheckpointLoader",
                "inputs": { "ckpt_name": self.config.svd_checkpoint }
            },
            // SVD conditioning from image (768x432, 14 frames for 8GB VRAM)
            "8": {
                "class_type": "SVD_img2vid_Conditioning",
                "inputs": {
                    "clip_vision": ["7", 1],
                    "init_image": ["6", 0],
                    "vae": ["7", 2],
                    "width": 768,
                    "height": 432,
                    "video_frames": 14,
                    "motion_bucket_id": 127,
                    "fps": 8,
                    "augmentation_level": 0
                }
            },
            // SVD KSampler for video
            "9": {
                "class_type": "KSampler",
                "inputs": {
                    "cfg": 2.5,
                    "denoise": 1,
                    "latent_image": ["8", 2],
                    "model": ["7", 0],
                    "negative": ["8", 1],
                    "positive": ["8", 0],
                    "sampler_name": "euler",
                    "scheduler": "karras",
                    "seed": seed,
                    "steps": 20
                }
            },
            // Decode SVD video frames
            "10": {
                "class_type": "VAEDecode",
                "inputs": { "samples": ["9", 0], "vae": ["7", 2] }
            },
            // Save as animated WEBP (or use VHS_VideoCombine if available)
            "11": {
                "class_type": "SaveAnimatedWEBP",
                "inputs": {
                    "images": ["10", 0],
                    "fps": 8,
                    "filename_prefix": "CinematicHQ",
                    "lossless": false,
                    "quality": 90,
                    "method": "default"
                }
            }
        })
    }

    fn build_animatediff_workflow(&self, prompt: &str, negative_prompt: &str) -> Value {
        let seed = (Utc::now().timestamp_nanos_opt().unwrap_or(0) & 0xFFFF_FFFF) as i64;
        json!({
            // Load checkpoint
            "1": {
                "class_type": "CheckpointLoaderSimple",
                "inputs": { "ckpt_name": self.config.default_checkpoint }
            },
            // Load AnimateDiff with motion model (Gen1 loader returns MODEL type)
            "2": {
                "class_type": "ADE_AnimateDiffLoaderGen1",
                "inputs": {
                    "model_name": "mm_sd_v15_v2.ckpt",
                    "model": ["1", 0],
                    "beta_schedule": "sqrt_linear (AnimateDiff)"
                }
            },
            // Positive prompt
            "4": {
                "class_type": "CLIPTextEncode",
                "inputs": { "clip": ["1", 1], "text": prompt }
            },
            // Negative prompt
            "5": {
                "class_type": "CLIPTextEncode",
                "inputs": { "clip": ["1", 1], "text": negative_prompt }
            },
            // Empty latent for 16 frames (video)
            "6": {
                "class_type": "EmptyLatentImage",
                "inputs": { "batch_size": 16, "height": 512, "width": 512 }
            },
            // KSampler with AnimateDiff model
            "7": {
                "class_type": "KSampler",
                "inputs": {
                    "cfg": 7.5,
                    "denoise": 1,
                    "latent_image": ["6", 0],
                    "model": ["2", 0],
                    "negative": ["5", 0],
                    "positive": ["4", 0],
                    "sampler_name": self.config.default_sampler,
                    "scheduler": "normal",
                    "seed": seed,
                    "steps": 20
                }
            },
            // Decode latents to images
            "8": {
                "class_type": "VAEDecode",
                "inputs": { "samples": ["7", 0], "vae": ["1", 2] }
            },
            // Save as animated WEBP
            "9": {
                "class_type": "SaveAnimatedWEBP",
                "inputs": {
                    "filename_prefix": "CinematicVideo",
                    "fps": 8,
                    "lossless": false,
                    "quality": 85,
                    "method": "default",
                    "images": ["8", 0]
                }
            }
        })
    }

    fn build_image_workflow(&self, prompt: &str, negative_prompt: &str) -> Value {
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
            "4": { "class_type": "CheckpointLoaderSimple", "inputs": { "ckpt_name": self.config.default_checkpoint } },
            "5": { "class_type": "EmptyLatentImage", "inputs": { "batch_size": 1, "height": 512, "width": 512 } },
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
                    category: Some("file".into()),
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

            if let Some(entry) = history.get(prompt_id) {
                if let Some(outputs) = entry.outputs.values().next() {
                    if let Some(images) = &outputs.images {
                        return Ok(images.clone());
                    }
                }
            }

            // AnimateDiff needs ~2-3 mins for 16 frames, allow up to 5 mins
            if attempts > 300 {
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

// ComfyUI returns history as { "prompt_id": { ... } } without a wrapper
type HistoryResponse = HashMap<String, PromptHistoryEntry>;

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
