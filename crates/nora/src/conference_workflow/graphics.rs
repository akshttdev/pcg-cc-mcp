//! Graphics Composition for Conference Thumbnails
//!
//! Composes thumbnails using real-world assets from the knowledge graph:
//! - Speaker photos from Entity.photo_url
//! - Sponsor/brand logos from Entity.photo_url
//! - Conference logo (if available)
//! - PCG branding
//!
//! AI backgrounds are generated via Maci agent (ComfyUI/SDXL).

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

use db::models::entity::EntityType;
use cinematics::CinematicsService;

use crate::{NoraError, Result};

use super::engine::ResearchFlowResult;

/// Thumbnail dimensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub fn landscape_16_9() -> Self {
        Self { width: 1792, height: 1024 }
    }

    pub fn square() -> Self {
        Self { width: 1024, height: 1024 }
    }

    pub fn og_image() -> Self {
        Self { width: 1200, height: 630 }
    }
}

/// A positioned asset in the composition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionedAsset {
    pub url: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub border_radius: Option<u32>,
    pub opacity: f32,
    pub z_index: i32,
}

/// Text overlay in the composition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextOverlay {
    pub text: String,
    pub x: u32,
    pub y: u32,
    pub font_size: u32,
    pub font_weight: String,
    pub color: String,
    pub max_width: Option<u32>,
    pub text_align: String,
    pub shadow: Option<TextShadow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextShadow {
    pub offset_x: i32,
    pub offset_y: i32,
    pub blur: u32,
    pub color: String,
}

/// Background specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BackgroundSpec {
    /// Solid color background
    Solid { color: String },
    /// Gradient background
    Gradient {
        colors: Vec<String>,
        direction: String, // "to-right", "to-bottom", "diagonal"
    },
    /// AI-generated background
    Generated {
        prompt: String,
        url: Option<String>, // Filled after generation
    },
    /// Use an existing image
    Image { url: String },
}

/// Complete thumbnail composition specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailComposition {
    pub dimensions: Dimensions,
    pub background: BackgroundSpec,
    pub assets: Vec<PositionedAsset>,
    pub text_overlays: Vec<TextOverlay>,
    pub branding: BrandingSpec,
}

/// Branding elements (logos at bottom)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingSpec {
    pub conference_logo_url: Option<String>,
    pub pcg_logo_url: Option<String>,
    pub position: String, // "bottom-left", "bottom-center", "bottom-right"
}

/// Assets collected from the knowledge graph
#[derive(Debug, Clone)]
pub struct CollectedAssets {
    pub speaker_photos: Vec<(String, String)>, // (name, photo_url)
    pub sponsor_logos: Vec<(String, String)>,  // (name, logo_url)
    pub conference_logo: Option<String>,
    pub venue_images: Vec<String>,
}

/// Graphics composer for conference thumbnails
pub struct GraphicsComposer {
    #[allow(dead_code)]
    pool: SqlitePool,
    cinematics: Option<Arc<CinematicsService>>,
}

impl GraphicsComposer {
    pub fn new(pool: SqlitePool, cinematics: Option<Arc<CinematicsService>>) -> Self {
        Self {
            pool,
            cinematics,
        }
    }

    /// Generate an image using Maci/ComfyUI or return None if unavailable
    async fn generate_image(&self, prompt: &str, width: u32, height: u32) -> Option<String> {
        if let Some(ref cinematics) = self.cinematics {
            match cinematics.generate_static_image_url(prompt, width, height).await {
                Ok(url) => {
                    tracing::info!("[GRAPHICS] Generated image via Maci/ComfyUI: {}...", &url[..url.len().min(60)]);
                    Some(url)
                }
                Err(e) => {
                    tracing::warn!("[GRAPHICS] Maci image generation failed: {}", e);
                    None
                }
            }
        } else {
            tracing::debug!("[GRAPHICS] CinematicsService not available, using gradient fallback");
            None
        }
    }

    /// Collect all available assets from entities
    pub fn collect_assets(&self, research: &ResearchFlowResult) -> CollectedAssets {
        let mut speaker_photos = Vec::new();
        let mut sponsor_logos = Vec::new();
        let mut venue_images = Vec::new();

        for entity in &research.entities {
            if let Some(ref photo_url) = entity.photo_url {
                match entity.entity_type {
                    EntityType::Speaker => {
                        speaker_photos.push((entity.canonical_name.clone(), photo_url.clone()));
                    }
                    EntityType::Sponsor | EntityType::Brand => {
                        sponsor_logos.push((entity.canonical_name.clone(), photo_url.clone()));
                    }
                    EntityType::Venue => {
                        venue_images.push(photo_url.clone());
                    }
                    _ => {}
                }
            }
        }

        CollectedAssets {
            speaker_photos,
            sponsor_logos,
            conference_logo: None, // Would come from workflow or project settings
            venue_images,
        }
    }

