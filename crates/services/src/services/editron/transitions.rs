//! Advanced Transitions Module for Editron
//!
//! Professional video transitions including:
//! - Standard cuts and dissolves
//! - Modern social media transitions (whip pan, zoom, glitch)
//! - Film-style transitions (wipes, iris, film burn)
//! - Custom transition presets
//! - Audio-synced transitions

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Transition types available
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transition {
    /// Simple cut (no transition effect)
    Cut,

    /// Cross dissolve / fade
    Dissolve {
        duration_frames: u32,
        curve: EasingCurve,
    },

    /// Fade to/from color
    FadeToColor {
        duration_frames: u32,
        color: String, // Hex color
        curve: EasingCurve,
    },

    /// Directional wipe
    Wipe {
        duration_frames: u32,
        direction: WipeDirection,
        softness: f32, // 0.0 to 1.0
    },

    /// Whip pan (fast blur motion)
    WhipPan {
        duration_frames: u32,
        direction: WhipDirection,
        motion_blur: f32,
        speed_ramp: bool,
    },

    /// Zoom/push transition
    ZoomPush {
        duration_frames: u32,
        direction: ZoomDirection,
        scale_amount: f32,
        motion_blur: bool,
    },

    /// Glitch/digital transition
    Glitch {
        duration_frames: u32,
        intensity: f32,
        rgb_split: bool,
        block_corruption: bool,
        scanlines: bool,
    },

    /// Light leak / film burn
    LightLeak {
        duration_frames: u32,
        color: LightLeakColor,
        intensity: f32,
        position: LightLeakPosition,
    },

    /// Lens distortion transition
    LensDistort {
        duration_frames: u32,
        distortion_amount: f32,
        chromatic_aberration: bool,
    },

    /// Spin/rotate transition
    Spin {
        duration_frames: u32,
        rotations: f32,
        direction: SpinDirection,
        zoom: bool,
    },

    /// Iris (circular wipe)
    Iris {
        duration_frames: u32,
        opening: bool, // true = iris in, false = iris out
        center_x: f32,
        center_y: f32,
    },

    /// Film frame/sprocket transition
    FilmFrame {
        duration_frames: u32,
        film_type: FilmType,
        grain_amount: f32,
    },

    /// Luma/luminance wipe
    LumaWipe {
        duration_frames: u32,
        matte_path: Option<PathBuf>,
        invert: bool,
        softness: f32,
    },

    /// Shape wipe with custom mask
    ShapeWipe {
        duration_frames: u32,
        shape: WipeShape,
        softness: f32,
    },

    /// Morph transition (experimental)
    Morph {
        duration_frames: u32,
        blend_mode: BlendMode,
    },

    /// Audio-synced beat transition
    BeatSync {
        base_transition: Box<Transition>,
        beat_sensitivity: f32,
    },
}

/// Easing curve for animations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EasingCurve {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseInBack,
    EaseOutBack,
    EaseInOutBack,
    EaseInBounce,
    EaseOutBounce,
}

