//! Advanced Audio Processing Module for Editron
//!
//! Professional audio capabilities including:
//! - Loudness normalization (EBU R128 / LUFS)
//! - Dynamic range compression
//! - Noise reduction
//! - Audio ducking for voice-overs
//! - Beat detection and sync
//! - Audio effects and filters

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use super::{EditronError, EditronResult};

/// Loudness standard for normalization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoudnessStandard {
    /// Broadcast standard: -24 LUFS
    EbuR128Broadcast,
    /// Streaming standard: -14 LUFS (Spotify, YouTube)
    Streaming,
    /// Podcast standard: -16 LUFS
    Podcast,
    /// Film/Cinema: -27 LUFS
    Cinema,
    /// Custom target
    Custom { lufs: f32, true_peak: f32 },
}

impl LoudnessStandard {
    pub fn target_lufs(&self) -> f32 {
        match self {
            LoudnessStandard::EbuR128Broadcast => -24.0,
            LoudnessStandard::Streaming => -14.0,
            LoudnessStandard::Podcast => -16.0,
            LoudnessStandard::Cinema => -27.0,
            LoudnessStandard::Custom { lufs, .. } => *lufs,
        }
    }

    pub fn true_peak_limit(&self) -> f32 {
        match self {
            LoudnessStandard::EbuR128Broadcast => -1.0,
            LoudnessStandard::Streaming => -1.0,
            LoudnessStandard::Podcast => -1.5,
            LoudnessStandard::Cinema => -1.0,
            LoudnessStandard::Custom { true_peak, .. } => *true_peak,
        }
    }
}

/// Audio loudness measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoudnessMeasurement {
    pub integrated_lufs: f32,
    pub true_peak_db: f32,
    pub loudness_range_lu: f32,
    pub short_term_max_lufs: f32,
    pub momentary_max_lufs: f32,
}

/// Compression settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionSettings {
    /// Threshold in dB
    pub threshold: f32,
    /// Compression ratio (e.g., 4:1 = 4.0)
    pub ratio: f32,
    /// Attack time in milliseconds
    pub attack_ms: f32,
    /// Release time in milliseconds
    pub release_ms: f32,
    /// Knee width in dB
    pub knee_db: f32,
    /// Makeup gain in dB
    pub makeup_gain: f32,
}

impl Default for CompressionSettings {
    fn default() -> Self {
        Self {
            threshold: -20.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            knee_db: 6.0,
            makeup_gain: 0.0,
        }
    }
}

impl CompressionSettings {
    /// Gentle compression for dialogue
    pub fn dialogue() -> Self {
        Self {
            threshold: -24.0,
            ratio: 2.5,
            attack_ms: 15.0,
            release_ms: 150.0,
            knee_db: 8.0,
            makeup_gain: 3.0,
        }
    }

    /// Voice-over compression
    pub fn voice_over() -> Self {
        Self {
            threshold: -18.0,
            ratio: 3.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            knee_db: 6.0,
            makeup_gain: 4.0,
        }
    }

    /// Music compression (subtle)
    pub fn music_subtle() -> Self {
        Self {
            threshold: -12.0,
            ratio: 2.0,
            attack_ms: 20.0,
            release_ms: 200.0,
            knee_db: 10.0,
            makeup_gain: 2.0,
        }
    }

    /// Heavy limiting
    pub fn limiter() -> Self {
        Self {
            threshold: -1.0,
            ratio: 20.0,
            attack_ms: 0.5,
            release_ms: 50.0,
            knee_db: 0.0,
            makeup_gain: 0.0,
        }
    }
}

/// EQ (Equalizer) settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EQSettings {
    pub bands: Vec<EQBand>,
    pub high_pass: Option<HighPassFilter>,
    pub low_pass: Option<LowPassFilter>,
}