    /// Compose a Speakers Article thumbnail
    pub async fn compose_speakers_thumbnail(
        &self,
        conference_name: &str,
        title: &str,
        assets: &CollectedAssets,
    ) -> Result<ThumbnailComposition> {
        let dimensions = Dimensions::landscape_16_9();

        // Generate background if we don't have venue images
        let background = if let Some(venue_url) = assets.venue_images.first() {
            BackgroundSpec::Image { url: venue_url.clone() }
        } else {
            // Generate a tech conference background via Maci/ComfyUI
            let prompt = format!(
                "Abstract tech conference stage background, modern auditorium with \
                dramatic lighting, purple and blue gradient, bokeh lights, \
                professional speaker event atmosphere, no people, no text, \
                minimalist design for {}",
                conference_name
            );

            match self.generate_image(&prompt, dimensions.width, dimensions.height).await {
                Some(url) => BackgroundSpec::Generated { prompt, url: Some(url) },
                None => BackgroundSpec::Gradient {
                    colors: vec!["#1a1a2e".to_string(), "#16213e".to_string(), "#0f3460".to_string()],
                    direction: "diagonal".to_string(),
                },
            }
        };

        // Position speaker photos in a row
        let mut positioned_assets = Vec::new();
        let photo_size = 180u32;
        let photos_to_show = assets.speaker_photos.iter().take(5).collect::<Vec<_>>();
        let total_width = photos_to_show.len() as u32 * (photo_size + 20);
        let start_x = (dimensions.width - total_width) / 2;

        for (idx, (_, photo_url)) in photos_to_show.iter().enumerate() {
            positioned_assets.push(PositionedAsset {
                url: photo_url.clone(),
                x: start_x + idx as u32 * (photo_size + 20),
                y: 400,
                width: photo_size,
                height: photo_size,
                border_radius: Some(photo_size / 2), // Circular
                opacity: 1.0,
                z_index: 10,
            });
        }

        // Title text overlay
        let text_overlays = vec![
            TextOverlay {
                text: title.to_string(),
                x: dimensions.width / 2,
                y: 200,
                font_size: 72,
                font_weight: "bold".to_string(),
                color: "#ffffff".to_string(),
                max_width: Some(dimensions.width - 100),
                text_align: "center".to_string(),
                shadow: Some(TextShadow {
                    offset_x: 2,
                    offset_y: 2,
                    blur: 8,
                    color: "rgba(0,0,0,0.5)".to_string(),
                }),
            },
            TextOverlay {
                text: conference_name.to_string(),
                x: dimensions.width / 2,
                y: 300,
                font_size: 36,
                font_weight: "normal".to_string(),
                color: "#cccccc".to_string(),
                max_width: Some(dimensions.width - 100),
                text_align: "center".to_string(),
                shadow: None,
            },
        ];

        Ok(ThumbnailComposition {
            dimensions,
            background,
            assets: positioned_assets,
            text_overlays,
            branding: BrandingSpec {
                conference_logo_url: assets.conference_logo.clone(),
                pcg_logo_url: Some("https://pcg.media/logo.png".to_string()), // PCG branding
                position: "bottom-center".to_string(),
            },
        })
    }

    /// Compose a Side Events Article thumbnail with sponsor logos
    pub async fn compose_side_events_thumbnail(
        &self,
        conference_name: &str,
        title: &str,
        assets: &CollectedAssets,
        event_count: usize,
    ) -> Result<ThumbnailComposition> {
        let dimensions = Dimensions::landscape_16_9();

        // Vibrant background for events via Maci/ComfyUI
        let prompt = format!(
            "Vibrant nightlife city scene, neon lights, networking event atmosphere, \
            social gathering energy, purple pink and blue colors, bokeh city lights, \
            no people, no text, abstract event poster background for {}",
            conference_name
        );

        let background = match self.generate_image(&prompt, dimensions.width, dimensions.height).await {
            Some(url) => BackgroundSpec::Generated { prompt, url: Some(url) },
            None => BackgroundSpec::Gradient {
                colors: vec!["#ff6b6b".to_string(), "#845ec2".to_string(), "#0081cf".to_string()],
                direction: "diagonal".to_string(),
            },
        };

        // Position sponsor logos in a grid at bottom
        let mut positioned_assets = Vec::new();
        let logo_size = 100u32;
        let logos_to_show = assets.sponsor_logos.iter().take(6).collect::<Vec<_>>();
        let logos_per_row = 3;
        let total_width = logos_per_row as u32 * (logo_size + 30);
        let start_x = (dimensions.width - total_width) / 2;

        for (idx, (_, logo_url)) in logos_to_show.iter().enumerate() {
            let row = idx / logos_per_row;
            let col = idx % logos_per_row;
            positioned_assets.push(PositionedAsset {
                url: logo_url.clone(),
                x: start_x + col as u32 * (logo_size + 30),
                y: 600 + row as u32 * (logo_size + 20),
                width: logo_size,
                height: logo_size,
                border_radius: Some(10),
                opacity: 0.9,
                z_index: 10,
            });
        }

        let text_overlays = vec![
            TextOverlay {
                text: title.to_string(),
                x: dimensions.width / 2,
                y: 180,
                font_size: 64,
                font_weight: "bold".to_string(),
                color: "#ffffff".to_string(),
                max_width: Some(dimensions.width - 100),
                text_align: "center".to_string(),
                shadow: Some(TextShadow {
                    offset_x: 2,
                    offset_y: 2,
                    blur: 8,
                    color: "rgba(0,0,0,0.6)".to_string(),
                }),
            },
            TextOverlay {
                text: format!("{} Events Around {}", event_count, conference_name),
                x: dimensions.width / 2,
                y: 280,
                font_size: 32,
                font_weight: "normal".to_string(),
                color: "#ffdd59".to_string(),
                max_width: Some(dimensions.width - 100),
                text_align: "center".to_string(),
                shadow: None,
            },
        ];

        Ok(ThumbnailComposition {
            dimensions,
            background,
            assets: positioned_assets,
            text_overlays,
            branding: BrandingSpec {
                conference_logo_url: assets.conference_logo.clone(),
                pcg_logo_url: Some("https://pcg.media/logo.png".to_string()),
                position: "bottom-right".to_string(),
            },
        })
    }