impl Default for EasingCurve {
    fn default() -> Self {
        EasingCurve::EaseInOut
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WipeDirection {
    Left,
    Right,
    Up,
    Down,
    DiagonalTopLeft,
    DiagonalTopRight,
    DiagonalBottomLeft,
    DiagonalBottomRight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhipDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoomDirection {
    In,
    Out,
    PushForward,
    PullBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LightLeakColor {
    Warm,     // Orange/yellow
    Cool,     // Blue/cyan
    Film,     // Classic amber
    Neon,     // Pink/purple
    Custom(String), // Hex color
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LightLeakPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
    Random,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpinDirection {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilmType {
    Film8mm,
    Film16mm,
    Film35mm,
    VHS,
    Digital,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WipeShape {
    Circle,
    Rectangle,
    Diamond,
    Heart,
    Star,
    Custom(PathBuf), // Custom SVG/image mask
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    SoftLight,
    HardLight,
    Difference,
    Add,
}

/// Transition preset collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionPreset {
    pub name: String,
    pub category: TransitionCategory,
    pub transition: Transition,
    pub description: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionCategory {
    Basic,
    Social,
    Cinematic,
    Retro,
    Creative,
    Audio,
}

/// Transition Engine
pub struct TransitionEngine {
    presets: Vec<TransitionPreset>,
    frame_rate: f32,
}

impl TransitionEngine {
    pub fn new(frame_rate: f32) -> Self {
        Self {
            presets: Self::default_presets(),
            frame_rate,
        }
    }

    /// Generate default professional presets
    fn default_presets() -> Vec<TransitionPreset> {
        vec![
            // Basic transitions
            TransitionPreset {
                name: "Cross Dissolve".to_string(),
                category: TransitionCategory::Basic,
                transition: Transition::Dissolve {
                    duration_frames: 15,
                    curve: EasingCurve::EaseInOut,
                },
                description: "Classic smooth fade between clips".to_string(),
                tags: vec!["fade".to_string(), "smooth".to_string()],
            },
            TransitionPreset {
                name: "Dip to Black".to_string(),
                category: TransitionCategory::Basic,
                transition: Transition::FadeToColor {
                    duration_frames: 20,
                    color: "#000000".to_string(),
                    curve: EasingCurve::EaseInOut,
                },
                description: "Fade through black".to_string(),
                tags: vec!["fade".to_string(), "black".to_string()],
            },
            TransitionPreset {
                name: "Dip to White".to_string(),
                category: TransitionCategory::Basic,
                transition: Transition::FadeToColor {
                    duration_frames: 20,
                    color: "#FFFFFF".to_string(),
                    curve: EasingCurve::EaseInOut,
                },
                description: "Fade through white (flash)".to_string(),
                tags: vec!["fade".to_string(), "white".to_string(), "flash".to_string()],
            },

            // Social media transitions
            TransitionPreset {
                name: "Whip Pan Right".to_string(),
                category: TransitionCategory::Social,
                transition: Transition::WhipPan {
                    duration_frames: 8,
                    direction: WhipDirection::Right,
                    motion_blur: 1.0,
                    speed_ramp: true,
                },
                description: "Fast pan to the right with motion blur".to_string(),
                tags: vec!["fast".to_string(), "dynamic".to_string(), "social".to_string()],
            },
            TransitionPreset {
                name: "Whip Pan Left".to_string(),
                category: TransitionCategory::Social,
                transition: Transition::WhipPan {
                    duration_frames: 8,
                    direction: WhipDirection::Left,
                    motion_blur: 1.0,
                    speed_ramp: true,
                },
                description: "Fast pan to the left with motion blur".to_string(),
                tags: vec!["fast".to_string(), "dynamic".to_string(), "social".to_string()],
            },
            TransitionPreset {
                name: "Zoom Push".to_string(),
                category: TransitionCategory::Social,
                transition: Transition::ZoomPush {
                    duration_frames: 10,
                    direction: ZoomDirection::In,
                    scale_amount: 1.3,
                    motion_blur: true,
                },
                description: "Zoom into the next clip".to_string(),
                tags: vec!["zoom".to_string(), "dynamic".to_string(), "impact".to_string()],
            },
            TransitionPreset {
                name: "Glitch".to_string(),
                category: TransitionCategory::Social,
                transition: Transition::Glitch {
                    duration_frames: 12,
                    intensity: 0.7,
                    rgb_split: true,
                    block_corruption: true,
                    scanlines: false,
                },
                description: "Digital glitch effect".to_string(),
                tags: vec!["glitch".to_string(), "digital".to_string(), "trendy".to_string()],
            },
            TransitionPreset {
                name: "Smooth Zoom Out".to_string(),
                category: TransitionCategory::Social,
                transition: Transition::ZoomPush {
                    duration_frames: 15,
                    direction: ZoomDirection::Out,
                    scale_amount: 0.8,
                    motion_blur: false,
                },
                description: "Gentle zoom out transition".to_string(),
                tags: vec!["zoom".to_string(), "smooth".to_string(), "reveal".to_string()],
            },

            // Cinematic transitions
            TransitionPreset {
                name: "Light Leak Warm".to_string(),
                category: TransitionCategory::Cinematic,
                transition: Transition::LightLeak {
                    duration_frames: 20,
                    color: LightLeakColor::Warm,
                    intensity: 0.8,
                    position: LightLeakPosition::TopRight,
                },
                description: "Warm film light leak".to_string(),
                tags: vec!["film".to_string(), "organic".to_string(), "warm".to_string()],
            },
            TransitionPreset {
                name: "Lens Flare".to_string(),
                category: TransitionCategory::Cinematic,
                transition: Transition::LightLeak {
                    duration_frames: 18,
                    color: LightLeakColor::Cool,
                    intensity: 0.6,
                    position: LightLeakPosition::Center,
                },
                description: "Anamorphic lens flare style".to_string(),
                tags: vec!["lens".to_string(), "flare".to_string(), "cinematic".to_string()],
            },
            TransitionPreset {
                name: "Iris In".to_string(),
                category: TransitionCategory::Cinematic,
                transition: Transition::Iris {
                    duration_frames: 20,
                    opening: true,
                    center_x: 0.5,
                    center_y: 0.5,
                },
                description: "Classic circular iris open".to_string(),
                tags: vec!["classic".to_string(), "vintage".to_string()],
            },

            // Retro transitions
            TransitionPreset {
                name: "Film Burn 8mm".to_string(),
                category: TransitionCategory::Retro,
                transition: Transition::FilmFrame {
                    duration_frames: 24,
                    film_type: FilmType::Film8mm,
                    grain_amount: 0.5,
                },
                description: "8mm film burn and frame flash".to_string(),
                tags: vec!["vintage".to_string(), "film".to_string(), "retro".to_string()],
            },
            TransitionPreset {
                name: "VHS Glitch".to_string(),
                category: TransitionCategory::Retro,
                transition: Transition::Glitch {
                    duration_frames: 15,
                    intensity: 0.5,
                    rgb_split: true,
                    block_corruption: false,
                    scanlines: true,
                },
                description: "VHS tape glitch effect".to_string(),
                tags: vec!["vhs".to_string(), "retro".to_string(), "nostalgic".to_string()],
            },

            // Creative transitions
            TransitionPreset {
                name: "Spin Clockwise".to_string(),
                category: TransitionCategory::Creative,
                transition: Transition::Spin {
                    duration_frames: 15,
                    rotations: 1.0,
                    direction: SpinDirection::Clockwise,
                    zoom: true,
                },
                description: "Spinning rotation with zoom".to_string(),
                tags: vec!["spin".to_string(), "dynamic".to_string(), "creative".to_string()],
            },
            TransitionPreset {
                name: "Lens Distort".to_string(),
                category: TransitionCategory::Creative,
                transition: Transition::LensDistort {
                    duration_frames: 12,
                    distortion_amount: 0.8,
                    chromatic_aberration: true,
                },
                description: "Lens distortion with chromatic aberration".to_string(),
                tags: vec!["lens".to_string(), "creative".to_string(), "trippy".to_string()],
            },
        ]
    }

    /// Convert transition to FFmpeg filter string
    pub fn to_ffmpeg_filter(&self, transition: &Transition, input_a: &str, input_b: &str) -> String {
        match transition {
            Transition::Dissolve { duration_frames, .. } => {
                let duration = *duration_frames as f32 / self.frame_rate;
                format!(
                    "[{}][{}]xfade=transition=fade:duration={}:offset=0",
                    input_a, input_b, duration
                )
            }
            Transition::FadeToColor { duration_frames, color, .. } => {
                let duration = *duration_frames as f32 / self.frame_rate;
                format!(
                    "[{}][{}]xfade=transition=fadeblack:duration={}:offset=0",
                    input_a, input_b, duration
                )
            }
            Transition::Wipe { duration_frames, direction, softness } => {
                let duration = *duration_frames as f32 / self.frame_rate;
                let xfade_type = match direction {
                    WipeDirection::Left => "wipeleft",
                    WipeDirection::Right => "wiperight",
                    WipeDirection::Up => "wipeup",
                    WipeDirection::Down => "wipedown",
                    WipeDirection::DiagonalTopLeft => "slideleft",
                    WipeDirection::DiagonalTopRight => "slideright",
                    WipeDirection::DiagonalBottomLeft => "slidedown",
                    WipeDirection::DiagonalBottomRight => "slideup",
                };
                format!(
                    "[{}][{}]xfade=transition={}:duration={}:offset=0",
                    input_a, input_b, xfade_type, duration
                )
            }
            Transition::ZoomPush { duration_frames, direction, .. } => {
                let duration = *duration_frames as f32 / self.frame_rate;
                let xfade_type = match direction {
                    ZoomDirection::In => "zoomin",
                    ZoomDirection::Out => "fadefast",
                    ZoomDirection::PushForward => "zoomin",
                    ZoomDirection::PullBack => "fadefast",
                };
                format!(
                    "[{}][{}]xfade=transition={}:duration={}:offset=0",
                    input_a, input_b, xfade_type, duration
                )
            }
            Transition::Iris { duration_frames, opening, .. } => {
                let duration = *duration_frames as f32 / self.frame_rate;
                let xfade_type = if *opening { "circleopen" } else { "circleclose" };
                format!(
                    "[{}][{}]xfade=transition={}:duration={}:offset=0",
                    input_a, input_b, xfade_type, duration
                )
            }
            Transition::Glitch { duration_frames, intensity, rgb_split, .. } => {
                let duration = *duration_frames as f32 / self.frame_rate;
                // Glitch requires custom filter chain
                let mut filters = vec![];
                if *rgb_split {
                    filters.push(format!("rgbashift=rh=-{}:bh={}", intensity * 10.0, intensity * 10.0));
                }
                format!(
                    "[{}][{}]xfade=transition=pixelize:duration={}:offset=0",
                    input_a, input_b, duration
                )
            }
            _ => {
                // Default to dissolve for unsupported transitions
                format!("[{}][{}]xfade=transition=fade:duration=0.5:offset=0", input_a, input_b)
            }
        }
    }

    /// Generate Premiere Pro transition ExtendScript
    pub fn to_premiere_script(&self, transition: &Transition, track_index: u32, clip_index: u32) -> String {
        let transition_name = match transition {
            Transition::Dissolve { .. } => "Cross Dissolve",
            Transition::FadeToColor { color, .. } => {
                if color == "#000000" { "Dip to Black" } else { "Dip to White" }
            }
            Transition::Wipe { direction, .. } => match direction {
                WipeDirection::Left => "Wipe Left",
                WipeDirection::Right => "Wipe Right",
                WipeDirection::Up => "Push Up",
                WipeDirection::Down => "Push Down",
                _ => "Wipe",
            },
            _ => "Cross Dissolve",
        };

        let duration_frames = match transition {
            Transition::Dissolve { duration_frames, .. } => *duration_frames,
            Transition::FadeToColor { duration_frames, .. } => *duration_frames,
            Transition::Wipe { duration_frames, .. } => *duration_frames,
            _ => 15,
        };

        format!(r#"
// Apply transition: {}
var seq = app.project.activeSequence;
var track = seq.videoTracks[{}];
var clip = track.clips[{}];

// Get transition from project
var transition = app.project.rootItem.findItemsMatchingMediaPath("{}", 1)[0];
if (clip && clip.end) {{
    // Apply transition at clip end
    clip.setTransition("Video Transitions\\Dissolve\\{}", clip.end.ticks - {});
}}
"#,
            transition_name,
            track_index,
            clip_index,
            transition_name,
            transition_name,
            duration_frames
        )
    }

    /// Get all presets
    pub fn presets(&self) -> &[TransitionPreset] {
        &self.presets
    }

    /// Get presets by category
    pub fn presets_by_category(&self, category: TransitionCategory) -> Vec<&TransitionPreset> {
        self.presets
            .iter()
            .filter(|p| matches!((&p.category, &category),
                (TransitionCategory::Basic, TransitionCategory::Basic) |
                (TransitionCategory::Social, TransitionCategory::Social) |
                (TransitionCategory::Cinematic, TransitionCategory::Cinematic) |
                (TransitionCategory::Retro, TransitionCategory::Retro) |
                (TransitionCategory::Creative, TransitionCategory::Creative) |
                (TransitionCategory::Audio, TransitionCategory::Audio)
            ))
            .collect()
    }

    /// Search presets by tag
    pub fn search_presets(&self, query: &str) -> Vec<&TransitionPreset> {
        let query_lower = query.to_lowercase();
        self.presets
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower) ||
                p.description.to_lowercase().contains(&query_lower) ||
                p.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_engine() {
        let engine = TransitionEngine::new(24.0);
        assert!(!engine.presets().is_empty());
    }

    #[test]
    fn test_ffmpeg_filter() {
        let engine = TransitionEngine::new(24.0);
        let transition = Transition::Dissolve {
            duration_frames: 12,
            curve: EasingCurve::EaseInOut,
        };
        let filter = engine.to_ffmpeg_filter(&transition, "0:v", "1:v");
        assert!(filter.contains("xfade"));
    }
}
