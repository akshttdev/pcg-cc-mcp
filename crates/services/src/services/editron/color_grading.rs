//! Advanced Color Grading Module for Editron
//!
//! Provides professional color grading capabilities including:
//! - LUT (Look-Up Table) application
//! - Color wheels (lift/gamma/gain)
//! - Curves adjustment
//! - HSL secondary corrections
//! - Color matching between clips
//! - Lumetri-style color presets

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use super::{EditronError, EditronResult};

/// LUT (Look-Up Table) for color grading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LUT {
    pub name: String,
    pub path: PathBuf,
    pub format: LUTFormat,
    pub intensity: f32, // 0.0 to 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LUTFormat {
    Cube,    // .cube format (most common)
    Cube3D,  // 3D LUT
    Look,    // Adobe Look format
    Mga,     // DaVinci Resolve format
}

/// Color wheel adjustments (Lift/Gamma/Gain)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColorWheels {
    pub lift: ColorWheel,   // Shadows
    pub gamma: ColorWheel,  // Midtones
    pub gain: ColorWheel,   // Highlights
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColorWheel {
    pub red: f32,    // -1.0 to 1.0
    pub green: f32,  // -1.0 to 1.0
    pub blue: f32,   // -1.0 to 1.0
    pub luminance: f32, // Master adjustment
}

/// RGB Curves for fine color control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorCurves {
    pub master: Vec<CurvePoint>,
    pub red: Vec<CurvePoint>,
    pub green: Vec<CurvePoint>,
    pub blue: Vec<CurvePoint>,
    pub hue_vs_hue: Vec<CurvePoint>,
    pub hue_vs_sat: Vec<CurvePoint>,
    pub hue_vs_lum: Vec<CurvePoint>,
}