    /// Compose a Press Release thumbnail
    pub async fn compose_press_release_thumbnail(
        &self,
        conference_name: &str,
        title: &str,
        assets: &CollectedAssets,
    ) -> Result<ThumbnailComposition> {
        let dimensions = Dimensions::landscape_16_9();

        // Professional, clean background for press via Maci/ComfyUI
        let prompt = format!(
            "Professional corporate press conference background, clean modern design, \
            subtle gradient, press room aesthetic, microphone podium silhouette, \
            blue and white corporate colors, no people, no text, \
            media announcement style for {}",
            conference_name
        );

        let background = match self.generate_image(&prompt, dimensions.width, dimensions.height).await {
            Some(url) => BackgroundSpec::Generated { prompt, url: Some(url) },
            None => BackgroundSpec::Gradient {
                colors: vec!["#1e3a5f".to_string(), "#2c5282".to_string(), "#3182ce".to_string()],
                direction: "to-right".to_string(),
            },
        };

        // Conference logo prominently displayed if available
        let mut positioned_assets = Vec::new();
        if let Some(ref logo_url) = assets.conference_logo {
            positioned_assets.push(PositionedAsset {
                url: logo_url.clone(),
                x: dimensions.width / 2 - 150,
                y: 350,
                width: 300,
                height: 150,
                border_radius: None,
                opacity: 1.0,
                z_index: 10,
            });
        }

        let text_overlays = vec![
            TextOverlay {
                text: "PRESS RELEASE".to_string(),
                x: dimensions.width / 2,
                y: 120,
                font_size: 28,
                font_weight: "bold".to_string(),
                color: "#90cdf4".to_string(),
                max_width: None,
                text_align: "center".to_string(),
                shadow: None,
            },
            TextOverlay {
                text: title.to_string(),
                x: dimensions.width / 2,
                y: 220,
                font_size: 56,
                font_weight: "bold".to_string(),
                color: "#ffffff".to_string(),
                max_width: Some(dimensions.width - 150),
                text_align: "center".to_string(),
                shadow: Some(TextShadow {
                    offset_x: 1,
                    offset_y: 1,
                    blur: 4,
                    color: "rgba(0,0,0,0.3)".to_string(),
                }),
            },
        ];

        Ok(ThumbnailComposition {
            dimensions,
            background,
            assets: positioned_assets,
            text_overlays,
            branding: BrandingSpec {
                conference_logo_url: assets.conference_logo.clone(),
                pcg_logo_url: Some("https://pcg.media/logo.png".to_string()),
                position: "bottom-center".to_string(),
            },
        })
    }

    /// Render a composition to an image URL
    ///
    /// This returns either:
    /// - The AI-generated background URL (for simple compositions)
    /// - Or sends the spec to a rendering service for full composition
    pub async fn render_composition(&self, composition: &ThumbnailComposition) -> Result<String> {
        // For now, if we have a generated background, return that
        // Full composition rendering would require:
        // 1. ComfyUI workflow for image compositing (Maci)
        // 2. Or a server-side rendering service (HTML to image)

        match &composition.background {
            BackgroundSpec::Generated { url: Some(url), .. } => Ok(url.clone()),
            BackgroundSpec::Image { url } => Ok(url.clone()),
            _ => {
                // Generate a simple background as fallback via Maci/ComfyUI
                let prompt = "Modern tech conference abstract background, gradient colors, professional";
                match self.generate_image(prompt, composition.dimensions.width, composition.dimensions.height).await {
                    Some(url) => Ok(url),
                    None => Err(NoraError::ExecutionError(
                        "ComfyUI not available for image generation".to_string()
                    )),
                }
            }
        }
    }
}

/// Result containing both the composition spec and rendered URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposedThumbnail {
    pub composition: ThumbnailComposition,
    pub rendered_url: String,
    pub article_type: String,
}