impl Default for EQSettings {
    fn default() -> Self {
        Self {
            bands: vec![],
            high_pass: None,
            low_pass: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EQBand {
    /// Center frequency in Hz
    pub frequency: f32,
    /// Gain in dB (-12 to +12)
    pub gain: f32,
    /// Q factor (bandwidth)
    pub q: f32,
    /// Band type
    pub band_type: EQBandType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EQBandType {
    Peak,
    LowShelf,
    HighShelf,
    Notch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighPassFilter {
    pub frequency: f32,
    pub slope: FilterSlope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowPassFilter {
    pub frequency: f32,
    pub slope: FilterSlope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterSlope {
    Db6,
    Db12,
    Db18,
    Db24,
}

impl FilterSlope {
    pub fn poles(&self) -> u32 {
        match self {
            FilterSlope::Db6 => 1,
            FilterSlope::Db12 => 2,
            FilterSlope::Db18 => 3,
            FilterSlope::Db24 => 4,
        }
    }
}

/// Audio ducking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuckingSettings {
    /// Amount to duck by (in dB, negative)
    pub duck_amount: f32,
    /// Threshold for trigger
    pub threshold: f32,
    /// Attack time in milliseconds
    pub attack_ms: f32,
    /// Release time in milliseconds
    pub release_ms: f32,
    /// Hold time before release
    pub hold_ms: f32,
}

impl Default for DuckingSettings {
    fn default() -> Self {
        Self {
            duck_amount: -12.0,
            threshold: -30.0,
            attack_ms: 50.0,
            release_ms: 500.0,
            hold_ms: 100.0,
        }
    }
}

/// Noise reduction settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseReductionSettings {
    /// Noise floor in dB
    pub noise_floor: f32,
    /// Reduction amount (0.0 to 1.0)
    pub reduction_amount: f32,
    /// Use noise profile from sample
    pub noise_profile: Option<PathBuf>,
    /// Preserve transients
    pub preserve_transients: bool,
}

impl Default for NoiseReductionSettings {
    fn default() -> Self {
        Self {
            noise_floor: -50.0,
            reduction_amount: 0.5,
            noise_profile: None,
            preserve_transients: true,
        }
    }
}

/// De-esser settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeEsserSettings {
    /// Frequency range to target (typically 4-10kHz)
    pub frequency: f32,
    /// Threshold in dB
    pub threshold: f32,
    /// Reduction amount
    pub reduction: f32,
}

impl Default for DeEsserSettings {
    fn default() -> Self {
        Self {
            frequency: 6000.0,
            threshold: -30.0,
            reduction: 6.0,
        }
    }
}

/// Reverb settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverbSettings {
    /// Room size (0.0 to 1.0)
    pub room_size: f32,
    /// Damping (0.0 to 1.0)
    pub damping: f32,
    /// Wet/dry mix (0.0 to 1.0)
    pub wet_dry: f32,
    /// Pre-delay in milliseconds
    pub pre_delay_ms: f32,
    /// Stereo width (0.0 to 1.0)
    pub stereo_width: f32,
}

impl Default for ReverbSettings {
    fn default() -> Self {
        Self {
            room_size: 0.5,
            damping: 0.5,
            wet_dry: 0.3,
            pre_delay_ms: 20.0,
            stereo_width: 1.0,
        }
    }
}

/// Beat detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatDetectionResult {
    /// Detected tempo in BPM
    pub bpm: f32,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Beat positions in seconds
    pub beat_times: Vec<f32>,
    /// Downbeat positions
    pub downbeat_times: Vec<f32>,
}

/// Audio processing preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioProcessingPreset {
    pub name: String,
    pub description: String,
    pub loudness: Option<LoudnessStandard>,
    pub compression: Option<CompressionSettings>,
    pub eq: Option<EQSettings>,
    pub noise_reduction: Option<NoiseReductionSettings>,
    pub de_esser: Option<DeEsserSettings>,
    pub reverb: Option<ReverbSettings>,
}

/// Audio Processing Engine
pub struct AudioProcessingEngine {
    presets: Vec<AudioProcessingPreset>,
}

impl AudioProcessingEngine {
    pub fn new() -> Self {
        Self {
            presets: Self::default_presets(),
        }
    }

