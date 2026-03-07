//! Beat & Music Structure Analysis Engine
//!
//! Uses FFmpeg to analyze audio tracks for:
//! - BPM detection via onset energy analysis with autocorrelation validation
//! - Beat grid generation (timestamp of every beat)
//! - Energy curve mapping (loudness over time)
//! - Structural section detection via energy gradient analysis
//! - Energy direction tracking per section (Rising/Falling/Stable)
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

/// Energy direction within a music section
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnergyDirection {
    Rising,
    Falling,
    Stable,
}

/// A section of the song structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicSection {
    pub name: String,
    pub start: f64,
    pub end: f64,
    pub energy_level: f64,
    pub suggested_content: SuggestedContent,
    /// Whether energy is rising, falling, or stable within this section
    pub energy_direction: EnergyDirection,
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

#[derive(Clone)]
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

    /// Measure loudness over time using volumedetect filter
    pub async fn measure_energy_curve(
        &self,
        path: &Path,
        duration: f64,
        window_size: f64,
    ) -> Result<Vec<EnergyPoint>, BeatAnalysisError> {
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

    /// Detect BPM using onset detection via energy analysis with autocorrelation validation.
    /// Falls back to provided BPM hint if detection is unreliable.
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

        let raw_bpm = if avg_interval > 0.0 {
            (60.0 / avg_interval).clamp(60.0, 200.0)
        } else {
            120.0
        };

        // Snap to nearest common tempo if within 5%
        let common_tempos = [
            60.0, 70.0, 75.0, 80.0, 85.0, 90.0, 95.0, 100.0, 105.0, 110.0, 115.0, 120.0,
            125.0, 128.0, 130.0, 135.0, 140.0, 145.0, 150.0, 155.0, 160.0, 170.0, 180.0,
        ];
        let snapped = common_tempos
            .iter()
            .min_by(|a, b| {
                (*a - raw_bpm)
                    .abs()
                    .partial_cmp(&(*b - raw_bpm).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
            .unwrap_or(raw_bpm);

        let base_bpm = if (snapped - raw_bpm).abs() / raw_bpm < 0.05 {
            snapped
        } else {
            raw_bpm
        };

        // Test candidates [bpm, bpm/2, bpm*2] with autocorrelation
        let candidates = [base_bpm, base_bpm / 2.0, base_bpm * 2.0];
        let best_bpm = candidates
            .iter()
            .filter(|&&b| b >= 60.0 && b <= 200.0)
            .max_by(|&&a, &&b| {
                let score_a = Self::autocorrelation_score(energy_curve, a);
                let score_b = Self::autocorrelation_score(energy_curve, b);
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
            .unwrap_or(base_bpm);

        // Clamp: if < 75 BPM, try doubling; if > 160, try halving
        if best_bpm < 75.0 {
            let doubled = best_bpm * 2.0;
            if doubled <= 200.0 {
                doubled
            } else {
                best_bpm
            }
        } else if best_bpm > 160.0 {
            let halved = best_bpm / 2.0;
            if halved >= 60.0 {
                halved
            } else {
                best_bpm
            }
        } else {
            best_bpm
        }
    }

    /// Compute autocorrelation score for a BPM candidate against the energy curve.
    /// Higher score means beat positions better align with energy peaks.
    fn autocorrelation_score(energy_curve: &[EnergyPoint], bpm: f64) -> f64 {
        if energy_curve.is_empty() || bpm <= 0.0 {
            return 0.0;
        }

        let beat_interval = 60.0 / bpm;
        let duration = energy_curve
            .last()
            .map(|p| p.timestamp)
            .unwrap_or(0.0);
        let num_beats = (duration / beat_interval).floor() as usize;

        if num_beats == 0 {
            return 0.0;
        }

        let mut score = 0.0;
        for i in 0..num_beats {
            let beat_time = i as f64 * beat_interval;
            let energy = Self::energy_at_time(energy_curve, beat_time);
            score += energy;
        }

        score / num_beats as f64
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

    /// Detect music sections based on energy gradient analysis.
    ///
    /// Algorithm:
    /// 1. Compute energy gradient (first derivative) with 3-point smoothing
    /// 2. Find plateau regions where gradient magnitude < 0.15
    /// 3. Merge sections shorter than 3 seconds with nearest-energy neighbor
    /// 4. Classify by energy relative to track average
    /// 5. Add energy direction analysis per section
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
                energy_direction: EnergyDirection::Stable,
            }];
        }

        let avg_energy = energy_curve
            .iter()
            .map(|p| p.normalized_energy)
            .sum::<f64>()
            / energy_curve.len() as f64;

        // Step 1: Compute energy gradient with 3-point smoothing
        let mut gradients = Vec::with_capacity(energy_curve.len());
        for i in 0..energy_curve.len() {
            let grad = if i == 0 {
                if energy_curve.len() > 1 {
                    let dt = energy_curve[1].timestamp - energy_curve[0].timestamp;
                    if dt > 0.0 {
                        (energy_curve[1].normalized_energy - energy_curve[0].normalized_energy)
                            / dt
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            } else if i == energy_curve.len() - 1 {
                let dt = energy_curve[i].timestamp - energy_curve[i - 1].timestamp;
                if dt > 0.0 {
                    (energy_curve[i].normalized_energy - energy_curve[i - 1].normalized_energy)
                        / dt
                } else {
                    0.0
                }
            } else {
                let dt = energy_curve[i + 1].timestamp - energy_curve[i - 1].timestamp;
                if dt > 0.0 {
                    (energy_curve[i + 1].normalized_energy
                        - energy_curve[i - 1].normalized_energy)
                        / dt
                } else {
                    0.0
                }
            };
            gradients.push(grad);
        }

        // 3-point smoothing of gradients
        let smoothed: Vec<f64> = (0..gradients.len())
            .map(|i| {
                let start = if i > 0 { i - 1 } else { 0 };
                let end = (i + 2).min(gradients.len());
                gradients[start..end].iter().sum::<f64>() / (end - start) as f64
            })
            .collect();

        // Step 2: Find section boundaries where gradient character changes
        let mut boundary_indices = vec![0usize];
        let mut in_plateau = smoothed[0].abs() < 0.15;

        for i in 1..smoothed.len() {
            let is_plateau = smoothed[i].abs() < 0.15;
            if is_plateau != in_plateau {
                boundary_indices.push(i);
                in_plateau = is_plateau;
            }
        }

        // Ensure we end at the last point
        if *boundary_indices.last().unwrap_or(&0) != energy_curve.len().saturating_sub(1) {
            boundary_indices.push(energy_curve.len().saturating_sub(1));
        }

        // Step 3: Build raw sections from boundaries
        let mut raw_sections: Vec<(f64, f64, f64)> = Vec::new(); // (start, end, avg_energy)
        for pair in boundary_indices.windows(2) {
            let start_idx = pair[0];
            let end_idx = pair[1];
            let start_time = energy_curve[start_idx].timestamp;
            let end_time = if end_idx >= energy_curve.len() - 1 {
                duration
            } else {
                energy_curve[end_idx].timestamp
            };

            if end_time <= start_time {
                continue;
            }

            let section_energies: Vec<f64> = energy_curve[start_idx..=end_idx.min(energy_curve.len() - 1)]
                .iter()
                .map(|p| p.normalized_energy)
                .collect();
            let avg = if section_energies.is_empty() {
                avg_energy
            } else {
                section_energies.iter().sum::<f64>() / section_energies.len() as f64
            };

            raw_sections.push((start_time, end_time, avg));
        }

        // Step 3b: Merge sections shorter than min duration with nearest-energy neighbor.
        // Scale threshold with track duration: short tracks need shorter minimum.
        let min_section_duration = (duration * 0.05).max(1.5).min(3.0);
        loop {
            let short_idx = raw_sections
                .iter()
                .position(|(s, e, _)| e - s < min_section_duration);
            if let Some(idx) = short_idx {
                if raw_sections.len() <= 1 {
                    break;
                }

                let (_, _, energy) = raw_sections[idx];
                let merge_with = if idx == 0 {
                    1
                } else if idx == raw_sections.len() - 1 {
                    idx - 1
                } else {
                    let prev_diff = (raw_sections[idx - 1].2 - energy).abs();
                    let next_diff = (raw_sections[idx + 1].2 - energy).abs();
                    if prev_diff <= next_diff {
                        idx - 1
                    } else {
                        idx + 1
                    }
                };

                let (low, high) = if merge_with < idx {
                    (merge_with, idx)
                } else {
                    (idx, merge_with)
                };
                let new_start = raw_sections[low].0;
                let new_end = raw_sections[high].1;
                let new_dur = new_end - new_start;
                let new_energy = if new_dur > 0.0 {
                    (raw_sections[low].2 * (raw_sections[low].1 - raw_sections[low].0)
                        + raw_sections[high].2 * (raw_sections[high].1 - raw_sections[high].0))
                        / new_dur
                } else {
                    avg_energy
                };

                raw_sections[low] = (new_start, new_end, new_energy);
                raw_sections.remove(high);
            } else {
                break;
            }
        }

        // Ensure we have at least one section
        if raw_sections.is_empty() {
            raw_sections.push((0.0, duration, avg_energy));
        }

        // Step 4 & 5: Classify sections and determine energy direction
        let mut sections = Vec::new();
        for (i, (start, end, energy)) in raw_sections.iter().enumerate() {
            let position = if duration > 0.0 {
                start / duration
            } else {
                0.0
            };
            let end_position = if duration > 0.0 {
                end / duration
            } else {
                1.0
            };

            // Energy direction analysis
            let section_points: Vec<f64> = energy_curve
                .iter()
                .filter(|p| p.timestamp >= *start && p.timestamp < *end)
                .map(|p| p.normalized_energy)
                .collect();

            let energy_direction = if section_points.len() < 2 {
                EnergyDirection::Stable
            } else {
                let half = section_points.len() / 2;
                let first_half =
                    section_points[..half].iter().sum::<f64>() / half.max(1) as f64;
                let second_half = section_points[half..].iter().sum::<f64>()
                    / (section_points.len() - half).max(1) as f64;
                let diff = second_half - first_half;
                if diff > 0.05 {
                    EnergyDirection::Rising
                } else if diff < -0.05 {
                    EnergyDirection::Falling
                } else {
                    EnergyDirection::Stable
                }
            };

            let suggested_content = if *energy < avg_energy * 0.85 {
                if position < 0.2 {
                    SuggestedContent::Establishing
                } else if end_position > 0.8 {
                    SuggestedContent::Resolution
                } else {
                    SuggestedContent::Building
                }
            } else if *energy > avg_energy * 1.15 {
                SuggestedContent::Peak
            } else {
                // Between 0.85*avg and 1.15*avg — classify by energy direction
                match energy_direction {
                    EnergyDirection::Rising => SuggestedContent::Building,
                    EnergyDirection::Falling => SuggestedContent::Resolution,
                    EnergyDirection::Stable => {
                        if *energy > avg_energy {
                            SuggestedContent::HeroMoment
                        } else {
                            SuggestedContent::Building
                        }
                    }
                }
            };

            let name = match suggested_content {
                SuggestedContent::Establishing => format!("Establishing {}", i + 1),
                SuggestedContent::Building => format!("Building {}", i + 1),
                SuggestedContent::Peak => format!("Peak {}", i + 1),
                SuggestedContent::HeroMoment => format!("Hero {}", i + 1),
                SuggestedContent::Resolution => format!("Resolution {}", i + 1),
                SuggestedContent::FlashCut => format!("Flash {}", i + 1),
            };

            sections.push(MusicSection {
                name,
                start: *start,
                end: *end,
                energy_level: *energy,
                suggested_content,
                energy_direction,
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

        markers.sort_by(|a, b| {
            a.timestamp
                .partial_cmp(&b.timestamp)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
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
        energy_curve
            .iter()
            .min_by(|a, b| {
                (a.timestamp - time)
                    .abs()
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
            EnergyPoint {
                timestamp: 0.0,
                loudness_lufs: -14.0,
                normalized_energy: 0.77,
            },
            EnergyPoint {
                timestamp: 5.0,
                loudness_lufs: -10.0,
                normalized_energy: 0.83,
            },
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
        let energy: Vec<EnergyPoint> = (0..20)
            .map(|i| {
                let t = i as f64 * 0.5;
                let e = if t < 3.0 {
                    0.3
                } else if t < 7.0 {
                    0.8
                } else {
                    0.4
                };
                EnergyPoint {
                    timestamp: t,
                    loudness_lufs: -14.0,
                    normalized_energy: e,
                }
            })
            .collect();
        let sections = BeatAnalysisEngine::detect_sections(&energy, 10.0);
        assert!(!sections.is_empty());
        assert!(sections[0].start < 0.01);
        // Should detect multiple sections due to energy changes
        assert!(
            sections.len() >= 2,
            "Expected at least 2 sections, got {}",
            sections.len()
        );
        // Should have a Peak section (energy 0.8 > avg*1.15)
        let has_peak = sections
            .iter()
            .any(|s| matches!(s.suggested_content, SuggestedContent::Peak));
        assert!(has_peak, "Expected at least one Peak section");
        // All sections should have energy direction
        for section in &sections {
            // Just verify the field exists and is valid
            let _ = &section.energy_direction;
        }
    }

    #[test]
    fn test_transition_markers() {
        let beats = BeatAnalysisEngine::generate_beat_grid(120.0, 5.0, 4, &[]);
        let sections = vec![MusicSection {
            name: "Test".to_string(),
            start: 0.0,
            end: 5.0,
            energy_level: 0.7,
            suggested_content: SuggestedContent::Peak,
            energy_direction: EnergyDirection::Stable,
        }];
        let markers = BeatAnalysisEngine::generate_transition_markers(&beats, &sections);
        assert!(!markers.is_empty());
    }

    #[test]
    fn test_autocorrelation_score() {
        // Energy peaks at regular intervals should score higher for matching BPM
        let energy: Vec<EnergyPoint> = (0..20)
            .map(|i| {
                let t = i as f64 * 0.5;
                // Create peaks at 1s intervals (60 BPM)
                let e = if i % 2 == 0 { 0.8 } else { 0.3 };
                EnergyPoint {
                    timestamp: t,
                    loudness_lufs: -14.0,
                    normalized_energy: e,
                }
            })
            .collect();

        let score_60 = BeatAnalysisEngine::autocorrelation_score(&energy, 60.0);
        let score_90 = BeatAnalysisEngine::autocorrelation_score(&energy, 90.0);

        // 60 BPM should align better with peaks at 1s intervals
        assert!(
            score_60 > score_90,
            "60 BPM score ({}) should be higher than 90 BPM score ({})",
            score_60,
            score_90
        );
    }
}
