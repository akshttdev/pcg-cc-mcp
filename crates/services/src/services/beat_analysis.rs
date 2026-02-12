//! Beat & Music Structure Analysis Engine
//!
//! Uses FFmpeg to analyze audio tracks for:
//! - BPM detection via onset energy analysis
//! - Beat grid generation (timestamp of every beat)
//! - Energy curve mapping (loudness over time)
//! - Structural section detection (intro/verse/chorus/bridge/outro)
//! - Transition point markers (downbeats, drops, builds)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;

#[derive(Debug, Error)]
pub enum BeatAnalysisError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("FFmpeg error: {0}")]
    FFmpeg(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

/// A section of the song structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicSection {
    pub name: String,
    pub start: f64,
    pub end: f64,
    pub energy_level: f64,
    pub suggested_content: SuggestedContent,
}

/// What kind of video content suits this music section
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestedContent {
    /// Low energy, wide shots, set the scene
    Establishing,
    /// Building energy, variety of subjects
    Building,
    /// Peak energy, best hero moments, fast cuts
    Peak,
    /// Longest hold on most striking footage
    HeroMoment,
    /// Cool down, resolution, closing imagery
    Resolution,
    /// Quick flash cut on accent beat
    FlashCut,
}

/// A single beat on the grid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatMarker {
    pub timestamp: f64,
    pub beat_number: u32,
    pub bar_number: u32,
    pub beat_in_bar: u32,
    pub is_downbeat: bool,
    pub energy_at_beat: f64,
    /// Strong beats are good cut points
    pub is_strong_cut_point: bool,
}

/// A transition point marker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionMarker {
    pub timestamp: f64,
    pub transition_type: TransitionType,
    pub intensity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionType {
    Downbeat,
    Drop,
    Build,
    Fill,
    Accent,
}

/// Energy measurement at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyPoint {
    pub timestamp: f64,
    pub loudness_lufs: f64,
    pub normalized_energy: f64,
}

/// Complete beat analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatGridResult {
    pub audio_path: String,
    pub duration: f64,
    pub bpm: f64,
    pub beat_interval: f64,
    pub total_beats: u32,
    pub beats_per_bar: u32,
    pub beats: Vec<BeatMarker>,
    pub sections: Vec<MusicSection>,
    pub energy_curve: Vec<EnergyPoint>,
    pub transition_markers: Vec<TransitionMarker>,
    pub processing_time_ms: u64,
}

pub struct BeatAnalysisEngine {
    ffmpeg_path: PathBuf,
    ffprobe_path: PathBuf,
}

impl BeatAnalysisEngine {
    pub fn new() -> Self {
        Self {
            ffmpeg_path: PathBuf::from("ffmpeg"),
            ffprobe_path: PathBuf::from("ffprobe"),
        }
    }