    fn default_presets() -> Vec<AudioProcessingPreset> {
        vec![
            // Broadcast Ready
            AudioProcessingPreset {
                name: "Broadcast Ready".to_string(),
                description: "EBU R128 compliant for broadcast".to_string(),
                loudness: Some(LoudnessStandard::EbuR128Broadcast),
                compression: Some(CompressionSettings::dialogue()),
                eq: Some(EQSettings {
                    bands: vec![
                        EQBand { frequency: 80.0, gain: -3.0, q: 1.0, band_type: EQBandType::LowShelf },
                        EQBand { frequency: 3000.0, gain: 2.0, q: 1.5, band_type: EQBandType::Peak },
                    ],
                    high_pass: Some(HighPassFilter { frequency: 80.0, slope: FilterSlope::Db12 }),
                    low_pass: None,
                }),
                noise_reduction: None,
                de_esser: None,
                reverb: None,
            },

            // YouTube/Streaming
            AudioProcessingPreset {
                name: "YouTube Optimized".to_string(),
                description: "-14 LUFS for streaming platforms".to_string(),
                loudness: Some(LoudnessStandard::Streaming),
                compression: Some(CompressionSettings {
                    threshold: -16.0,
                    ratio: 3.0,
                    attack_ms: 15.0,
                    release_ms: 100.0,
                    knee_db: 6.0,
                    makeup_gain: 2.0,
                }),
                eq: None,
                noise_reduction: None,
                de_esser: None,
                reverb: None,
            },

            // Podcast
            AudioProcessingPreset {
                name: "Podcast Standard".to_string(),
                description: "Clear voice with consistent levels".to_string(),
                loudness: Some(LoudnessStandard::Podcast),
                compression: Some(CompressionSettings::voice_over()),
                eq: Some(EQSettings {
                    bands: vec![
                        EQBand { frequency: 100.0, gain: -2.0, q: 1.0, band_type: EQBandType::LowShelf },
                        EQBand { frequency: 200.0, gain: -3.0, q: 2.0, band_type: EQBandType::Peak },
                        EQBand { frequency: 3500.0, gain: 3.0, q: 1.5, band_type: EQBandType::Peak },
                        EQBand { frequency: 8000.0, gain: 2.0, q: 1.0, band_type: EQBandType::HighShelf },
                    ],
                    high_pass: Some(HighPassFilter { frequency: 80.0, slope: FilterSlope::Db18 }),
                    low_pass: None,
                }),
                noise_reduction: Some(NoiseReductionSettings::default()),
                de_esser: Some(DeEsserSettings::default()),
                reverb: None,
            },

            // Voice Over Clean
            AudioProcessingPreset {
                name: "Voice Over Clean".to_string(),
                description: "Pristine voice-over for commercial use".to_string(),
                loudness: Some(LoudnessStandard::Custom { lufs: -20.0, true_peak: -1.5 }),
                compression: Some(CompressionSettings::voice_over()),
                eq: Some(EQSettings {
                    bands: vec![
                        EQBand { frequency: 250.0, gain: -2.0, q: 2.0, band_type: EQBandType::Peak },
                        EQBand { frequency: 4000.0, gain: 2.5, q: 1.5, band_type: EQBandType::Peak },
                        EQBand { frequency: 10000.0, gain: 1.5, q: 1.0, band_type: EQBandType::HighShelf },
                    ],
                    high_pass: Some(HighPassFilter { frequency: 100.0, slope: FilterSlope::Db24 }),
                    low_pass: Some(LowPassFilter { frequency: 16000.0, slope: FilterSlope::Db12 }),
                }),
                noise_reduction: Some(NoiseReductionSettings {
                    noise_floor: -55.0,
                    reduction_amount: 0.6,
                    noise_profile: None,
                    preserve_transients: true,
                }),
                de_esser: Some(DeEsserSettings {
                    frequency: 6500.0,
                    threshold: -35.0,
                    reduction: 8.0,
                }),
                reverb: None,
            },

            // Music Background
            AudioProcessingPreset {
                name: "Music Background".to_string(),
                description: "Background music processing".to_string(),
                loudness: None,
                compression: Some(CompressionSettings::music_subtle()),
                eq: Some(EQSettings {
                    bands: vec![
                        // Cut some mids to make room for voice
                        EQBand { frequency: 800.0, gain: -2.0, q: 1.0, band_type: EQBandType::Peak },
                        EQBand { frequency: 3000.0, gain: -3.0, q: 1.5, band_type: EQBandType::Peak },
                    ],
                    high_pass: Some(HighPassFilter { frequency: 40.0, slope: FilterSlope::Db12 }),
                    low_pass: None,
                }),
                noise_reduction: None,
                de_esser: None,
                reverb: None,
            },

            // Cinematic
            AudioProcessingPreset {
                name: "Cinematic Mix".to_string(),
                description: "Film-style audio processing".to_string(),
                loudness: Some(LoudnessStandard::Cinema),
                compression: Some(CompressionSettings {
                    threshold: -18.0,
                    ratio: 2.0,
                    attack_ms: 25.0,
                    release_ms: 250.0,
                    knee_db: 10.0,
                    makeup_gain: 1.0,
                }),
                eq: None,
                noise_reduction: None,
                de_esser: None,
                reverb: Some(ReverbSettings {
                    room_size: 0.4,
                    damping: 0.6,
                    wet_dry: 0.15,
                    pre_delay_ms: 30.0,
                    stereo_width: 0.8,
                }),
            },
        ]
    }