impl Default for ColorCurves {
    fn default() -> Self {
        // Linear curve (no adjustment)
        let linear = vec![
            CurvePoint { x: 0.0, y: 0.0 },
            CurvePoint { x: 1.0, y: 1.0 },
        ];
        Self {
            master: linear.clone(),
            red: linear.clone(),
            green: linear.clone(),
            blue: linear.clone(),
            hue_vs_hue: linear.clone(),
            hue_vs_sat: linear.clone(),
            hue_vs_lum: linear,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvePoint {
    pub x: f32, // 0.0 to 1.0
    pub y: f32, // 0.0 to 1.0
}

/// HSL Secondary Color Correction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HSLSecondary {
    pub enabled: bool,
    /// Hue range to target (0-360 degrees)
    pub hue_range: (f32, f32),
    /// Saturation range (0-1)
    pub sat_range: (f32, f32),
    /// Luminance range (0-1)
    pub lum_range: (f32, f32),
    /// Adjustments to apply
    pub adjustments: HSLAdjustments,
    /// Feathering/softness of selection
    pub softness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HSLAdjustments {
    pub hue_shift: f32,      // -180 to 180
    pub saturation: f32,     // -1.0 to 1.0
    pub luminance: f32,      // -1.0 to 1.0
}

/// Complete color grade preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorGradePreset {
    pub name: String,
    pub description: String,
    /// Basic corrections (exposure, contrast, etc.)
    pub basic: BasicColorCorrection,
    /// Color wheels
    pub wheels: ColorWheels,
    /// Curves
    pub curves: ColorCurves,
    /// LUT to apply (optional)
    pub lut: Option<LUT>,
    /// HSL secondaries
    pub secondaries: Vec<HSLSecondary>,
    /// Vignette
    pub vignette: Option<Vignette>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BasicColorCorrection {
    pub exposure: f32,       // -4.0 to 4.0 stops
    pub contrast: f32,       // -100 to 100
    pub highlights: f32,     // -100 to 100
    pub shadows: f32,        // -100 to 100
    pub whites: f32,         // -100 to 100
    pub blacks: f32,         // -100 to 100
    pub temperature: f32,    // -100 to 100 (cool to warm)
    pub tint: f32,           // -100 to 100 (green to magenta)
    pub vibrance: f32,       // -100 to 100
    pub saturation: f32,     // -100 to 100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vignette {
    pub amount: f32,    // -100 to 100
    pub midpoint: f32,  // 0 to 100
    pub roundness: f32, // -100 to 100
    pub feather: f32,   // 0 to 100
}

/// Color Grading Engine
pub struct ColorGradingEngine {
    lut_directory: PathBuf,
    presets: Vec<ColorGradePreset>,
}

impl ColorGradingEngine {
    pub fn new<P: AsRef<Path>>(lut_directory: P) -> Self {
        Self {
            lut_directory: lut_directory.as_ref().to_path_buf(),
            presets: Self::default_presets(),
        }
    }

    /// Get default professional presets
    fn default_presets() -> Vec<ColorGradePreset> {
        vec![
            Self::preset_cinematic_teal_orange(),
            Self::preset_film_emulation(),
            Self::preset_clean_commercial(),
            Self::preset_moody_desaturated(),
            Self::preset_vibrant_lifestyle(),
            Self::preset_vintage_film(),
        ]
    }

    /// Cinematic teal/orange look
    pub fn preset_cinematic_teal_orange() -> ColorGradePreset {
        ColorGradePreset {
            name: "Cinematic Teal & Orange".to_string(),
            description: "Classic Hollywood blockbuster color grade".to_string(),
            basic: BasicColorCorrection {
                contrast: 15.0,
                highlights: -10.0,
                shadows: 5.0,
                temperature: 5.0,
                saturation: -5.0,
                ..Default::default()
            },
            wheels: ColorWheels {
                lift: ColorWheel { red: -0.05, green: 0.0, blue: 0.08, luminance: 0.0 },
                gamma: ColorWheel { red: 0.0, green: -0.02, blue: 0.0, luminance: 0.0 },
                gain: ColorWheel { red: 0.05, green: 0.02, blue: -0.05, luminance: 0.0 },
            },
            curves: ColorCurves::default(),
            lut: None,
            secondaries: vec![
                // Push skin tones to orange
                HSLSecondary {
                    enabled: true,
                    hue_range: (15.0, 45.0), // Orange/skin tone range
                    sat_range: (0.2, 1.0),
                    lum_range: (0.2, 0.9),
                    adjustments: HSLAdjustments {
                        hue_shift: 5.0,
                        saturation: 0.1,
                        luminance: 0.05,
                    },
                    softness: 0.3,
                },
            ],
            vignette: Some(Vignette {
                amount: -20.0,
                midpoint: 50.0,
                roundness: 0.0,
                feather: 50.0,
            }),
        }
    }

    /// Film emulation (Kodak-style)
    pub fn preset_film_emulation() -> ColorGradePreset {
        ColorGradePreset {
            name: "Film Emulation".to_string(),
            description: "Kodak 2383 print film style".to_string(),
            basic: BasicColorCorrection {
                contrast: 10.0,
                highlights: -15.0,
                shadows: 10.0,
                blacks: 5.0,
                saturation: -10.0,
                ..Default::default()
            },
            wheels: ColorWheels {
                lift: ColorWheel { red: 0.02, green: 0.01, blue: 0.05, luminance: 0.02 },
                gamma: ColorWheel { red: 0.0, green: 0.01, blue: 0.0, luminance: 0.0 },
                gain: ColorWheel { red: 0.02, green: 0.0, blue: -0.02, luminance: 0.0 },
            },
            curves: ColorCurves {
                // Slight S-curve for contrast
                master: vec![
                    CurvePoint { x: 0.0, y: 0.02 },
                    CurvePoint { x: 0.25, y: 0.22 },
                    CurvePoint { x: 0.5, y: 0.5 },
                    CurvePoint { x: 0.75, y: 0.78 },
                    CurvePoint { x: 1.0, y: 0.98 },
                ],
                ..Default::default()
            },
            lut: None,
            secondaries: vec![],
            vignette: Some(Vignette {
                amount: -15.0,
                midpoint: 60.0,
                roundness: -20.0,
                feather: 70.0,
            }),
        }
    }

    /// Clean commercial look
    pub fn preset_clean_commercial() -> ColorGradePreset {
        ColorGradePreset {
            name: "Clean Commercial".to_string(),
            description: "Bright, clean look for commercial/lifestyle content".to_string(),
            basic: BasicColorCorrection {
                exposure: 0.3,
                contrast: 5.0,
                highlights: -5.0,
                shadows: 15.0,
                whites: 5.0,
                vibrance: 10.0,
                saturation: 5.0,
                ..Default::default()
            },
            wheels: ColorWheels::default(),
            curves: ColorCurves::default(),
            lut: None,
            secondaries: vec![],
            vignette: None,
        }
    }

    /// Moody desaturated look
    pub fn preset_moody_desaturated() -> ColorGradePreset {
        ColorGradePreset {
            name: "Moody Desaturated".to_string(),
            description: "Dark, moody atmosphere with pulled saturation".to_string(),
            basic: BasicColorCorrection {
                exposure: -0.3,
                contrast: 20.0,
                highlights: -20.0,
                shadows: -5.0,
                blacks: -10.0,
                saturation: -30.0,
                ..Default::default()
            },
            wheels: ColorWheels {
                lift: ColorWheel { red: 0.0, green: 0.0, blue: 0.03, luminance: -0.02 },
                gamma: ColorWheel { red: 0.0, green: -0.01, blue: 0.0, luminance: 0.0 },
                gain: ColorWheel { red: 0.0, green: 0.0, blue: -0.02, luminance: 0.0 },
            },
            curves: ColorCurves::default(),
            lut: None,
            secondaries: vec![],
            vignette: Some(Vignette {
                amount: -30.0,
                midpoint: 40.0,
                roundness: 0.0,
                feather: 40.0,
            }),
        }
    }

    /// Vibrant lifestyle look
    pub fn preset_vibrant_lifestyle() -> ColorGradePreset {
        ColorGradePreset {
            name: "Vibrant Lifestyle".to_string(),
            description: "Punchy, vibrant colors for social media content".to_string(),
            basic: BasicColorCorrection {
                exposure: 0.2,
                contrast: 10.0,
                highlights: 5.0,
                shadows: 10.0,
                temperature: 3.0,
                vibrance: 25.0,
                saturation: 10.0,
                ..Default::default()
            },
            wheels: ColorWheels {
                lift: ColorWheel { red: 0.0, green: 0.0, blue: 0.02, luminance: 0.0 },
                gamma: ColorWheel::default(),
                gain: ColorWheel { red: 0.02, green: 0.01, blue: 0.0, luminance: 0.0 },
            },
            curves: ColorCurves::default(),
            lut: None,
            secondaries: vec![],
            vignette: None,
        }
    }

    /// Vintage film look
    pub fn preset_vintage_film() -> ColorGradePreset {
        ColorGradePreset {
            name: "Vintage Film".to_string(),
            description: "Faded, nostalgic film look".to_string(),
            basic: BasicColorCorrection {
                contrast: -5.0,
                highlights: -10.0,
                shadows: 15.0,
                blacks: 15.0, // Lifted blacks
                temperature: 10.0,
                saturation: -20.0,
                ..Default::default()
            },
            wheels: ColorWheels {
                lift: ColorWheel { red: 0.05, green: 0.03, blue: 0.0, luminance: 0.05 },
                gamma: ColorWheel { red: 0.02, green: 0.0, blue: -0.02, luminance: 0.0 },
                gain: ColorWheel { red: 0.0, green: -0.02, blue: -0.05, luminance: -0.03 },
            },
            curves: ColorCurves {
                master: vec![
                    CurvePoint { x: 0.0, y: 0.05 }, // Lifted blacks
                    CurvePoint { x: 0.5, y: 0.5 },
                    CurvePoint { x: 1.0, y: 0.95 }, // Crushed whites
                ],
                ..Default::default()
            },
            lut: None,
            secondaries: vec![],
            vignette: Some(Vignette {
                amount: -25.0,
                midpoint: 50.0,
                roundness: -30.0,
                feather: 80.0,
            }),
        }
    }

    /// Generate FFmpeg filter string for color grade
    pub fn to_ffmpeg_filter(&self, preset: &ColorGradePreset) -> String {
        let mut filters = Vec::new();

        // Basic corrections via eq filter
        let basic = &preset.basic;
        let brightness = basic.exposure * 0.1;
        let contrast = 1.0 + (basic.contrast / 100.0);
        let saturation = 1.0 + (basic.saturation / 100.0);
        let gamma = 1.0; // Could calculate from shadows/highlights

        filters.push(format!(
            "eq=brightness={}:contrast={}:saturation={}:gamma={}",
            brightness, contrast, saturation, gamma
        ));

        // Color temperature via colortemperature filter
        if basic.temperature != 0.0 {
            let temp = 6500.0 + (basic.temperature * 30.0); // Map to Kelvin
            filters.push(format!("colortemperature=temperature={}", temp));
        }

        // Color wheels via colorbalance
        let w = &preset.wheels;
        if w.lift.red != 0.0 || w.lift.green != 0.0 || w.lift.blue != 0.0 ||
           w.gamma.red != 0.0 || w.gamma.green != 0.0 || w.gamma.blue != 0.0 ||
           w.gain.red != 0.0 || w.gain.green != 0.0 || w.gain.blue != 0.0 {
            filters.push(format!(
                "colorbalance=rs={}:gs={}:bs={}:rm={}:gm={}:bm={}:rh={}:gh={}:bh={}",
                w.lift.red, w.lift.green, w.lift.blue,
                w.gamma.red, w.gamma.green, w.gamma.blue,
                w.gain.red, w.gain.green, w.gain.blue
            ));
        }

        // Vignette
        if let Some(v) = &preset.vignette {
            if v.amount != 0.0 {
                let angle = std::f32::consts::PI / 5.0; // Vignette angle
                filters.push(format!(
                    "vignette=angle={}:mode=backward",
                    angle
                ));
            }
        }

        filters.join(",")
    }

    /// Generate Premiere Pro Lumetri settings (ExtendScript)
    pub fn to_premiere_lumetri(&self, preset: &ColorGradePreset) -> String {
        let basic = &preset.basic;
        format!(r#"
// Lumetri Color Settings for: {}
var lumetriEffect = app.project.activeSequence.videoTracks[0].clips[0].components[1];
if (lumetriEffect) {{
    // Basic Correction
    lumetriEffect.properties.getParamForDisplayName("Exposure").setValue({});
    lumetriEffect.properties.getParamForDisplayName("Contrast").setValue({});
    lumetriEffect.properties.getParamForDisplayName("Highlights").setValue({});
    lumetriEffect.properties.getParamForDisplayName("Shadows").setValue({});
    lumetriEffect.properties.getParamForDisplayName("Whites").setValue({});
    lumetriEffect.properties.getParamForDisplayName("Blacks").setValue({});
    lumetriEffect.properties.getParamForDisplayName("Temperature").setValue({});
    lumetriEffect.properties.getParamForDisplayName("Tint").setValue({});
    lumetriEffect.properties.getParamForDisplayName("Vibrance").setValue({});
    lumetriEffect.properties.getParamForDisplayName("Saturation").setValue({});
}}
"#,
            preset.name,
            basic.exposure,
            basic.contrast,
            basic.highlights,
            basic.shadows,
            basic.whites,
            basic.blacks,
            basic.temperature,
            basic.tint,
            basic.vibrance,
            basic.saturation
        )
    }

    /// List available LUTs
    pub async fn list_luts(&self) -> EditronResult<Vec<LUT>> {
        let mut luts = Vec::new();

        if self.lut_directory.exists() {
            let mut entries = tokio::fs::read_dir(&self.lut_directory).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    let format = match ext.to_str() {
                        Some("cube") => Some(LUTFormat::Cube),
                        Some("3dl") => Some(LUTFormat::Cube3D),
                        Some("look") => Some(LUTFormat::Look),
                        Some("mga") => Some(LUTFormat::Mga),
                        _ => None,
                    };

                    if let Some(format) = format {
                        luts.push(LUT {
                            name: path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Unknown")
                                .to_string(),
                            path: path.clone(),
                            format,
                            intensity: 1.0,
                        });
                    }
                }
            }
        }

        Ok(luts)
    }

    /// Get all presets
    pub fn presets(&self) -> &[ColorGradePreset] {
        &self.presets
    }

    /// Get preset by name
    pub fn get_preset(&self, name: &str) -> Option<&ColorGradePreset> {
        self.presets.iter().find(|p| p.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_presets() {
        let engine = ColorGradingEngine::new("/tmp/luts");
        assert!(!engine.presets().is_empty());
    }

    #[test]
    fn test_ffmpeg_filter_generation() {
        let engine = ColorGradingEngine::new("/tmp/luts");
        let preset = ColorGradingEngine::preset_cinematic_teal_orange();
        let filter = engine.to_ffmpeg_filter(&preset);
        assert!(filter.contains("eq="));
    }
}