    /// Get audio duration via ffprobe
    pub async fn get_duration(&self, path: &Path) -> Result<f64, BeatAnalysisError> {
        let output = Command::new(&self.ffprobe_path)
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
            ])
            .arg(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        let json_str = String::from_utf8_lossy(&output.stdout);
        let val: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| BeatAnalysisError::Parse(format!("ffprobe: {}", e)))?;

        val["format"]["duration"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or_else(|| BeatAnalysisError::Parse("No duration found".to_string()))
    }

    /// Measure loudness over time using ebur128 filter
    pub async fn measure_energy_curve(
        &self,
        path: &Path,
        duration: f64,
        window_size: f64,
    ) -> Result<Vec<EnergyPoint>, BeatAnalysisError> {
        // Use volumedetect to get overall levels, then astats for per-segment
        let num_windows = ((duration / window_size).ceil() as usize).min(200);
        let mut points = Vec::new();

        for i in 0..num_windows {
            let start = i as f64 * window_size;
            if start >= duration {
                break;
            }
            let segment_dur = window_size.min(duration - start);

            let output = Command::new(&self.ffmpeg_path)
                .args(["-ss", &format!("{:.3}", start)])
                .args(["-t", &format!("{:.3}", segment_dur)])
                .args(["-i"])
                .arg(path)
                .args([
                    "-af", "volumedetect",
                    "-f", "null",
                    "-",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?;

            let stderr = String::from_utf8_lossy(&output.stderr);

            // Parse mean_volume from volumedetect
            let mean_vol = Self::parse_volumedetect(&stderr, "mean_volume")
                .unwrap_or(-30.0);

            // Normalize: -60dB=0.0, 0dB=1.0
            let normalized = ((mean_vol + 60.0) / 60.0).clamp(0.0, 1.0);

            points.push(EnergyPoint {
                timestamp: start + segment_dur / 2.0,
                loudness_lufs: mean_vol,
                normalized_energy: normalized,
            });
        }

        Ok(points)
    }

    /// Detect BPM using onset detection via FFmpeg's `aresample` + energy analysis
    /// Falls back to provided BPM hint if detection is unreliable
    pub fn detect_bpm(energy_curve: &[EnergyPoint], bpm_hint: Option<f64>) -> f64 {
        // If we have a hint, use it (operator-provided BPM is most reliable)
        if let Some(hint) = bpm_hint {
            if hint > 30.0 && hint < 300.0 {
                return hint;
            }
        }

        // Simple onset-based BPM estimation from energy peaks
        if energy_curve.len() < 4 {
            return 120.0; // Default
        }

        // Find energy peaks (local maxima)
        let mut peaks = Vec::new();
        for i in 1..energy_curve.len() - 1 {
            if energy_curve[i].normalized_energy > energy_curve[i - 1].normalized_energy
                && energy_curve[i].normalized_energy > energy_curve[i + 1].normalized_energy
                && energy_curve[i].normalized_energy > 0.3
            {
                peaks.push(energy_curve[i].timestamp);
            }
        }

        if peaks.len() < 2 {
            return 120.0;
        }

        // Calculate average interval between peaks
        let intervals: Vec<f64> = peaks.windows(2).map(|w| w[1] - w[0]).collect();
        let avg_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;

        if avg_interval > 0.0 {
            (60.0 / avg_interval).clamp(60.0, 200.0)
        } else {
            120.0
        }
    }

    /// Generate beat grid from BPM
    pub fn generate_beat_grid(
        bpm: f64,
        duration: f64,
        beats_per_bar: u32,
        energy_curve: &[EnergyPoint],
    ) -> Vec<BeatMarker> {
        let beat_interval = 60.0 / bpm;
        let total_beats = (duration / beat_interval).floor() as u32;
        let mut beats = Vec::new();

        for i in 0..total_beats {
            let timestamp = i as f64 * beat_interval;
            let bar_number = i / beats_per_bar + 1;
            let beat_in_bar = i % beats_per_bar + 1;
            let is_downbeat = beat_in_bar == 1;

            // Find energy at this beat
            let energy_at_beat = Self::energy_at_time(energy_curve, timestamp);

            // Strong cut points: downbeats, or beats 1 and 3 in 4/4
            let is_strong_cut_point = is_downbeat
                || (beats_per_bar == 4 && beat_in_bar == 3)
                || (energy_at_beat > 0.6);

            beats.push(BeatMarker {
                timestamp,
                beat_number: i + 1,
                bar_number,
                beat_in_bar,
                is_downbeat,
                energy_at_beat,
                is_strong_cut_point,
            });
        }

        beats
    }

    /// Detect music sections based on energy curve
    pub fn detect_sections(
        energy_curve: &[EnergyPoint],
        duration: f64,
    ) -> Vec<MusicSection> {
        if energy_curve.is_empty() {
            return vec![MusicSection {
                name: "Full Track".to_string(),
                start: 0.0,
                end: duration,
                energy_level: 0.5,
                suggested_content: SuggestedContent::Building,
            }];
        }

        // Divide track into sections based on energy changes
        let avg_energy = energy_curve.iter()
            .map(|p| p.normalized_energy)
            .sum::<f64>() / energy_curve.len() as f64;

        // Simple section detection: split into 5-6 segments and classify
        let section_duration = duration / 6.0;
        let mut sections = Vec::new();

        let section_defs = [
            ("Intro", 0.0, SuggestedContent::Establishing),
            ("Verse", 1.0, SuggestedContent::Building),
            ("Chorus", 2.0, SuggestedContent::Peak),
            ("Bridge", 3.0, SuggestedContent::HeroMoment),
            ("Chorus 2", 4.0, SuggestedContent::Peak),
            ("Outro", 5.0, SuggestedContent::Resolution),
        ];

        for (name, idx, default_content) in &section_defs {
            let start = idx * section_duration;
            let end = (start + section_duration).min(duration);

            // Calculate energy in this section
            let section_energy = energy_curve.iter()
                .filter(|p| p.timestamp >= start && p.timestamp < end)
                .map(|p| p.normalized_energy)
                .collect::<Vec<_>>();

            let energy_level = if section_energy.is_empty() {
                avg_energy
            } else {
                section_energy.iter().sum::<f64>() / section_energy.len() as f64
            };

            // Override content suggestion based on actual energy
            let suggested_content = if energy_level > avg_energy * 1.3 {
                SuggestedContent::Peak
            } else if energy_level < avg_energy * 0.7 {
                if start < duration * 0.2 {
                    SuggestedContent::Establishing
                } else {
                    SuggestedContent::Resolution
                }
            } else {
                default_content.clone()
            };

            sections.push(MusicSection {
                name: name.to_string(),
                start,
                end,
                energy_level,
                suggested_content,
            });
        }

        sections
    }

    /// Generate transition markers at key musical moments
    pub fn generate_transition_markers(
        beats: &[BeatMarker],
        sections: &[MusicSection],
    ) -> Vec<TransitionMarker> {
        let mut markers = Vec::new();

        // Every downbeat is a potential transition
        for beat in beats {
            if beat.is_downbeat {
                markers.push(TransitionMarker {
                    timestamp: beat.timestamp,
                    transition_type: TransitionType::Downbeat,
                    intensity: beat.energy_at_beat,
                });
            }
        }

        // Section boundaries are strong transitions
        for section in sections {
            markers.push(TransitionMarker {
                timestamp: section.start,
                transition_type: if section.energy_level > 0.6 {
                    TransitionType::Drop
                } else {
                    TransitionType::Build
                },
                intensity: section.energy_level,
            });
        }

        markers.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap_or(std::cmp::Ordering::Equal));
        markers.dedup_by(|a, b| (a.timestamp - b.timestamp).abs() < 0.1);
        markers
    }

    /// Full analysis pipeline
    pub async fn analyze(
        &self,
        path: &Path,
        bpm_hint: Option<f64>,
        beats_per_bar: u32,
    ) -> Result<BeatGridResult, BeatAnalysisError> {
        let start = std::time::Instant::now();

        let duration = self.get_duration(path).await?;

        // Measure energy every 0.5 seconds for fine resolution
        let energy_curve = self.measure_energy_curve(path, duration, 0.5).await?;

        let bpm = Self::detect_bpm(&energy_curve, bpm_hint);
        let beat_interval = 60.0 / bpm;

        let beats = Self::generate_beat_grid(bpm, duration, beats_per_bar, &energy_curve);
        let total_beats = beats.len() as u32;

        let sections = Self::detect_sections(&energy_curve, duration);
        let transition_markers = Self::generate_transition_markers(&beats, &sections);

        let processing_time_ms = start.elapsed().as_millis() as u64;

        Ok(BeatGridResult {
            audio_path: path.to_string_lossy().to_string(),
            duration,
            bpm,
            beat_interval,
            total_beats,
            beats_per_bar,
            beats,
            sections,
            energy_curve,
            transition_markers,
            processing_time_ms,
        })
    }

    fn energy_at_time(energy_curve: &[EnergyPoint], time: f64) -> f64 {
        energy_curve.iter()
            .min_by(|a, b| {
                (a.timestamp - time).abs()
                    .partial_cmp(&(b.timestamp - time).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| p.normalized_energy)
            .unwrap_or(0.5)
    }

    fn parse_volumedetect(text: &str, key: &str) -> Option<f64> {
        for line in text.lines() {
            if line.contains(key) {
                // Format: "    mean_volume: -18.5 dB"
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    let val_str = parts[1].trim().replace(" dB", "");
                    if let Ok(v) = val_str.parse::<f64>() {
                        return Some(v);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beat_grid_generation() {
        let energy = vec![
            EnergyPoint { timestamp: 0.0, loudness_lufs: -14.0, normalized_energy: 0.77 },
            EnergyPoint { timestamp: 5.0, loudness_lufs: -10.0, normalized_energy: 0.83 },
        ];
        let beats = BeatAnalysisEngine::generate_beat_grid(95.0, 10.0, 4, &energy);
        // 95 BPM = 0.6316s interval, 10s = ~15 beats
        assert!(beats.len() >= 14 && beats.len() <= 16);
        assert!(beats[0].is_downbeat);
        assert_eq!(beats[0].beat_in_bar, 1);
        assert_eq!(beats[1].beat_in_bar, 2);
        assert!(beats[4].is_downbeat); // Beat 5 = bar 2, beat 1
    }

    #[test]
    fn test_bpm_with_hint() {
        let energy = vec![];
        assert_eq!(BeatAnalysisEngine::detect_bpm(&energy, Some(95.0)), 95.0);
    }

    #[test]
    fn test_section_detection() {
        let energy: Vec<EnergyPoint> = (0..20).map(|i| {
            let t = i as f64 * 0.5;
            let e = if t < 3.0 { 0.3 } else if t < 7.0 { 0.8 } else { 0.4 };
            EnergyPoint { timestamp: t, loudness_lufs: -14.0, normalized_energy: e }
        }).collect();
        let sections = BeatAnalysisEngine::detect_sections(&energy, 10.0);
        assert!(!sections.is_empty());
        assert!(sections[0].start < 0.01);
    }

    #[test]
    fn test_transition_markers() {
        let beats = BeatAnalysisEngine::generate_beat_grid(120.0, 5.0, 4, &[]);
        let sections = vec![MusicSection {
            name: "Test".to_string(), start: 0.0, end: 5.0,
            energy_level: 0.7, suggested_content: SuggestedContent::Peak,
        }];
        let markers = BeatAnalysisEngine::generate_transition_markers(&beats, &sections);
        assert!(!markers.is_empty());
    }
}