    /// Generate FFmpeg audio filter string
    pub fn to_ffmpeg_filter(&self, preset: &AudioProcessingPreset) -> String {
        let mut filters = Vec::new();

        // High-pass filter
        if let Some(eq) = &preset.eq {
            if let Some(hp) = &eq.high_pass {
                filters.push(format!("highpass=f={}:poles={}", hp.frequency, hp.slope.poles()));
            }
            if let Some(lp) = &eq.low_pass {
                filters.push(format!("lowpass=f={}:poles={}", lp.frequency, lp.slope.poles()));
            }

            // EQ bands
            for band in &eq.bands {
                match band.band_type {
                    EQBandType::Peak => {
                        filters.push(format!(
                            "equalizer=f={}:width_type=q:width={}:g={}",
                            band.frequency, band.q, band.gain
                        ));
                    }
                    EQBandType::LowShelf => {
                        filters.push(format!(
                            "lowshelf=f={}:g={}:width_type=q:width={}",
                            band.frequency, band.gain, band.q
                        ));
                    }
                    EQBandType::HighShelf => {
                        filters.push(format!(
                            "highshelf=f={}:g={}:width_type=q:width={}",
                            band.frequency, band.gain, band.q
                        ));
                    }
                    EQBandType::Notch => {
                        filters.push(format!(
                            "bandreject=f={}:width_type=q:width={}",
                            band.frequency, band.q
                        ));
                    }
                }
            }
        }

        // Compression
        if let Some(comp) = &preset.compression {
            filters.push(format!(
                "acompressor=threshold={}dB:ratio={}:attack={}:release={}:knee={}:makeup={}",
                comp.threshold,
                comp.ratio,
                comp.attack_ms,
                comp.release_ms,
                comp.knee_db,
                comp.makeup_gain
            ));
        }

        // Noise reduction (basic gate)
        if let Some(nr) = &preset.noise_reduction {
            filters.push(format!(
                "agate=threshold={}dB:ratio={}:attack=10:release=100",
                nr.noise_floor,
                1.0 / (1.0 - nr.reduction_amount)
            ));
        }

        // Loudness normalization
        if let Some(loudness) = &preset.loudness {
            filters.push(format!(
                "loudnorm=I={}:TP={}:LRA=11",
                loudness.target_lufs(),
                loudness.true_peak_limit()
            ));
        }

        if filters.is_empty() {
            "anull".to_string()
        } else {
            filters.join(",")
        }
    }

    /// Generate FFmpeg command for loudness measurement
    pub fn loudness_measure_command<P: AsRef<Path>>(input: P) -> String {
        format!(
            "ffmpeg -i \"{}\" -af loudnorm=I=-16:TP=-1.5:LRA=11:print_format=json -f null -",
            input.as_ref().display()
        )
    }

    /// Generate audio ducking filter
    pub fn ducking_filter(&self, settings: &DuckingSettings) -> String {
        format!(
            "sidechaincompress=threshold={}dB:ratio=4:attack={}:release={}:makeup={}",
            settings.threshold,
            settings.attack_ms,
            settings.release_ms,
            settings.duck_amount.abs()
        )
    }

    /// Get all presets
    pub fn presets(&self) -> &[AudioProcessingPreset] {
        &self.presets
    }

    /// Get preset by name
    pub fn get_preset(&self, name: &str) -> Option<&AudioProcessingPreset> {
        self.presets.iter().find(|p| p.name == name)
    }
}

impl Default for AudioProcessingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loudness_standards() {
        assert_eq!(LoudnessStandard::Streaming.target_lufs(), -14.0);
        assert_eq!(LoudnessStandard::Podcast.target_lufs(), -16.0);
    }

    #[test]
    fn test_audio_engine() {
        let engine = AudioProcessingEngine::new();
        assert!(!engine.presets().is_empty());
    }

    #[test]
    fn test_ffmpeg_filter() {
        let engine = AudioProcessingEngine::new();
        let preset = engine.get_preset("YouTube Optimized").unwrap();
        let filter = engine.to_ffmpeg_filter(preset);
        assert!(filter.contains("loudnorm"));
    }
}
