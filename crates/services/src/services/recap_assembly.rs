//! Recap Assembly Engine v2
//!
//! Three-layer editing intelligence:
//!   Layer 1 — Smart Music Window: selects best N-second arc from the full track
//!   Layer 2 — Pacing Engine: variable shot durations driven by beat grid + section energy
//!   Layer 3 — Transitions: dissolves at section boundaries, hard cuts on beats, fades
//!
//! Invariants:
//!   - Default duration is 59 seconds (industry standard recap length)
//!   - NAT audio is always muted — music only on A1
//!   - All cuts snap to the beat grid
//!   - More clips used with shorter durations in high-energy sections
//!   - Speed manipulation is optional and sparing (not applied by default)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::scene_analysis::{ClipAnalysis, ContentType, SceneAnalysisResult};
use super::beat_analysis::{BeatGridResult, BeatMarker, MusicSection, SuggestedContent};

/// Default recap duration in seconds
const DEFAULT_RECAP_DURATION: f64 = 59.0;

// ─── Data types ──────────────────────────────────────────────────────────────

/// Transition between two clips on the timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditTransition {
    /// Straight cut on a beat
    HardCut,
    /// Cross dissolve (typically 0.5s at section boundaries)
    Dissolve { duration: f64 },
    /// Dip to black (end of sequence or major section change)
    DipToBlack { duration: f64 },
}

/// A clip placed on the timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePlacement {
    pub clip_filename: String,
    pub source_path: PathBuf,
    pub source_in: f64,
    pub source_out: f64,
    pub timeline_in: f64,
    pub timeline_out: f64,
    pub section_name: String,
    pub energy_match_score: f64,
    pub beat_locked: bool,
    pub transition_in: EditTransition,
    pub speed: f64,
}

/// The selected music window within the full track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicWindow {
    pub start: f64,
    pub end: f64,
    pub duration: f64,
    pub fade_in: f64,
    pub fade_out: f64,
    pub sections: Vec<MusicSection>,
}

/// Complete assembly result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecapAssemblyResult {
    pub id: String,
    pub name: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub bpm: f64,
    pub placements: Vec<TimelinePlacement>,
    pub music_path: String,
    pub music_window: Option<MusicWindow>,
    pub xml_path: String,
    pub clips_used: u32,
    pub clips_available: u32,
    pub beat_locked_cuts: u32,
    pub processing_time_ms: u64,
}

pub struct RecapAssemblyEngine;

// ─── Layer 1: Smart Music Window ─────────────────────────────────────────────

