//! Brand Tools Implementation
//!
//! Tools for image generation, logo design, color systems, and typography.
//! Used primarily by Genesis and Maci agents.
//!
//! # Status: Partially Integrated
//!
//! ComfyUI integration is functional. Some helper functions reserved for future use.
//! TODO(agent-tools): Complete accessibility checker implementation
//! TODO(agent-tools): Wire font pairing suggestions

#![allow(dead_code)]
#![allow(unused_variables)]

use crate::services::agent_tools::ToolResult;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

// ============================================================================
// ComfyUI Client
// ============================================================================

/// ComfyUI server configuration
#[derive(Debug, Clone)]
pub struct ComfyUIConfig {
    pub server_url: String,
    pub default_model: String,
    pub output_dir: String,
    pub default_steps: u32,
    pub default_cfg: f32,
}

impl Default for ComfyUIConfig {
    fn default() -> Self {
        Self {
            server_url: "http://127.0.0.1:8188".to_string(),
            default_model: "flux1-dev.safetensors".to_string(),
            output_dir: "/home/spaceterminal/topos/ComfyUI/output".to_string(),
            default_steps: 30,
            default_cfg: 7.5,
        }
    }
}

/// Image generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationRequest {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub width: u32,
    pub height: u32,
    pub steps: Option<u32>,
    pub cfg_scale: Option<f32>,
    pub seed: Option<i64>,
    pub model: Option<String>,
    pub style_preset: Option<String>,
}

impl ImageGenerationRequest {
    pub fn new(prompt: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            negative_prompt: None,
            width: 1024,
            height: 1024,
            steps: None,
            cfg_scale: None,
            seed: None,
            model: None,
            style_preset: None,
        }
    }

    pub fn with_aspect_ratio(mut self, ratio: &str) -> Self {
        match ratio {
            "1:1" => { self.width = 1024; self.height = 1024; }
            "16:9" => { self.width = 1344; self.height = 768; }
            "9:16" => { self.width = 768; self.height = 1344; }
            "4:3" => { self.width = 1152; self.height = 896; }
            "3:4" => { self.width = 896; self.height = 1152; }
            "21:9" => { self.width = 1536; self.height = 640; }
            _ => {}
        }
        self
    }

    pub fn with_style(mut self, style: &str) -> Self {
        self.style_preset = Some(style.to_string());

        // Add style-specific prompt modifications
        let style_suffix = match style {
            "cinematic" => ", cinematic lighting, dramatic composition, film grain, movie still",
            "commercial" => ", professional photography, studio lighting, commercial quality, clean aesthetic",
            "editorial" => ", editorial photography, magazine quality, artistic composition",
            "abstract" => ", abstract art, geometric shapes, modern art, vibrant colors",
            "minimal" => ", minimalist design, clean lines, white space, simple composition",
            "bold" => ", bold colors, high contrast, striking imagery, impactful design",
            _ => "",
        };

        self.prompt = format!("{}{}", self.prompt, style_suffix);
        self
    }
}

/// Image generation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    pub image_path: String,
    pub image_url: Option<String>,
    pub prompt_used: String,
    pub seed: i64,
    pub width: u32,
    pub height: u32,
    pub model: String,
    pub generation_time_ms: u64,
}

/// ComfyUI API client
pub struct ComfyUIClient {
    config: ComfyUIConfig,
    client: Client,
}

impl ComfyUIClient {
    pub fn new(config: ComfyUIConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// Check if ComfyUI server is running
    pub async fn health_check(&self) -> bool {
        self.client
            .get(format!("{}/system_stats", self.config.server_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Generate an image using ComfyUI
    pub async fn generate_image(&self, request: ImageGenerationRequest) -> Result<GeneratedImage, ComfyUIError> {
        let start = Instant::now();

        // Build the workflow
        let workflow = self.build_flux_workflow(&request);

        // Queue the prompt
        let prompt_response = self.queue_prompt(&workflow).await?;
        let prompt_id = prompt_response.prompt_id;

        // Wait for completion
        let output_images = self.wait_for_completion(&prompt_id).await?;

        // Get the first output image
        let image_info = output_images.first()
            .ok_or(ComfyUIError::NoOutput)?;

        Ok(GeneratedImage {
            image_path: image_info.filename.clone(),
            image_url: Some(format!(
                "{}/view?filename={}&subfolder={}&type=output",
                self.config.server_url,
                image_info.filename,
                image_info.subfolder.as_deref().unwrap_or("")
            )),
            prompt_used: request.prompt.clone(),
            seed: request.seed.unwrap_or_else(|| rand::random()),
            width: request.width,
            height: request.height,
            model: request.model.unwrap_or_else(|| self.config.default_model.clone()),
            generation_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn build_flux_workflow(&self, request: &ImageGenerationRequest) -> serde_json::Value {
        let seed = request.seed.unwrap_or_else(|| rand::random::<i64>().abs());
        let steps = request.steps.unwrap_or(self.config.default_steps);
        let cfg = request.cfg_scale.unwrap_or(self.config.default_cfg);
        let model = request.model.as_deref().unwrap_or(&self.config.default_model);

        // Flux workflow (simplified)
        serde_json::json!({
            "3": {
                "class_type": "KSampler",
                "inputs": {
                    "cfg": cfg,
                    "denoise": 1.0,
                    "latent_image": ["5", 0],
                    "model": ["4", 0],
                    "negative": ["7", 0],
                    "positive": ["6", 0],
                    "sampler_name": "euler",
                    "scheduler": "normal",
                    "seed": seed,
                    "steps": steps
                }
            },
            "4": {
                "class_type": "CheckpointLoaderSimple",
                "inputs": {
                    "ckpt_name": model
                }
            },
            "5": {
                "class_type": "EmptyLatentImage",
                "inputs": {
                    "batch_size": 1,
                    "height": request.height,
                    "width": request.width
                }
            },
            "6": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "clip": ["4", 1],
                    "text": request.prompt
                }
            },
            "7": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "clip": ["4", 1],
                    "text": request.negative_prompt.as_deref().unwrap_or("blurry, low quality, distorted")
                }
            },
            "8": {
                "class_type": "VAEDecode",
                "inputs": {
                    "samples": ["3", 0],
                    "vae": ["4", 2]
                }
            },
            "9": {
                "class_type": "SaveImage",
                "inputs": {
                    "filename_prefix": "PCG",
                    "images": ["8", 0]
                }
            }
        })
    }

    async fn queue_prompt(&self, workflow: &serde_json::Value) -> Result<PromptResponse, ComfyUIError> {
        #[derive(Serialize)]
        struct PromptRequest {
            prompt: serde_json::Value,
        }

        let response = self.client
            .post(format!("{}/prompt", self.config.server_url))
            .json(&PromptRequest { prompt: workflow.clone() })
            .send()
            .await
            .map_err(|e| ComfyUIError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ComfyUIError::ApiError(format!("Queue failed: {}", response.status())));
        }

        response.json().await
            .map_err(|e| ComfyUIError::ParseError(e.to_string()))
    }

    async fn wait_for_completion(&self, prompt_id: &str) -> Result<Vec<OutputImage>, ComfyUIError> {
        // Poll for completion
        let max_attempts = 120; // 2 minutes max
        for _ in 0..max_attempts {
            let history = self.get_history(prompt_id).await?;

            if let Some(prompt_data) = history.get(prompt_id) {
                if let Some(outputs) = prompt_data.get("outputs") {
                    // Find the SaveImage node output
                    for (_, node_output) in outputs.as_object().unwrap_or(&serde_json::Map::new()) {
                        if let Some(images) = node_output.get("images") {
                            let images: Vec<OutputImage> = serde_json::from_value(images.clone())
                                .unwrap_or_default();
                            if !images.is_empty() {
                                return Ok(images);
                            }
                        }
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        Err(ComfyUIError::Timeout)
    }

    async fn get_history(&self, prompt_id: &str) -> Result<HashMap<String, serde_json::Value>, ComfyUIError> {
        let response = self.client
            .get(format!("{}/history/{}", self.config.server_url, prompt_id))
            .send()
            .await
            .map_err(|e| ComfyUIError::RequestFailed(e.to_string()))?;

        response.json().await
            .map_err(|e| ComfyUIError::ParseError(e.to_string()))
    }
}

#[derive(Debug, Deserialize)]
struct PromptResponse {
    prompt_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct OutputImage {
    filename: String,
    subfolder: Option<String>,
    #[serde(rename = "type")]
    image_type: Option<String>,
}

#[derive(Debug)]
pub enum ComfyUIError {
    RequestFailed(String),
    ApiError(String),
    ParseError(String),
    Timeout,
    NoOutput,
}

// ============================================================================
// Color System
// ============================================================================

/// A color in multiple formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color {
    pub hex: String,
    pub rgb: RGB,
    pub hsl: HSL,
    pub name: Option<String>,
    pub usage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HSL {
    pub h: f32,
    pub s: f32,
    pub l: f32,
}

/// Color palette
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub neutrals: Vec<Color>,
    pub semantic: SemanticColors,
    pub accessibility_report: AccessibilityReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticColors {
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityReport {
    pub wcag_aa_compliant: bool,
    pub wcag_aaa_compliant: bool,
    pub contrast_ratios: HashMap<String, f32>,
    pub issues: Vec<String>,
}

/// Color system utilities
pub struct ColorSystem;

impl ColorSystem {
    /// Generate a color palette based on personality and industry
    pub fn generate_palette(
        personality: &[String],
        industry: &str,
        base_color: Option<&str>,
    ) -> ColorPalette {
        // Determine primary color based on personality and industry
        let primary_hex = base_color
            .map(|s| s.to_string())
            .unwrap_or_else(|| Self::personality_to_color(personality, industry));

        let primary = Self::hex_to_color(&primary_hex, Some("Primary"), Some("Main brand color"));

        // Generate harmonious colors
        let secondary = Self::generate_complementary(&primary, 30.0);
        let accent = Self::generate_complementary(&primary, 180.0);

        // Generate neutrals
        let neutrals = Self::generate_neutral_scale(&primary);

        // Generate semantic colors
        let semantic = SemanticColors {
            success: Self::hex_to_color("#22C55E", Some("Success"), Some("Positive actions")),
            warning: Self::hex_to_color("#F59E0B", Some("Warning"), Some("Caution states")),
            error: Self::hex_to_color("#EF4444", Some("Error"), Some("Error states")),
            info: Self::hex_to_color("#3B82F6", Some("Info"), Some("Informational")),
        };

        // Check accessibility
        let accessibility_report = Self::check_accessibility(&primary, &secondary, &neutrals);

        ColorPalette {
            primary,
            secondary,
            accent,
            neutrals,
            semantic,
            accessibility_report,
        }
    }

    fn personality_to_color(personality: &[String], industry: &str) -> String {
        // Map personality traits to color hues
        let personality_lower: Vec<String> = personality.iter().map(|s| s.to_lowercase()).collect();

        // Check for specific traits
        if personality_lower.iter().any(|p| p.contains("bold") || p.contains("energetic")) {
            return "#EF4444".to_string(); // Red
        }
        if personality_lower.iter().any(|p| p.contains("trust") || p.contains("reliable") || p.contains("professional")) {
            return "#3B82F6".to_string(); // Blue
        }
        if personality_lower.iter().any(|p| p.contains("growth") || p.contains("natural") || p.contains("sustainable")) {
            return "#22C55E".to_string(); // Green
        }
        if personality_lower.iter().any(|p| p.contains("creative") || p.contains("innovative") || p.contains("unique")) {
            return "#8B5CF6".to_string(); // Purple
        }
        if personality_lower.iter().any(|p| p.contains("warm") || p.contains("friendly") || p.contains("approachable")) {
            return "#F97316".to_string(); // Orange
        }
        if personality_lower.iter().any(|p| p.contains("premium") || p.contains("luxury") || p.contains("sophisticated")) {
            return "#1F2937".to_string(); // Dark gray/black
        }

        // Industry defaults
        match industry.to_lowercase().as_str() {
            "technology" | "tech" | "software" | "saas" => "#3B82F6".to_string(),
            "healthcare" | "health" | "medical" => "#06B6D4".to_string(),
            "finance" | "fintech" | "banking" => "#1E40AF".to_string(),
            "environment" | "sustainability" | "eco" => "#22C55E".to_string(),
            "food" | "restaurant" | "culinary" => "#F97316".to_string(),
            "fashion" | "beauty" | "lifestyle" => "#EC4899".to_string(),
            "education" | "learning" => "#8B5CF6".to_string(),
            _ => "#3B82F6".to_string(), // Default blue
        }
    }

    fn hex_to_color(hex: &str, name: Option<&str>, usage: Option<&str>) -> Color {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);

        let rgb = RGB { r, g, b };
        let hsl = Self::rgb_to_hsl(&rgb);

        Color {
            hex: format!("#{}", hex.to_uppercase()),
            rgb,
            hsl,
            name: name.map(|s| s.to_string()),
            usage: usage.map(|s| s.to_string()),
        }
    }

    fn rgb_to_hsl(rgb: &RGB) -> HSL {
        let r = rgb.r as f32 / 255.0;
        let g = rgb.g as f32 / 255.0;
        let b = rgb.b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;

        if max == min {
            return HSL { h: 0.0, s: 0.0, l };
        }

        let d = max - min;
        let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };

        let h = if max == r {
            (g - b) / d + (if g < b { 6.0 } else { 0.0 })
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };

        HSL { h: h * 60.0, s, l }
    }

    fn hsl_to_rgb(hsl: &HSL) -> RGB {
        let h = hsl.h / 360.0;
        let s = hsl.s;
        let l = hsl.l;

        if s == 0.0 {
            let v = (l * 255.0) as u8;
            return RGB { r: v, g: v, b: v };
        }

        let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        let p = 2.0 * l - q;

        let hue_to_rgb = |p: f32, q: f32, mut t: f32| -> f32 {
            if t < 0.0 { t += 1.0; }
            if t > 1.0 { t -= 1.0; }
            if t < 1.0/6.0 { return p + (q - p) * 6.0 * t; }
            if t < 1.0/2.0 { return q; }
            if t < 2.0/3.0 { return p + (q - p) * (2.0/3.0 - t) * 6.0; }
            p
        };

        RGB {
            r: (hue_to_rgb(p, q, h + 1.0/3.0) * 255.0) as u8,
            g: (hue_to_rgb(p, q, h) * 255.0) as u8,
            b: (hue_to_rgb(p, q, h - 1.0/3.0) * 255.0) as u8,
        }
    }

    fn generate_complementary(base: &Color, hue_shift: f32) -> Color {
        let mut hsl = base.hsl.clone();
        hsl.h = (hsl.h + hue_shift) % 360.0;

        let rgb = Self::hsl_to_rgb(&hsl);
        let hex = format!("#{:02X}{:02X}{:02X}", rgb.r, rgb.g, rgb.b);

        Color {
            hex,
            rgb,
            hsl,
            name: Some("Secondary".to_string()),
            usage: Some("Supporting brand color".to_string()),
        }
    }

    fn generate_neutral_scale(base: &Color) -> Vec<Color> {
        let base_hsl = &base.hsl;

        // Generate a neutral scale with a hint of the brand color
        [50, 100, 200, 300, 400, 500, 600, 700, 800, 900]
            .iter()
            .map(|&weight| {
                let l = 1.0 - (weight as f32 / 1000.0);
                let s = 0.05; // Slight saturation for warmth

                let hsl = HSL { h: base_hsl.h, s, l };
                let rgb = Self::hsl_to_rgb(&hsl);
                let hex = format!("#{:02X}{:02X}{:02X}", rgb.r, rgb.g, rgb.b);

                Color {
                    hex,
                    rgb,
                    hsl,
                    name: Some(format!("Neutral-{}", weight)),
                    usage: Some(format!("Neutral shade {}", weight)),
                }
            })
            .collect()
    }

    fn check_accessibility(primary: &Color, secondary: &Color, neutrals: &[Color]) -> AccessibilityReport {
        let mut contrast_ratios = HashMap::new();
        let mut issues = vec![];

        // Calculate contrast ratios
        let white = RGB { r: 255, g: 255, b: 255 };
        let black = RGB { r: 0, g: 0, b: 0 };

        let primary_white = Self::contrast_ratio(&primary.rgb, &white);
        let primary_black = Self::contrast_ratio(&primary.rgb, &black);

        contrast_ratios.insert("primary_on_white".to_string(), primary_white);
        contrast_ratios.insert("primary_on_black".to_string(), primary_black);

        // Check WCAG compliance
        let mut wcag_aa = true;
        let mut wcag_aaa = true;

        if primary_white < 4.5 && primary_black < 4.5 {
            issues.push("Primary color may have contrast issues with text".to_string());
            wcag_aa = false;
        }

        if primary_white < 7.0 && primary_black < 7.0 {
            wcag_aaa = false;
        }

        AccessibilityReport {
            wcag_aa_compliant: wcag_aa,
            wcag_aaa_compliant: wcag_aaa,
            contrast_ratios,
            issues,
        }
    }

    fn contrast_ratio(c1: &RGB, c2: &RGB) -> f32 {
        let l1 = Self::relative_luminance(c1);
        let l2 = Self::relative_luminance(c2);

        let lighter = l1.max(l2);
        let darker = l1.min(l2);

        (lighter + 0.05) / (darker + 0.05)
    }

    fn relative_luminance(rgb: &RGB) -> f32 {
        let r = Self::srgb_to_linear(rgb.r as f32 / 255.0);
        let g = Self::srgb_to_linear(rgb.g as f32 / 255.0);
        let b = Self::srgb_to_linear(rgb.b as f32 / 255.0);

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    fn srgb_to_linear(value: f32) -> f32 {
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
}

// ============================================================================
// Typography Tools
// ============================================================================

/// Font pairing recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontPairing {
    pub heading_font: FontInfo,
    pub body_font: FontInfo,
    pub accent_font: Option<FontInfo>,
    pub type_scale: TypeScale,
    pub pairing_rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontInfo {
    pub name: String,
    pub category: String, // serif, sans-serif, display, monospace
    pub weights: Vec<u32>,
    pub source: String, // google, adobe, system
    pub css_import: Option<String>,
    pub fallback_stack: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeScale {
    pub base_size: u32,
    pub scale_ratio: f32,
    pub sizes: HashMap<String, TypeSize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSize {
    pub size_px: u32,
    pub size_rem: f32,
    pub line_height: f32,
    pub letter_spacing: f32,
}

/// Typography selection tools
pub struct TypographyTools;

impl TypographyTools {
    /// Select typography based on brand personality
    pub fn select_fonts(
        personality: &[String],
        style_preference: &str,
        require_free: bool,
    ) -> FontPairing {
        let personality_lower: Vec<String> = personality.iter().map(|s| s.to_lowercase()).collect();

        // Determine heading font based on personality
        let heading_font = if personality_lower.iter().any(|p| p.contains("modern") || p.contains("tech")) {
            Self::get_font("Inter", "sans-serif")
        } else if personality_lower.iter().any(|p| p.contains("elegant") || p.contains("sophisticated")) {
            Self::get_font("Playfair Display", "serif")
        } else if personality_lower.iter().any(|p| p.contains("bold") || p.contains("strong")) {
            Self::get_font("Montserrat", "sans-serif")
        } else if personality_lower.iter().any(|p| p.contains("friendly") || p.contains("approachable")) {
            Self::get_font("Poppins", "sans-serif")
        } else {
            match style_preference {
                "serif" => Self::get_font("Merriweather", "serif"),
                "sans-serif" => Self::get_font("Inter", "sans-serif"),
                "display" => Self::get_font("Space Grotesk", "sans-serif"),
                _ => Self::get_font("Inter", "sans-serif"),
            }
        };

        // Pair with complementary body font
        let body_font = Self::pair_body_font(&heading_font);

        // Generate type scale
        let type_scale = Self::generate_type_scale(16, 1.25);

        let rationale = format!(
            "{} provides {} presence for headings, while {} ensures excellent readability for body text. \
            This pairing balances {} with clarity.",
            heading_font.name,
            if heading_font.category == "serif" { "an elegant" } else { "a modern" },
            body_font.name,
            if heading_font.category == "serif" { "sophistication" } else { "contemporary style" }
        );

        FontPairing {
            heading_font,
            body_font,
            accent_font: None,
            type_scale,
            pairing_rationale: rationale,
        }
    }

    fn get_font(name: &str, category: &str) -> FontInfo {
        let weights = match name {
            "Inter" => vec![400, 500, 600, 700],
            "Playfair Display" => vec![400, 500, 600, 700],
            "Montserrat" => vec![400, 500, 600, 700, 800],
            "Poppins" => vec![400, 500, 600, 700],
            "Merriweather" => vec![400, 700],
            "Space Grotesk" => vec![400, 500, 600, 700],
            _ => vec![400, 700],
        };

        let fallback = match category {
            "serif" => "Georgia, 'Times New Roman', serif",
            "sans-serif" => "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
            "monospace" => "'SF Mono', Monaco, monospace",
            _ => "sans-serif",
        };

        FontInfo {
            name: name.to_string(),
            category: category.to_string(),
            weights,
            source: "google".to_string(),
            css_import: Some(format!(
                "@import url('https://fonts.googleapis.com/css2?family={}:wght@400;500;600;700&display=swap');",
                name.replace(' ', "+")
            )),
            fallback_stack: fallback.to_string(),
        }
    }

    fn pair_body_font(heading: &FontInfo) -> FontInfo {
        // Pair serif headings with sans-serif body and vice versa for contrast
        // Or use complementary weights of same family for cohesion
        match heading.name.as_str() {
            "Playfair Display" => Self::get_font("Source Sans Pro", "sans-serif"),
            "Merriweather" => Self::get_font("Open Sans", "sans-serif"),
            "Montserrat" => Self::get_font("Open Sans", "sans-serif"),
            "Poppins" => Self::get_font("Inter", "sans-serif"),
            "Space Grotesk" => Self::get_font("Inter", "sans-serif"),
            _ => Self::get_font("Inter", "sans-serif"),
        }
    }

    fn generate_type_scale(base_size: u32, scale_ratio: f32) -> TypeScale {
        let mut sizes = HashMap::new();

        let scale_names = ["xs", "sm", "base", "lg", "xl", "2xl", "3xl", "4xl", "5xl"];
        let scale_multipliers = [
            scale_ratio.powi(-2),
            scale_ratio.powi(-1),
            1.0,
            scale_ratio,
            scale_ratio.powi(2),
            scale_ratio.powi(3),
            scale_ratio.powi(4),
            scale_ratio.powi(5),
            scale_ratio.powi(6),
        ];

        for (name, multiplier) in scale_names.iter().zip(scale_multipliers.iter()) {
            let size_px = (base_size as f32 * multiplier).round() as u32;
            let size_rem = size_px as f32 / 16.0;

            // Line height decreases as size increases
            let line_height = if size_px < 20 { 1.6 } else if size_px < 32 { 1.4 } else { 1.2 };

            // Letter spacing adjusts for size
            let letter_spacing = if size_px > 32 { -0.02 } else { 0.0 };

            sizes.insert(name.to_string(), TypeSize {
                size_px,
                size_rem,
                line_height,
                letter_spacing,
            });
        }

        TypeScale {
            base_size,
            scale_ratio,
            sizes,
        }
    }
}

// ============================================================================
// Brand Tools (High-level interface)
// ============================================================================

/// High-level brand tools interface
pub struct BrandTools {
    pub comfyui: Option<ComfyUIClient>,
    pub color_system: ColorSystem,
    pub typography: TypographyTools,
}

impl BrandTools {
    pub fn new(comfyui_config: Option<ComfyUIConfig>) -> Self {
        Self {
            comfyui: comfyui_config.map(ComfyUIClient::new),
            color_system: ColorSystem,
            typography: TypographyTools,
        }
    }

    /// Execute a brand tool by name
    pub async fn execute(&self, tool_name: &str, params: serde_json::Value) -> ToolResult {
        let start = Instant::now();

        let result = match tool_name {
            "generate_image" => self.execute_generate_image(params).await,
            "generate_color_palette" => self.execute_generate_palette(params).await,
            "select_typography" => self.execute_select_typography(params).await,
            _ => Err(format!("Unknown tool: {}", tool_name)),
        };

        match result {
            Ok(data) => ToolResult {
                success: true,
                tool_name: tool_name.to_string(),
                agent_name: "brand".to_string(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                data,
                error: None,
            },
            Err(error) => ToolResult {
                success: false,
                tool_name: tool_name.to_string(),
                agent_name: "brand".to_string(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                data: serde_json::json!({}),
                error: Some(error),
            },
        }
    }

    async fn execute_generate_image(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let comfyui = self.comfyui.as_ref()
            .ok_or("ComfyUI not configured")?;

        let prompt = params.get("prompt")
            .and_then(|v| v.as_str())
            .ok_or("Missing prompt parameter")?;

        let mut request = ImageGenerationRequest::new(prompt);

        if let Some(neg) = params.get("negative_prompt").and_then(|v| v.as_str()) {
            request.negative_prompt = Some(neg.to_string());
        }

        if let Some(style) = params.get("style").and_then(|v| v.as_str()) {
            request = request.with_style(style);
        }

        if let Some(ratio) = params.get("aspect_ratio").and_then(|v| v.as_str()) {
            request = request.with_aspect_ratio(ratio);
        }

        if let Some(steps) = params.get("steps").and_then(|v| v.as_u64()) {
            request.steps = Some(steps as u32);
        }

        let result = comfyui.generate_image(request).await
            .map_err(|e| format!("Image generation failed: {:?}", e))?;

        Ok(serde_json::to_value(result).unwrap())
    }

    async fn execute_generate_palette(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let personality: Vec<String> = params.get("brand_personality")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let industry = params.get("industry")
            .and_then(|v| v.as_str())
            .unwrap_or("technology");

        let base_color = params.get("base_color")
            .and_then(|v| v.as_str());

        let palette = ColorSystem::generate_palette(&personality, industry, base_color);

        Ok(serde_json::to_value(palette).unwrap())
    }

    async fn execute_select_typography(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let personality: Vec<String> = params.get("brand_personality")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let style = params.get("style_preference")
            .and_then(|v| v.as_str())
            .unwrap_or("sans-serif");

        let require_free = params.get("require_free_fonts")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let pairing = TypographyTools::select_fonts(&personality, style, require_free);

        Ok(serde_json::to_value(pairing).unwrap())
    }
}