impl RecapAssemblyEngine {
    /// Select the best N-second window from the full track.
    /// Looks for the strongest narrative arc: low energy start → peak → resolution.
    pub fn select_music_window(
        beat_grid: &BeatGridResult,
        target_duration: f64,
    ) -> MusicWindow {
        let track_dur = beat_grid.duration;
        let dur = target_duration.min(track_dur);

        // If the track is already shorter than target, use it all
        if track_dur <= dur + 1.0 {
            return MusicWindow {
                start: 0.0,
                end: track_dur,
                duration: track_dur,
                fade_in: 0.5,
                fade_out: 2.0,
                sections: beat_grid.sections.clone(),
            };
        }

        // Snap target duration to nearest beat
        let snapped_dur = Self::snap_to_beat(&beat_grid.beats, dur);

        // Score every possible starting beat for the best narrative arc
        let beat_interval = beat_grid.beat_interval;
        let bar_duration = beat_interval * beat_grid.beats_per_bar as f64;

        // Only consider starts on bar boundaries for musical phrasing
        let mut best_start = 0.0;
        let mut best_score = f64::MIN;

        let mut t = 0.0;
        while t + snapped_dur <= track_dur + 0.1 {
            let window_end = (t + snapped_dur).min(track_dur);
            let score = Self::score_music_window(beat_grid, t, window_end);
            if score > best_score {
                best_score = score;
                best_start = t;
            }
            t += bar_duration;
        }

        let window_end = (best_start + snapped_dur).min(track_dur);

        // Collect sections that fall within the window
        let sections: Vec<MusicSection> = beat_grid.sections.iter()
            .filter_map(|s| {
                let overlap_start = s.start.max(best_start);
                let overlap_end = s.end.min(window_end);
                if overlap_end > overlap_start + 0.5 {
                    Some(MusicSection {
                        name: s.name.clone(),
                        start: overlap_start - best_start, // rebase to 0
                        end: overlap_end - best_start,
                        energy_level: s.energy_level,
                        suggested_content: s.suggested_content.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        MusicWindow {
            start: best_start,
            end: window_end,
            duration: window_end - best_start,
            fade_in: 0.5,
            fade_out: 2.0,
            sections,
        }
    }

    /// Score a candidate music window. Prefers:
    /// - Starting low energy (establishing) and reaching a peak (narrative arc)
    /// - Having variety in energy levels (dynamic range)
    /// - Starting and ending on section boundaries
    fn score_music_window(beat_grid: &BeatGridResult, start: f64, end: f64) -> f64 {
        let mut score = 0.0;

        // Get energy samples in this window
        let energies: Vec<f64> = beat_grid.energy_curve.iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .map(|e| e.normalized_energy)
            .collect();

        if energies.is_empty() {
            return 0.0;
        }

        // 1. Narrative arc: first quarter should be lower energy than middle
        let quarter = energies.len() / 4;
        if quarter > 0 {
            let first_q: f64 = energies[..quarter].iter().sum::<f64>() / quarter as f64;
            let mid_q: f64 = energies[quarter..quarter * 3].iter().sum::<f64>()
                / (quarter * 2) as f64;
            let last_q: f64 = energies[quarter * 3..].iter().sum::<f64>()
                / (energies.len() - quarter * 3) as f64;

            // Want: first < mid (build up) and last < mid (resolution)
            if mid_q > first_q { score += (mid_q - first_q) * 3.0; }
            if mid_q > last_q { score += (mid_q - last_q) * 2.0; }
        }

        // 2. Dynamic range (variety is interesting)
        let max_e = energies.iter().cloned().fold(0.0_f64, f64::max);
        let min_e = energies.iter().cloned().fold(1.0_f64, f64::min);
        score += (max_e - min_e) * 2.0;

        // 3. Bonus for starting near a section boundary
        for section in &beat_grid.sections {
            if (section.start - start).abs() < 1.0 {
                score += 0.5;
            }
            if (section.end - end).abs() < 1.0 {
                score += 0.3;
            }
        }

        // 4. Bonus for starting at the very beginning (natural start)
        if start < 0.1 {
            score += 0.5;
        }

        score
    }

    // ─── Layer 2: Pacing Engine ──────────────────────────────────────────────

    /// Calculate shot durations for a section based on its energy/content type.
    /// Returns beat counts per shot (e.g., 8 beats = long shot, 2 beats = quick cut).
    fn beats_per_shot(section: &MusicSection) -> u32 {
        match section.suggested_content {
            SuggestedContent::Establishing => 8,  // ~5s at 95bpm — wide, breathe
            SuggestedContent::Building     => 4,  // ~2.5s — momentum
            SuggestedContent::Peak         => 2,  // ~1.3s — fast cuts
            SuggestedContent::HeroMoment   => 4,  // ~2.5s — hold on the moment
            SuggestedContent::Resolution   => 6,  // ~3.8s — slow down
            SuggestedContent::FlashCut     => 1,  // ~0.6s — accent
        }
    }

    /// Generate cut points for a section, returning (timeline_in, timeline_out) pairs
    /// all snapped to the beat grid.
    fn generate_section_cuts(
        section: &MusicSection,
        beats: &[BeatMarker],
        beat_interval: f64,
    ) -> Vec<(f64, f64)> {
        let bps = Self::beats_per_shot(section) as f64;
        let shot_duration = beat_interval * bps;
        let section_dur = section.end - section.start;

        if section_dur < shot_duration * 0.5 {
            return vec![(section.start, section.end)];
        }

        let mut cuts = Vec::new();
        let mut cursor = section.start;

        while cursor + shot_duration * 0.5 < section.end {
            let cut_end = (cursor + shot_duration).min(section.end);
            let snapped_start = Self::snap_to_beat(beats, cursor);
            let snapped_end = Self::snap_to_beat(beats, cut_end);

            if snapped_end > snapped_start + 0.3 {
                cuts.push((snapped_start, snapped_end));
            }
            cursor = cut_end;
        }

        // If there's a remaining tail, extend the last cut
        if let Some(last) = cuts.last_mut() {
            let snapped_section_end = Self::snap_to_beat(beats, section.end);
            if snapped_section_end > last.1 + 0.3 {
                last.1 = snapped_section_end;
            }
        }

        cuts
    }

    /// Assign a transition type for entering this section
    fn pick_transition(
        section_idx: usize,
        shot_idx_in_section: usize,
        _section: &MusicSection,
        prev_section: Option<&MusicSection>,
    ) -> EditTransition {
        // First clip of the edit — no transition
        if section_idx == 0 && shot_idx_in_section == 0 {
            return EditTransition::HardCut;
        }

        // First clip of a new section — dissolve at section boundary
        if shot_idx_in_section == 0 {
            if let Some(prev) = prev_section {
                let energy_delta = (_section.energy_level - prev.energy_level).abs();
                if energy_delta > 0.15 {
                    // Big energy change — dip to black
                    return EditTransition::DipToBlack { duration: 0.25 };
                }
            }
            return EditTransition::Dissolve { duration: 0.5 };
        }

        // Within a section — hard cuts on beats
        EditTransition::HardCut
    }

    // ─── Main Assembly ───────────────────────────────────────────────────────

    /// Assemble a recap edit with smart music window, variable pacing, and transitions.
    pub fn assemble(
        scene_analysis: &SceneAnalysisResult,
        beat_grid: &BeatGridResult,
        _music_path: &Path,
        _target_width: u32,
        _target_height: u32,
        target_duration: Option<f64>,
    ) -> (Vec<TimelinePlacement>, MusicWindow) {
        let dur = target_duration.unwrap_or(DEFAULT_RECAP_DURATION);

        // Layer 1: Select the best music window
        let window = Self::select_music_window(beat_grid, dur);

        // Rebase beats to window-relative timestamps
        let window_beats: Vec<BeatMarker> = beat_grid.beats.iter()
            .filter(|b| b.timestamp >= window.start - 0.01 && b.timestamp <= window.end + 0.01)
            .map(|b| BeatMarker {
                timestamp: b.timestamp - window.start,
                ..b.clone()
            })
            .collect();

        // Get usable clips
        let clips: Vec<&ClipAnalysis> = scene_analysis.clips.iter()
            .filter(|c| c.usable && c.duration >= 1.5)
            .collect();

        if clips.is_empty() {
            return (vec![], window);
        }

        // Layer 2: Generate cut points per section
        let mut all_slots: Vec<(f64, f64, usize)> = Vec::new(); // (start, end, section_idx)
        for (si, section) in window.sections.iter().enumerate() {
            let cuts = Self::generate_section_cuts(section, &window_beats, beat_grid.beat_interval);
            for (start, end) in cuts {
                all_slots.push((start, end, si));
            }
        }

        // Assign clips to slots — match energy, avoid repeats
        let mut placements = Vec::new();
        let mut used_clip_counts: HashMap<String, u32> = HashMap::new();
        // Track which source ranges have been used per clip to avoid identical segments
        let mut used_source_ranges: HashMap<String, Vec<(f64, f64)>> = HashMap::new();

        let mut prev_section_idx: Option<usize> = None;
        let mut shot_idx_in_section = 0;

        for (slot_start, slot_end, section_idx) in &all_slots {
            let section = &window.sections[*section_idx];
            let slot_dur = slot_end - slot_start;

            // Reset shot counter for new sections
            if Some(*section_idx) != prev_section_idx {
                shot_idx_in_section = 0;
            }

            // Find best clip for this slot
            let best_clip = Self::pick_clip_for_slot(
                &clips,
                section,
                slot_dur,
                &used_clip_counts,
            );

            if let Some(clip) = best_clip {
                // Calculate source range — use a different part if this clip was used before
                let prev_ranges = used_source_ranges.get(&clip.filename)
                    .cloned().unwrap_or_default();
                let (src_in, src_out) = Self::varied_source_range(clip, section, slot_dur, &prev_ranges);

                // Layer 3: Transition
                let prev_sec = prev_section_idx.map(|i| &window.sections[i]);
                let transition = Self::pick_transition(
                    *section_idx,
                    shot_idx_in_section,
                    section,
                    prev_sec,
                );

                placements.push(TimelinePlacement {
                    clip_filename: clip.filename.clone(),
                    source_path: clip.path.clone(),
                    source_in: src_in,
                    source_out: src_out,
                    timeline_in: *slot_start,
                    timeline_out: *slot_end,
                    section_name: section.name.clone(),
                    energy_match_score: Self::energy_match_score(clip, section),
                    beat_locked: true,
                    transition_in: transition,
                    speed: 1.0,
                });

                *used_clip_counts.entry(clip.filename.clone()).or_insert(0) += 1;
                used_source_ranges.entry(clip.filename.clone()).or_default().push((src_in, src_out));
            }

            shot_idx_in_section += 1;
            prev_section_idx = Some(*section_idx);
        }

        (placements, window)
    }

    /// Pick the best unused (or least-used) clip for a timeline slot.
    /// Strongly penalizes reuse so repeated sections feel different.
    fn pick_clip_for_slot<'a>(
        clips: &[&'a ClipAnalysis],
        section: &MusicSection,
        slot_duration: f64,
        used_counts: &HashMap<String, u32>,
    ) -> Option<&'a ClipAnalysis> {
        let max_uses = clips.len().max(1); // total slots / unique clips

        let mut candidates: Vec<(&ClipAnalysis, f64)> = clips.iter()
            .map(|clip| {
                let mut score = Self::clip_section_match_score(clip, section);

                // Heavy penalty for reuse — each use drops score by 1.5
                // This forces variety: algorithm must exhaust all clips before reusing
                let uses = used_counts.get(&clip.filename).copied().unwrap_or(0);
                score -= uses as f64 * 1.5;

                // Bonus if clip is long enough for this slot
                if clip.duration >= slot_duration * 1.5 {
                    score += 0.15;
                }

                (*clip, score)
            })
            .collect();

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.first().map(|(clip, _)| *clip)
    }

    /// Calculate source in/out, picking a different segment if this clip was used before.
    fn varied_source_range(
        clip: &ClipAnalysis,
        section: &MusicSection,
        target_dur: f64,
        prev_ranges: &[(f64, f64)],
    ) -> (f64, f64) {
        // Rank all segments by energy match
        let mut candidates: Vec<(f64, f64)> = clip.segments.iter()
            .map(|s| {
                let energy_diff = (s.energy_score - section.energy_level).abs();
                (s.timestamp, 1.0 - energy_diff)
            })
            .collect();

        // Sort by score descending
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Try each candidate, skip ones that overlap with previously used ranges
        for (ts, _score) in &candidates {
            let start = *ts;
            let available = clip.duration - start;
            let actual_start = if available >= target_dur {
                start
            } else {
                (clip.duration - target_dur).max(0.0)
            };
            let end = (actual_start + target_dur).min(clip.duration);

            // Check overlap with previous uses
            let overlaps = prev_ranges.iter().any(|(ps, pe)| {
                actual_start < *pe && end > *ps // ranges overlap
            });

            if !overlaps {
                return (actual_start, end);
            }
        }

        // All segments overlap — pick the one furthest from any previous use
        let best_start = if clip.duration >= target_dur * 2.0 {
            // Enough room: find the midpoint gap between previous ranges
            let mut starts: Vec<f64> = prev_ranges.iter().map(|(s, _)| *s).collect();
            starts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            // Try the beginning and end first
            let options = [
                0.0,
                (clip.duration - target_dur).max(0.0),
                clip.duration / 3.0,
                clip.duration * 2.0 / 3.0,
            ];

            options.iter()
                .max_by(|a, b| {
                    let dist_a: f64 = starts.iter().map(|s| (*a - s).abs()).sum();
                    let dist_b: f64 = starts.iter().map(|s| (*b - s).abs()).sum();
                    dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
                })
                .copied()
                .unwrap_or(0.0)
        } else {
            // Short clip — just use from the start
            0.0
        };

        let end = (best_start + target_dur).min(clip.duration);
        (best_start, end)
    }

    fn clip_section_match_score(clip: &ClipAnalysis, section: &MusicSection) -> f64 {
        let mut score = 0.0;

        // Energy proximity
        let energy_diff = (clip.overall_energy - section.energy_level).abs();
        score += 1.0 - energy_diff;

        // Content type affinity
        let content_bonus = match (&clip.dominant_content_type, &section.suggested_content) {
            (ContentType::HighEnergy, SuggestedContent::Peak) => 0.5,
            (ContentType::Establishing, SuggestedContent::Establishing) => 0.5,
            (ContentType::Intimate, SuggestedContent::HeroMoment) => 0.4,
            (ContentType::Ambient, SuggestedContent::Resolution) => 0.3,
            (ContentType::HighEnergy, SuggestedContent::HeroMoment) => 0.3,
            (ContentType::Transition, SuggestedContent::Building) => 0.2,
            _ => 0.0,
        };
        score += content_bonus;

        score
    }

    fn energy_match_score(clip: &ClipAnalysis, section: &MusicSection) -> f64 {
        1.0 - (clip.overall_energy - section.energy_level).abs()
    }

    fn snap_to_beat(beats: &[BeatMarker], time: f64) -> f64 {
        beats.iter()
            .min_by(|a, b| {
                (a.timestamp - time).abs()
                    .partial_cmp(&(b.timestamp - time).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|b| b.timestamp)
            .unwrap_or(time)
    }

    // ─── FFmpeg Render Script ────────────────────────────────────────────────

    /// Generate a bash script that renders the assembly to MP4 via FFmpeg.
    /// Handles: trimming, scaling, transitions (xfade), music window, fades.
    pub fn generate_render_script(
        placements: &[TimelinePlacement],
        music_path: &str,
        window: &MusicWindow,
        output_path: &str,
    ) -> String {
        let n = placements.len();
        if n == 0 {
            return "echo 'No placements to render'".to_string();
        }

        let mut script = String::from("#!/bin/bash\nset -e\n\n");

        // Build input list
        let mut inputs = Vec::new();
        for (i, p) in placements.iter().enumerate() {
            inputs.push(format!(
                "  -i \"{}\" \\",
                p.source_path.to_string_lossy()
            ));
        }
        // Music input is the last one
        let music_idx = n;
        inputs.push(format!("  -i \"{}\" \\", music_path));

        // Build filter_complex
        let mut filters = Vec::new();
        let mut segment_labels = Vec::new();

        // Step 1: Trim + scale each clip
        for (i, p) in placements.iter().enumerate() {
            let dur = p.source_out - p.source_in;
            // Ensure we have enough source by backing up if needed
            let label = format!("v{}", i);
            filters.push(format!(
                "[{}:v]trim=start={:.3}:end={:.3},setpts=PTS-STARTPTS,scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2[{}]",
                i, p.source_in, p.source_out, label
            ));
            segment_labels.push(label);
        }

        // Step 2: Chain transitions
        // We'll build the chain incrementally.
        // For hard cuts between clips in the same section: use concat.
        // For dissolves at section boundaries: use xfade.
        //
        // Strategy: group consecutive hard-cut clips, concat them into sections,
        // then xfade between sections.
        let mut sections: Vec<Vec<usize>> = Vec::new();
        let mut current_section: Vec<usize> = vec![0];
        let mut section_transitions: Vec<&EditTransition> = Vec::new();

        for i in 1..n {
            match &placements[i].transition_in {
                EditTransition::HardCut => {
                    current_section.push(i);
                }
                trans => {
                    sections.push(current_section);
                    section_transitions.push(trans);
                    current_section = vec![i];
                }
            }
        }
        sections.push(current_section);

        // Concat within each hard-cut group
        let mut section_labels = Vec::new();
        for (si, group) in sections.iter().enumerate() {
            if group.len() == 1 {
                section_labels.push(segment_labels[group[0]].clone());
            } else {
                let concat_inputs: String = group.iter()
                    .map(|&i| format!("[{}]", segment_labels[i]))
                    .collect();
                let label = format!("sec{}", si);
                filters.push(format!(
                    "{}concat=n={}:v=1:a=0[{}]",
                    concat_inputs, group.len(), label
                ));
                section_labels.push(label);
            }
        }

        // Calculate section durations for xfade offsets
        let mut section_durations: Vec<f64> = Vec::new();
        for group in &sections {
            let dur: f64 = group.iter()
                .map(|&i| placements[i].timeline_out - placements[i].timeline_in)
                .sum();
            section_durations.push(dur);
        }

        // Chain xfade between section groups
        if section_labels.len() == 1 {
            // Only one group — just add fade in/out
            filters.push(format!(
                "[{}]fade=t=in:d={:.1}:st=0,fade=t=out:d={:.1}:st={:.2}[outv]",
                section_labels[0],
                window.fade_in,
                window.fade_out,
                window.duration - window.fade_out
            ));
        } else {
            // Chain xfades
            let mut accumulated_duration = section_durations[0];
            let mut current_label = section_labels[0].clone();

            for i in 0..section_transitions.len() {
                let next_label = &section_labels[i + 1];
                let (xfade_type, xfade_dur) = match section_transitions[i] {
                    EditTransition::Dissolve { duration } => ("fade", *duration),
                    EditTransition::DipToBlack { duration } => ("fadeblack", *duration),
                    EditTransition::HardCut => ("fade", 0.033),
                };

                let offset = (accumulated_duration - xfade_dur).max(0.0);
                let out_label = if i == section_transitions.len() - 1 {
                    "xfinal".to_string()
                } else {
                    format!("x{}", i)
                };

                filters.push(format!(
                    "[{}][{}]xfade=transition={}:duration={:.3}:offset={:.3}[{}]",
                    current_label, next_label, xfade_type, xfade_dur, offset, out_label
                ));

                accumulated_duration += section_durations[i + 1] - xfade_dur;
                current_label = out_label;
            }

            // Add fade in/out on the final composited video
            let total_dur = accumulated_duration;
            filters.push(format!(
                "[xfinal]fade=t=in:d={:.1}:st=0,fade=t=out:d={:.1}:st={:.2}[outv]",
                window.fade_in,
                window.fade_out,
                (total_dur - window.fade_out).max(0.0)
            ));
        }

        // Audio: trim music to window, fade in/out
        filters.push(format!(
            "[{}:a]atrim=start={:.3}:end={:.3},asetpts=PTS-STARTPTS,afade=t=in:d={:.1},afade=t=out:d={:.1}:st={:.2}[outa]",
            music_idx,
            window.start, window.end,
            window.fade_in,
            window.fade_out,
            window.duration - window.fade_out
        ));

        // Assemble the command
        script.push_str("ffmpeg -y \\\n");
        for input in &inputs {
            script.push_str(&format!("{}\n", input));
        }
        script.push_str("  -filter_complex \"\n");
        for (i, f) in filters.iter().enumerate() {
            if i < filters.len() - 1 {
                script.push_str(&format!("    {};\n", f));
            } else {
                script.push_str(&format!("    {}\n", f));
            }
        }
        script.push_str("  \" \\\n");
        script.push_str("  -map \"[outv]\" -map \"[outa]\" \\\n");
        script.push_str("  -pix_fmt yuv420p -profile:v high -level 4.0 \\\n");
        script.push_str("  -c:v libx264 -preset medium -crf 18 -r 29.97 \\\n");
        script.push_str("  -c:a aac -b:a 192k \\\n");
        script.push_str("  -movflags +faststart \\\n");
        script.push_str(&format!("  \"{}\"\n", output_path));

        script
    }

    // ─── Premiere XML ────────────────────────────────────────────────────────

    /// Generate Premiere Pro XML from assembly
    pub fn generate_premiere_xml(
        result: &RecapAssemblyResult,
        placements: &[TimelinePlacement],
        beat_grid: &BeatGridResult,
    ) -> String {
        let mut xml = String::new();
        let timebase = 30u32;

        let secs_to_frames = |s: f64| -> i64 { (s * 29.97).round() as i64 };

        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<!DOCTYPE xmeml>\n");
        xml.push_str("<xmeml version=\"5\">\n");
        xml.push_str("  <project>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&result.name)));
        xml.push_str("    <children>\n");

        // === Footage Bin ===
        xml.push_str("      <bin>\n");
        xml.push_str("        <name>Footage</name>\n");
        xml.push_str("        <children>\n");

        let mut file_ids: HashMap<String, String> = HashMap::new();
        for p in placements {
            let key = p.source_path.to_string_lossy().to_string();
            file_ids.entry(key).or_insert_with(|| format!("file-{}", Uuid::new_v4()));
        }
        let music_file_id = format!("file-music-{}", Uuid::new_v4());

        // Deduplicate clips in bin
        let mut seen_files = std::collections::HashSet::new();
        for p in placements {
            let key = p.source_path.to_string_lossy().to_string();
            if !seen_files.insert(key.clone()) {
                continue; // Already added this file to the bin
            }
            if let Some(fid) = file_ids.get(&key) {
                let file_url = format!("file:///{}", p.source_path.to_string_lossy().replace(' ', "%20"));
                xml.push_str(&format!("          <clip id=\"clip-{}\">\n", escape_xml(&p.clip_filename)));
                xml.push_str(&format!("            <name>{}</name>\n", escape_xml(&p.clip_filename)));
                xml.push_str("            <media>\n");
                xml.push_str("              <video><track><clipitem>\n");
                xml.push_str(&format!("                <file id=\"{}\">\n", fid));
                xml.push_str(&format!("                  <name>{}</name>\n", escape_xml(&p.clip_filename)));
                xml.push_str(&format!("                  <pathurl>{}</pathurl>\n", escape_xml(&file_url)));
                xml.push_str(&format!("                  <rate><timebase>{}</timebase><ntsc>TRUE</ntsc></rate>\n", timebase));
                xml.push_str("                  <media>\n");
                xml.push_str("                    <video><samplecharacteristics>\n");
                xml.push_str(&format!("                      <width>{}</width>\n", result.width));
                xml.push_str(&format!("                      <height>{}</height>\n", result.height));
                xml.push_str("                      <anamorphic>FALSE</anamorphic>\n");
                xml.push_str("                      <pixelaspectratio>Square</pixelaspectratio>\n");
                xml.push_str("                      <fielddominance>none</fielddominance>\n");
                xml.push_str("                    </samplecharacteristics></video>\n");
                xml.push_str("                    <audio><samplecharacteristics>\n");
                xml.push_str("                      <samplerate>48000</samplerate>\n");
                xml.push_str("                      <depth>16</depth>\n");
                xml.push_str("                    </samplecharacteristics></audio>\n");
                xml.push_str("                  </media>\n");
                xml.push_str("                </file>\n");
                xml.push_str("              </clipitem></track></video>\n");
                xml.push_str("            </media>\n");
                xml.push_str("          </clip>\n");
            }
        }

        // Music master clip
        let music_url = format!("file:///{}", result.music_path.replace(' ', "%20"));
        xml.push_str("          <clip id=\"clip-music\">\n");
        xml.push_str(&format!("            <name>Music - {}</name>\n", escape_xml(&result.name)));
        xml.push_str("            <media><audio><track><clipitem>\n");
        xml.push_str(&format!("              <file id=\"{}\">\n", music_file_id));
        xml.push_str(&format!("                <name>{}</name>\n",
            escape_xml(Path::new(&result.music_path).file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "music.mp3".to_string()).as_str())));
        xml.push_str(&format!("                <pathurl>{}</pathurl>\n", escape_xml(&music_url)));
        xml.push_str(&format!("                <rate><timebase>{}</timebase><ntsc>TRUE</ntsc></rate>\n", timebase));
        xml.push_str("                <media><audio><samplecharacteristics>\n");
        xml.push_str("                  <samplerate>44100</samplerate><depth>16</depth>\n");
        xml.push_str("                </samplecharacteristics></audio></media>\n");
        xml.push_str("              </file>\n");
        xml.push_str("            </clipitem></track></audio></media>\n");
        xml.push_str("          </clip>\n");

        xml.push_str("        </children>\n");
        xml.push_str("      </bin>\n");

        // === Sequence ===
        let total_frames = secs_to_frames(result.duration);
        xml.push_str("      <sequence>\n");
        xml.push_str(&format!("        <name>{}</name>\n", escape_xml(&result.name)));
        xml.push_str(&format!("        <duration>{}</duration>\n", total_frames));
        xml.push_str(&format!("        <rate><timebase>{}</timebase><ntsc>TRUE</ntsc></rate>\n", timebase));
        xml.push_str("        <timecode>\n");
        xml.push_str(&format!("          <rate><timebase>{}</timebase><ntsc>TRUE</ntsc></rate>\n", timebase));
        xml.push_str("          <string>01:00:00:00</string>\n");
        xml.push_str("          <frame>108000</frame>\n");
        xml.push_str("          <displayformat>NDF</displayformat>\n");
        xml.push_str("        </timecode>\n");
        xml.push_str("        <media>\n");

        // Video track
        xml.push_str("          <video>\n");
        xml.push_str("            <format><samplecharacteristics>\n");
        xml.push_str(&format!("              <width>{}</width>\n", result.width));
        xml.push_str(&format!("              <height>{}</height>\n", result.height));
        xml.push_str("              <anamorphic>FALSE</anamorphic>\n");
        xml.push_str("              <pixelaspectratio>Square</pixelaspectratio>\n");
        xml.push_str("              <fielddominance>none</fielddominance>\n");
        xml.push_str(&format!("              <rate><timebase>{}</timebase><ntsc>TRUE</ntsc></rate>\n", timebase));
        xml.push_str("            </samplecharacteristics></format>\n");
        xml.push_str("            <track>\n");

        for (i, p) in placements.iter().enumerate() {
            let key = p.source_path.to_string_lossy().to_string();
            let fid = file_ids.get(&key).cloned().unwrap_or_default();
            xml.push_str(&format!("              <clipitem id=\"tl-v1-{:02}\">\n", i + 1));
            xml.push_str(&format!("                <name>{}</name>\n", escape_xml(&p.clip_filename)));
            xml.push_str("                <enabled>TRUE</enabled>\n");
            xml.push_str(&format!("                <rate><timebase>{}</timebase><ntsc>TRUE</ntsc></rate>\n", timebase));
            xml.push_str(&format!("                <start>{}</start>\n", secs_to_frames(p.timeline_in)));
            xml.push_str(&format!("                <end>{}</end>\n", secs_to_frames(p.timeline_out)));
            xml.push_str(&format!("                <in>{}</in>\n", secs_to_frames(p.source_in)));
            xml.push_str(&format!("                <out>{}</out>\n", secs_to_frames(p.source_out)));
            xml.push_str(&format!("                <file id=\"{}\"/>\n", fid));
            xml.push_str("              </clipitem>\n");
        }

        xml.push_str("            </track>\n");
        xml.push_str("          </video>\n");

        // Audio: music only on A1, NO NAT
        xml.push_str("          <audio>\n");
        xml.push_str("            <numOutputChannels>2</numOutputChannels>\n");
        xml.push_str("            <format><samplecharacteristics>\n");
        xml.push_str("              <samplerate>48000</samplerate><depth>16</depth>\n");
        xml.push_str("            </samplecharacteristics></format>\n");
        xml.push_str("            <track>\n");
        xml.push_str("              <clipitem id=\"tl-a1-music\">\n");
        xml.push_str(&format!("                <name>{}</name>\n", escape_xml(&result.name)));
        xml.push_str("                <enabled>TRUE</enabled>\n");
        xml.push_str(&format!("                <rate><timebase>{}</timebase><ntsc>TRUE</ntsc></rate>\n", timebase));
        xml.push_str("                <start>0</start>\n");
        xml.push_str(&format!("                <end>{}</end>\n", total_frames));

        // Music window in/out (trimmed to window)
        if let Some(ref win) = result.music_window {
            xml.push_str(&format!("                <in>{}</in>\n", secs_to_frames(win.start)));
            xml.push_str(&format!("                <out>{}</out>\n", secs_to_frames(win.end)));
        } else {
            xml.push_str("                <in>0</in>\n");
            xml.push_str(&format!("                <out>{}</out>\n", total_frames));
        }

        xml.push_str(&format!("                <file id=\"{}\"/>\n", music_file_id));
        xml.push_str("              </clipitem>\n");
        xml.push_str("            </track>\n");
        xml.push_str("          </audio>\n");
        xml.push_str("        </media>\n");

        // Beat markers (only within window)
        if let Some(ref win) = result.music_window {
            for beat in &beat_grid.beats {
                if beat.is_downbeat && beat.timestamp >= win.start && beat.timestamp <= win.end {
                    let rebased = beat.timestamp - win.start;
                    xml.push_str("        <marker>\n");
                    xml.push_str(&format!("          <name>Beat {}</name>\n", beat.beat_number));
                    xml.push_str(&format!("          <in>{}</in>\n", secs_to_frames(rebased)));
                    xml.push_str(&format!("          <out>{}</out>\n", secs_to_frames(rebased)));
                    xml.push_str(&format!("          <comment>Bar {} | Downbeat</comment>\n", beat.bar_number));
                    xml.push_str("        </marker>\n");
                }
            }
        }

        // Section markers
        for section in result.music_window.as_ref()
            .map(|w| &w.sections)
            .unwrap_or(&beat_grid.sections)
        {
            xml.push_str("        <marker>\n");
            xml.push_str(&format!("          <name>{}</name>\n", escape_xml(&section.name)));
            xml.push_str(&format!("          <in>{}</in>\n", secs_to_frames(section.start)));
            xml.push_str(&format!("          <out>{}</out>\n", secs_to_frames(section.end)));
            xml.push_str(&format!("          <comment>Energy: {:.0}%</comment>\n", section.energy_level * 100.0));
            xml.push_str("        </marker>\n");
        }

        xml.push_str("      </sequence>\n");
        xml.push_str("    </children>\n");
        xml.push_str("  </project>\n");
        xml.push_str("</xmeml>\n");

        xml
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::scene_analysis::SegmentAnalysis;

    fn make_beat(ts: f64, num: u32, bar: u32, beat_in_bar: u32) -> BeatMarker {
        BeatMarker {
            timestamp: ts,
            beat_number: num,
            bar_number: bar,
            beat_in_bar,
            is_downbeat: beat_in_bar == 1,
            energy_at_beat: 0.5,
            is_strong_cut_point: beat_in_bar == 1 || beat_in_bar == 3,
        }
    }

    #[test]
    fn test_snap_to_beat() {
        let beats = vec![
            make_beat(0.0, 1, 1, 1),
            make_beat(0.632, 2, 1, 2),
            make_beat(1.263, 3, 1, 3),
        ];
        let snapped = RecapAssemblyEngine::snap_to_beat(&beats, 0.4);
        assert!((snapped - 0.632).abs() < 0.01);
    }

    #[test]
    fn test_pacing_fast_cuts_for_peak() {
        let peak = MusicSection {
            name: "Chorus".to_string(),
            start: 0.0, end: 10.0,
            energy_level: 0.9,
            suggested_content: SuggestedContent::Peak,
        };
        // At 95 BPM, beat interval ~ 0.632s, 2 beats per shot = 1.263s
        let beats: Vec<BeatMarker> = (0..80).map(|i| {
            make_beat(i as f64 * 0.632, i + 1, i / 4 + 1, i % 4 + 1)
        }).collect();

        let cuts = RecapAssemblyEngine::generate_section_cuts(&peak, &beats, 0.632);
        // Should produce ~8 cuts for a 10s peak section at 2 beats/shot
        assert!(cuts.len() >= 6, "Peak section should have many fast cuts, got {}", cuts.len());
    }

    #[test]
    fn test_pacing_long_shots_for_establishing() {
        let intro = MusicSection {
            name: "Intro".to_string(),
            start: 0.0, end: 10.0,
            energy_level: 0.4,
            suggested_content: SuggestedContent::Establishing,
        };
        let beats: Vec<BeatMarker> = (0..80).map(|i| {
            make_beat(i as f64 * 0.632, i + 1, i / 4 + 1, i % 4 + 1)
        }).collect();

        let cuts = RecapAssemblyEngine::generate_section_cuts(&intro, &beats, 0.632);
        // 8 beats per shot = ~5s, so ~2 cuts for 10s
        assert!(cuts.len() <= 3, "Intro should have few long shots, got {}", cuts.len());
    }

    #[test]
    fn test_default_duration_59s() {
        assert_eq!(DEFAULT_RECAP_DURATION, 59.0);
    }

    #[test]
    fn test_xml_has_no_nat_audio() {
        let result = RecapAssemblyResult {
            id: "test".to_string(), name: "Test".to_string(),
            duration: 10.0, width: 1920, height: 1080, fps: 29.97, bpm: 120.0,
            placements: vec![], music_path: "/music.mp3".to_string(),
            music_window: None,
            xml_path: "/out.xml".to_string(), clips_used: 0,
            clips_available: 0, beat_locked_cuts: 0, processing_time_ms: 0,
        };
        let beat_grid = BeatGridResult {
            audio_path: "/music.mp3".to_string(), duration: 10.0,
            bpm: 120.0, beat_interval: 0.5, total_beats: 20, beats_per_bar: 4,
            beats: vec![], sections: vec![], energy_curve: vec![],
            transition_markers: vec![], processing_time_ms: 0,
        };
        let xml = RecapAssemblyEngine::generate_premiere_xml(&result, &[], &beat_grid);
        assert!(xml.contains("numOutputChannels"));
        assert!(!xml.contains("NAT"));
    }
}
