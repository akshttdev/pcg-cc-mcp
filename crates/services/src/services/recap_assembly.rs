//! Recap Assembly Engine v3
//!
//! Three-layer editing intelligence:
//!   Layer 1 — Smart Music Window: selects best N-second arc from the full track
//!   Layer 2 — Pacing Engine: dynamic shot durations driven by beat grid + section energy + narrative arc
//!   Layer 3 — Transitions: dissolves at section boundaries, hard cuts on beats, whip dissolves, fades
//!
//! Invariants:
//!   - Default duration is 59 seconds (industry standard recap length)
//!   - NAT audio is always muted — music only on A1
//!   - All cuts snap to the beat grid
//!   - More clips used with shorter durations in high-energy sections
//!   - Progressive acceleration within sections
//!   - No clip used more than 2x in a 59s edit
//!   - Source folder diversity maximized

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    beat_analysis::{BeatGridResult, BeatMarker, MusicSection, SuggestedContent},
    scene_analysis::{ClipAnalysis, ContentType, SceneAnalysisResult},
};

/// Default recap duration in seconds
const DEFAULT_RECAP_DURATION: f64 = 59.0;

// ─── Data types ──────────────────────────────────────────────────────────────

/// Transition between two clips on the timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditTransition {
    /// Straight cut on a beat
    HardCut,
    /// Cross dissolve (0.3-0.6s variable at section boundaries)
    Dissolve { duration: f64 },
    /// Dip to black (0.3-0.5s for major section changes or finality)
    DipToBlack { duration: f64 },
    /// Fast wipe dissolve (0.15-0.25s for energy and variety)
    WhipDissolve { duration: f64 },
    /// Bright flash transition (additive mix)
    AdditiveMix { duration: f64 },
    /// Directional wipe right — pair to WhipDissolve
    WhipRight { duration: f64 },
    /// Smooth lateral slide left
    SlideLeft { duration: f64 },
    /// Smooth lateral slide right
    SlideRight { duration: f64 },
    /// Dramatic circle open reveal
    CircleOpen { duration: f64 },
    /// Dramatic circle close
    CircleClose { duration: f64 },
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

/// Scale a base transition duration by the beat interval.
/// Faster BPMs (shorter interval) shrink transitions; slower BPMs stretch them.
fn bpm_scaled_duration(base: f64, beat_interval: f64) -> f64 {
    let scale = (beat_interval / 0.5).clamp(0.6, 1.5);
    (base * scale).clamp(0.1, 0.8)
}

// ─── Layer 1: Smart Music Window ─────────────────────────────────────────────

impl RecapAssemblyEngine {
    /// Select the best N-second window from the full track.
    /// Searches at half-bar granularity for best narrative arc.
    pub fn select_music_window(beat_grid: &BeatGridResult, target_duration: f64) -> MusicWindow {
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

        // Score every possible starting position for the best narrative arc
        let beat_interval = beat_grid.beat_interval;
        let bar_duration = beat_interval * beat_grid.beats_per_bar as f64;
        // Search at half-bar granularity (doubles candidate pool vs full bar)
        let half_bar = bar_duration / 2.0;

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
            t += half_bar;
        }

        let window_end = (best_start + snapped_dur).min(track_dur);

        // Collect sections that fall within the window
        let sections: Vec<MusicSection> = beat_grid
            .sections
            .iter()
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
                        energy_direction: s.energy_direction.clone(),
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
    /// - Starting low energy and reaching a peak (narrative arc)
    /// - Having variety in energy levels (dynamic range)
    /// - Starting and ending on section boundaries
    /// - Peak prominence (distinct climax)
    ///
    /// Penalizes flat-energy windows.
    fn score_music_window(beat_grid: &BeatGridResult, start: f64, end: f64) -> f64 {
        let mut score = 0.0;

        let energies: Vec<f64> = beat_grid
            .energy_curve
            .iter()
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
            let mid_q: f64 =
                energies[quarter..quarter * 3].iter().sum::<f64>() / (quarter * 2) as f64;
            let last_q: f64 =
                energies[quarter * 3..].iter().sum::<f64>() / (energies.len() - quarter * 3) as f64;

            if mid_q > first_q {
                score += (mid_q - first_q) * 3.0;
            }
            if mid_q > last_q {
                score += (mid_q - last_q) * 2.0;
            }
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

        // 5. Flat-energy penalty: if stddev < 0.05, subtract 1.0
        let mean_e = energies.iter().sum::<f64>() / energies.len() as f64;
        let variance =
            energies.iter().map(|e| (e - mean_e).powi(2)).sum::<f64>() / energies.len() as f64;
        let stddev = variance.sqrt();
        if stddev < 0.05 {
            score -= 1.0;
        }

        // 6. Peak prominence bonus: if max energy > 1.5x window average
        if max_e > mean_e * 1.5 {
            score += 1.0;
        }

        score
    }

    // ─── Layer 2: Pacing Engine ──────────────────────────────────────────────

    /// Calculate dynamic beats per shot based on section type, position within section,
    /// narrative position within the entire edit, and section energy.
    fn calculate_beats_per_shot(
        section: &MusicSection,
        shot_position: f64,      // 0.0-1.0 within section
        narrative_position: f64, // 0.0-1.0 within entire edit
        beat_interval: f64,
    ) -> u32 {
        // Tighter base counts
        let base: f64 = match section.suggested_content {
            SuggestedContent::Establishing => 6.0, // was 8
            SuggestedContent::Building => 4.0,
            SuggestedContent::Peak => 2.0,
            SuggestedContent::HeroMoment => 3.0, // was 4
            SuggestedContent::Resolution => 5.0, // was 6
            SuggestedContent::FlashCut => 1.0,
        };

        let mut beats = base;

        // Progressive acceleration within section:
        // First shot +2 beats, last shot -1 beat (linear interpolation)
        let position_adjust = 2.0 - 3.0 * shot_position;
        beats += position_adjust;

        // Narrative arc modulation
        if narrative_position < 0.15 {
            beats += 2.0; // Let audience breathe
        } else if (0.40..=0.70).contains(&narrative_position) {
            beats -= 1.0; // Tightest pacing in peak zone
        } else if narrative_position >= 0.85 {
            beats += 2.0; // Emotional slowdown
        }

        // Energy override
        if section.energy_level > 0.8 {
            beats = beats.min(3.0);
        } else if section.energy_level < 0.3 {
            beats = beats.max(4.0);
        }

        // Clamp to 1-8 beats
        let beats_clamped = (beats.round() as i32).clamp(1, 8) as u32;

        // Also clamp duration to 0.5s-5.0s
        let dur = beats_clamped as f64 * beat_interval;
        if dur < 0.5 {
            ((0.5 / beat_interval).ceil() as u32).max(1)
        } else if dur > 5.0 {
            ((5.0 / beat_interval).floor() as u32).max(1)
        } else {
            beats_clamped
        }
    }

    /// Generate cut points for a section with dynamic pacing based on narrative position.
    /// Returns (timeline_in, timeline_out) pairs all snapped to the beat grid.
    fn generate_section_cuts(
        section: &MusicSection,
        beats: &[BeatMarker],
        beat_interval: f64,
        total_duration: f64,
    ) -> Vec<(f64, f64)> {
        let section_dur = section.end - section.start;

        // Estimate average shot duration for loop control
        let mid_narrative_pos = if total_duration > 0.0 {
            ((section.start + section.end) / 2.0) / total_duration
        } else {
            0.5
        };
        let avg_beats =
            Self::calculate_beats_per_shot(section, 0.5, mid_narrative_pos, beat_interval);
        let avg_shot_dur = beat_interval * avg_beats as f64;

        if section_dur < avg_shot_dur * 0.5 {
            return vec![(section.start, section.end)];
        }

        let estimated_shots = (section_dur / avg_shot_dur).ceil() as usize;

        let mut cuts = Vec::new();
        let mut cursor = section.start;
        let mut shot_idx = 0;

        while cursor + avg_shot_dur * 0.3 < section.end {
            let shot_position = if estimated_shots > 1 {
                (shot_idx as f64 / (estimated_shots - 1) as f64).min(1.0)
            } else {
                0.5
            };

            let narrative_position = if total_duration > 0.0 {
                cursor / total_duration
            } else {
                0.5
            };

            let bps = Self::calculate_beats_per_shot(
                section,
                shot_position,
                narrative_position,
                beat_interval,
            );
            let shot_duration = beat_interval * bps as f64;

            let cut_end = (cursor + shot_duration).min(section.end);
            let snapped_start = Self::snap_to_beat(beats, cursor);
            let snapped_end = Self::snap_to_beat(beats, cut_end);

            if snapped_end > snapped_start + 0.3 {
                cuts.push((snapped_start, snapped_end));
            }
            cursor = cut_end;
            shot_idx += 1;
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

    /// Assign a transition type for entering this shot.
    /// Uses BPM-scaled durations and directional variety with new transition types.
    fn pick_transition(
        section_idx: usize,
        shot_idx_in_section: usize,
        section: &MusicSection,
        prev_section: Option<&MusicSection>,
        is_last_shot: bool,
        beat_interval: f64,
        global_shot_idx: usize,
    ) -> EditTransition {
        // First clip of the edit — no transition
        if section_idx == 0 && shot_idx_in_section == 0 {
            return EditTransition::HardCut;
        }

        // Last transition — dip to black for finality
        if is_last_shot {
            return EditTransition::DipToBlack {
                duration: bpm_scaled_duration(0.5, beat_interval),
            };
        }

        // First clip of a new section — section boundary transitions
        if shot_idx_in_section == 0 {
            if let Some(prev) = prev_section {
                let energy_delta = section.energy_level - prev.energy_level;

                // Rising into Peak — alternate WhipDissolve / WhipRight
                if energy_delta > 0.0 && matches!(section.suggested_content, SuggestedContent::Peak)
                {
                    return if section_idx % 2 == 0 {
                        EditTransition::WhipDissolve {
                            duration: bpm_scaled_duration(0.2, beat_interval),
                        }
                    } else {
                        EditTransition::WhipRight {
                            duration: bpm_scaled_duration(0.2, beat_interval),
                        }
                    };
                }

                // Large energy drop — dip to black
                if energy_delta < 0.0 && energy_delta.abs() > 0.25 {
                    return EditTransition::DipToBlack {
                        duration: bpm_scaled_duration(0.3, beat_interval),
                    };
                }

                // Falling into Resolution — dissolve
                if energy_delta < 0.0
                    && matches!(section.suggested_content, SuggestedContent::Resolution)
                {
                    return EditTransition::Dissolve {
                        duration: bpm_scaled_duration(0.6, beat_interval),
                    };
                }

                // Establishing → Building — circle open reveal
                if matches!(prev.suggested_content, SuggestedContent::Establishing)
                    && matches!(section.suggested_content, SuggestedContent::Building)
                {
                    return EditTransition::CircleOpen {
                        duration: bpm_scaled_duration(0.3, beat_interval),
                    };
                }

                return EditTransition::Dissolve {
                    duration: bpm_scaled_duration(0.4, beat_interval),
                };
            }
            return EditTransition::Dissolve {
                duration: bpm_scaled_duration(0.5, beat_interval),
            };
        }

        // Within a section — mostly hard cuts with occasional variety
        let variety_hash = (shot_idx_in_section * 7 + section_idx * 13 + global_shot_idx * 3) % 20;
        if variety_hash < 3 {
            // ~15% chance: rotating directional transitions
            let rotation = global_shot_idx % 4;
            match rotation {
                0 => EditTransition::WhipDissolve {
                    duration: bpm_scaled_duration(0.15, beat_interval),
                },
                1 => EditTransition::WhipRight {
                    duration: bpm_scaled_duration(0.15, beat_interval),
                },
                2 => EditTransition::SlideLeft {
                    duration: bpm_scaled_duration(0.15, beat_interval),
                },
                _ => EditTransition::SlideRight {
                    duration: bpm_scaled_duration(0.15, beat_interval),
                },
            }
        } else if variety_hash == 3 {
            // ~5% chance: circle open or close
            if global_shot_idx % 2 == 0 {
                EditTransition::CircleOpen {
                    duration: bpm_scaled_duration(0.25, beat_interval),
                }
            } else {
                EditTransition::CircleClose {
                    duration: bpm_scaled_duration(0.25, beat_interval),
                }
            }
        } else {
            EditTransition::HardCut
        }
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
        let window_beats: Vec<BeatMarker> = beat_grid
            .beats
            .iter()
            .filter(|b| b.timestamp >= window.start - 0.01 && b.timestamp <= window.end + 0.01)
            .map(|b| BeatMarker {
                timestamp: b.timestamp - window.start,
                ..b.clone()
            })
            .collect();

        // Get usable clips
        let clips: Vec<&ClipAnalysis> = scene_analysis
            .clips
            .iter()
            .filter(|c| c.usable && c.duration >= 1.5)
            .collect();

        if clips.is_empty() {
            return (vec![], window);
        }

        // Collect unique source folders for diversity scoring
        let total_folders = clips
            .iter()
            .map(|c| &c.source_folder)
            .collect::<std::collections::HashSet<_>>()
            .len();

        // Layer 2: Generate cut points per section with dynamic pacing
        let mut all_slots: Vec<(f64, f64, usize)> = Vec::new(); // (start, end, section_idx)
        for (si, section) in window.sections.iter().enumerate() {
            let cuts = Self::generate_section_cuts(
                section,
                &window_beats,
                beat_grid.beat_interval,
                window.duration,
            );
            for (start, end) in cuts {
                all_slots.push((start, end, si));
            }
        }

        let total_slots = all_slots.len();

        // Assign clips to slots — multi-factor scoring with diversity
        let mut placements = Vec::new();
        let mut used_clip_counts: HashMap<String, u32> = HashMap::new();
        let mut used_source_ranges: HashMap<String, Vec<(f64, f64)>> = HashMap::new();
        let mut folder_counts: HashMap<String, u32> = HashMap::new();
        let mut last_content_type: Option<ContentType> = None;
        let mut recent_content_types: Vec<ContentType> = Vec::new();

        let mut prev_section_idx: Option<usize> = None;
        let mut shot_idx_in_section = 0;

        for (slot_idx, (slot_start, slot_end, section_idx)) in all_slots.iter().enumerate() {
            let section = &window.sections[*section_idx];
            let slot_dur = slot_end - slot_start;

            // Reset shot counter for new sections
            if Some(*section_idx) != prev_section_idx {
                shot_idx_in_section = 0;
            }

            // Build recent_content_types window (last 2)
            let recent_types_window: Vec<ContentType> =
                recent_content_types.iter().rev().take(2).cloned().collect();

            // Find best clip for this slot (multi-factor scoring)
            let best_clip = Self::pick_clip_for_slot(
                &clips,
                section,
                slot_dur,
                &used_clip_counts,
                last_content_type.as_ref(),
                &folder_counts,
                total_folders,
                &recent_types_window,
                slot_idx,
                total_slots,
            );

            if let Some(clip) = best_clip {
                // Calculate source range with section-aware extraction
                let prev_ranges = used_source_ranges
                    .get(&clip.filename)
                    .cloned()
                    .unwrap_or_default();
                let (src_in, src_out) =
                    Self::varied_source_range(clip, section, slot_dur, &prev_ranges);

                // Layer 3: Transition (with last-shot awareness)
                let is_last_shot = slot_idx == total_slots - 1;
                let prev_sec = prev_section_idx.map(|i| &window.sections[i]);
                let transition = Self::pick_transition(
                    *section_idx,
                    shot_idx_in_section,
                    section,
                    prev_sec,
                    is_last_shot,
                    beat_grid.beat_interval,
                    slot_idx,
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
                *folder_counts.entry(clip.source_folder.clone()).or_insert(0) += 1;
                used_source_ranges
                    .entry(clip.filename.clone())
                    .or_default()
                    .push((src_in, src_out));
                last_content_type = Some(clip.dominant_content_type.clone());
                recent_content_types.push(clip.dominant_content_type.clone());
            }

            shot_idx_in_section += 1;
            prev_section_idx = Some(*section_idx);
        }

        // ── Enhancement 5: Speed ramps ──────────────────────────────────────
        // Find peak hero clip (highest energy in any Peak section) → speed 0.6
        // Find final resolution clip (last placement) → speed 0.75
        if !placements.is_empty() {
            // Peak hero: highest energy_match_score clip in a Peak section
            let peak_hero_idx = placements
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    window.sections.iter().any(|s| {
                        s.name == p.section_name
                            && matches!(s.suggested_content, SuggestedContent::Peak)
                    })
                })
                .max_by(|(_, a), (_, b)| {
                    a.energy_match_score
                        .partial_cmp(&b.energy_match_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i);

            if let Some(idx) = peak_hero_idx {
                let p = &mut placements[idx];
                p.speed = 0.6;
                // Adjust source range: slow-mo needs less source material
                let timeline_dur = p.timeline_out - p.timeline_in;
                let source_needed = timeline_dur * p.speed;
                p.source_out = (p.source_in + source_needed).min(
                    clips
                        .iter()
                        .find(|c| c.filename == p.clip_filename)
                        .map(|c| c.duration)
                        .unwrap_or(p.source_out),
                );
            }

            // Final clip: gentle slow-mo
            let last_idx = placements.len() - 1;
            // Only apply if not already the peak hero clip
            if Some(last_idx) != peak_hero_idx {
                let p = &mut placements[last_idx];
                p.speed = 0.75;
                let timeline_dur = p.timeline_out - p.timeline_in;
                let source_needed = timeline_dur * p.speed;
                p.source_out = (p.source_in + source_needed).min(
                    clips
                        .iter()
                        .find(|c| c.filename == p.clip_filename)
                        .map(|c| c.duration)
                        .unwrap_or(p.source_out),
                );
            }
        }

        // ── Enhancement 6: Within-section energy + thematic ordering ────────
        // Reorder clip assignments within each section based on section type.
        {
            // Group placement indices by section
            let mut section_groups: Vec<(usize, Vec<usize>)> = Vec::new();
            let mut current_section_name = String::new();
            for (i, p) in placements.iter().enumerate() {
                if p.section_name != current_section_name {
                    section_groups.push((i, vec![i]));
                    current_section_name = p.section_name.clone();
                } else if let Some(last) = section_groups.last_mut() {
                    last.1.push(i);
                }
            }

            for (_start, indices) in &section_groups {
                if indices.len() < 2 {
                    continue;
                }

                // Determine section type from the first placement's section name
                let section_type = window
                    .sections
                    .iter()
                    .find(|s| s.name == placements[indices[0]].section_name)
                    .map(|s| &s.suggested_content);

                // Collect (index, energy_match_score, source_folder) for sorting
                let mut slot_data: Vec<(usize, f64, String)> = indices
                    .iter()
                    .map(|&i| {
                        let folder = clips
                            .iter()
                            .find(|c| c.filename == placements[i].clip_filename)
                            .map(|c| c.source_folder.clone())
                            .unwrap_or_default();
                        (i, placements[i].energy_match_score, folder)
                    })
                    .collect();

                match section_type {
                    Some(SuggestedContent::Building) => {
                        // Sort by ascending energy (build momentum)
                        slot_data.sort_by(|a, b| {
                            a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                        });
                    }
                    Some(SuggestedContent::Resolution) => {
                        // Sort by descending energy (wind down)
                        slot_data.sort_by(|a, b| {
                            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                        });
                    }
                    Some(SuggestedContent::Peak) => {
                        // Alternate high/low energy + different folders
                        slot_data.sort_by(|a, b| {
                            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        let mut reordered = Vec::with_capacity(slot_data.len());
                        let mut high = 0;
                        let mut low = slot_data.len() - 1;
                        let mut take_high = true;
                        while high <= low {
                            if take_high {
                                reordered.push(slot_data[high].clone());
                                high += 1;
                            } else {
                                reordered.push(slot_data[low].clone());
                                if low == 0 {
                                    break;
                                }
                                low -= 1;
                            }
                            take_high = !take_high;
                        }
                        slot_data = reordered;
                    }
                    _ => continue, // Establishing/HeroMoment: no reorder
                }

                // Apply reorder: swap clip data between placements while keeping timeline positions
                let clip_data: Vec<(String, PathBuf, f64, f64, f64, f64)> = slot_data
                    .iter()
                    .map(|(orig_idx, _, _)| {
                        let p = &placements[*orig_idx];
                        (
                            p.clip_filename.clone(),
                            p.source_path.clone(),
                            p.source_in,
                            p.source_out,
                            p.energy_match_score,
                            p.speed,
                        )
                    })
                    .collect();

                for (pos, &idx) in indices.iter().enumerate() {
                    if pos < clip_data.len() {
                        let (ref name, ref path, src_in, src_out, energy, speed) = clip_data[pos];
                        placements[idx].clip_filename = name.clone();
                        placements[idx].source_path = path.clone();
                        placements[idx].source_in = src_in;
                        placements[idx].source_out = src_out;
                        placements[idx].energy_match_score = energy;
                        placements[idx].speed = speed;
                    }
                }
            }
        }

        (placements, window)
    }

    /// Pick the best clip for a timeline slot using multi-factor scoring.
    ///
    /// Factors: quality score, energy quartile fitness, source folder diversity,
    /// duration fitness, hard reuse cap (max 2x), content type diversity.
    fn pick_clip_for_slot<'a>(
        clips: &[&'a ClipAnalysis],
        section: &MusicSection,
        slot_duration: f64,
        used_counts: &HashMap<String, u32>,
        last_content_type: Option<&ContentType>,
        folder_counts: &HashMap<String, u32>,
        total_folders: usize,
        recent_content_types: &[ContentType],
        slot_index: usize,
        total_slots: usize,
    ) -> Option<&'a ClipAnalysis> {
        let mut candidates: Vec<(&ClipAnalysis, f64)> = clips
            .iter()
            .map(|clip| {
                let mut score = Self::clip_section_match_score(clip, section);

                // Quality score factor (0 to +0.3, 1.5x multiplier in Peak sections)
                let quality_bonus = clip.quality_score * 0.3;
                let quality_multiplier =
                    if matches!(section.suggested_content, SuggestedContent::Peak) {
                        1.5
                    } else {
                        1.0
                    };
                score += quality_bonus * quality_multiplier;

                // Energy quartile fitness (0 to +0.4)
                let quartile_bonus = match (clip.energy_quartile, &section.suggested_content) {
                    (4, SuggestedContent::Peak) | (4, SuggestedContent::FlashCut) => 0.4,
                    (3, SuggestedContent::Peak) | (3, SuggestedContent::HeroMoment) => 0.3,
                    (1, SuggestedContent::Establishing) | (2, SuggestedContent::Establishing) => {
                        0.3
                    }
                    (1, SuggestedContent::Resolution) | (2, SuggestedContent::Resolution) => 0.25,
                    (2, SuggestedContent::Building) | (3, SuggestedContent::Building) => 0.2,
                    _ => 0.0,
                };
                score += quartile_bonus;

                // Source folder diversity (-0.5 to +0.5)
                if total_folders > 0 {
                    let folder_use_count =
                        folder_counts.get(&clip.source_folder).copied().unwrap_or(0);
                    let total_placed: f64 = used_counts.values().sum::<u32>() as f64 + 1.0;
                    let expected_per_folder = total_placed / total_folders as f64;

                    if folder_use_count == 0 {
                        score += 0.5; // Underrepresented folder
                    } else if (folder_use_count as f64) > expected_per_folder * 1.5 {
                        score -= 0.5; // Overrepresented folder
                    } else if (folder_use_count as f64) < expected_per_folder * 0.5 {
                        score += 0.3;
                    }
                }

                // Duration fitness (-0.5 to +0.2)
                if clip.duration >= slot_duration * 2.0 {
                    score += 0.2; // Long clips have more material
                } else if clip.duration < slot_duration {
                    score -= 0.5; // Too short
                }

                // Hard reuse cap: no clip used more than 2x
                let uses = used_counts.get(&clip.filename).copied().unwrap_or(0);
                if uses >= 2 {
                    score -= 999.0;
                } else {
                    // Penalty for reuse
                    score -= uses as f64 * 1.5;
                }

                // Diversity — penalize consecutive same-type clips for visual variety
                if let Some(last_type) = last_content_type
                    && clip.dominant_content_type == *last_type
                {
                    score -= 0.3;
                }

                // Hero moment bonus — clips whose peak energy aligns with Peak/HeroMoment
                if matches!(
                    section.suggested_content,
                    SuggestedContent::Peak | SuggestedContent::HeroMoment
                ) {
                    let peak_ts = clip.peak_energy_timestamp;
                    if peak_ts >= 0.0 && peak_ts <= clip.duration {
                        score += 0.25;
                    }
                }

                // Enhancement 3: Triple-repeat penalty — avoid 3 consecutive same content type
                if recent_content_types.len() >= 2
                    && recent_content_types
                        .iter()
                        .all(|t| *t == clip.dominant_content_type)
                {
                    score -= 0.6;
                }

                // Enhancement 4: Positional curation — opener and closer bonuses
                if slot_index == 0 {
                    // Opener: prefer Establishing, high quality, low motion
                    if matches!(clip.dominant_content_type, ContentType::Establishing) {
                        score += 0.4;
                    }
                    if clip.quality_score > 0.7 {
                        score += 0.3;
                    }
                    if clip.overall_energy < 0.3 {
                        score += 0.2;
                    }
                } else if slot_index == total_slots - 1 {
                    // Closer: prefer Resolution/Establishing type, high quality, calm
                    if matches!(
                        clip.dominant_content_type,
                        ContentType::Ambient | ContentType::Establishing
                    ) {
                        score += 0.3;
                    }
                    if clip.quality_score > 0.7 {
                        score += 0.3;
                    }
                    if clip.overall_energy < 0.4 {
                        score += 0.2;
                    }
                }

                (*clip, score)
            })
            .collect();

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.first().map(|(clip, _)| *clip)
    }

    /// Calculate source in/out with section-aware extraction strategy.
    ///
    /// - Peak/HeroMoment: center on clip's peak_energy_timestamp
    /// - Establishing: use segment with LOWEST motion (steadiest shot)
    /// - Building: use segment where energy is RISING (visual momentum)
    /// - Resolution: use segment near end of clip or with falling energy
    /// - Never start within 0.3s of a detected hard cut
    fn varied_source_range(
        clip: &ClipAnalysis,
        section: &MusicSection,
        target_dur: f64,
        prev_ranges: &[(f64, f64)],
    ) -> (f64, f64) {
        let clamp_range = |start: f64| -> (f64, f64) {
            let s = start.max(0.0);
            let e = (s + target_dur).min(clip.duration);
            (s, e)
        };

        let avoid_hard_cuts = |start: f64| -> f64 {
            for &cut_ts in &clip.hard_cuts {
                if (start - cut_ts).abs() < 0.3 {
                    return (cut_ts + 0.3).min((clip.duration - target_dur).max(0.0));
                }
            }
            start
        };

        let not_overlapping =
            |s: f64, e: f64| -> bool { !prev_ranges.iter().any(|(ps, pe)| s < *pe && e > *ps) };

        // Section-type-specific source range selection
        match section.suggested_content {
            SuggestedContent::Peak | SuggestedContent::HeroMoment => {
                // Use clip's peak_energy_timestamp as center
                let peak_start = (clip.peak_energy_timestamp - target_dur / 2.0).max(0.0);
                let peak_start = avoid_hard_cuts(peak_start);
                let (s, e) = clamp_range(peak_start);
                if not_overlapping(s, e) {
                    return (s, e);
                }
            }
            SuggestedContent::Establishing => {
                // Use segment with LOWEST motion (steadiest shot)
                if let Some(best) = clip.segments.iter().min_by(|a, b| {
                    a.motion_intensity
                        .partial_cmp(&b.motion_intensity)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }) {
                    let start = avoid_hard_cuts(best.timestamp);
                    let (s, e) = clamp_range(start);
                    if not_overlapping(s, e) {
                        return (s, e);
                    }
                }
            }
            SuggestedContent::Building => {
                // Use segment where energy is RISING (visual momentum)
                for i in 0..clip.segments.len().saturating_sub(1) {
                    if clip.segments[i + 1].energy_score > clip.segments[i].energy_score {
                        let start = avoid_hard_cuts(clip.segments[i].timestamp);
                        let (s, e) = clamp_range(start);
                        if not_overlapping(s, e) {
                            return (s, e);
                        }
                    }
                }
            }
            SuggestedContent::Resolution => {
                // Use segment near end of clip or with falling energy
                let start = ((clip.duration * 0.66) - target_dur / 2.0).max(0.0);
                let start = avoid_hard_cuts(start);
                let (s, e) = clamp_range(start);
                if not_overlapping(s, e) {
                    return (s, e);
                }
            }
            _ => {}
        }

        // Fallback: rank all segments by energy match
        let mut candidates: Vec<(f64, f64)> = clip
            .segments
            .iter()
            .map(|s| {
                let energy_diff = (s.energy_score - section.energy_level).abs();
                (s.timestamp, 1.0 - energy_diff)
            })
            .collect();

        // Sort by score descending
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Try each candidate, skip ones that overlap with previously used ranges
        for (ts, _score) in &candidates {
            let start = avoid_hard_cuts(*ts);
            let available = clip.duration - start;
            let actual_start = if available >= target_dur {
                start
            } else {
                (clip.duration - target_dur).max(0.0)
            };
            let end = (actual_start + target_dur).min(clip.duration);

            if not_overlapping(actual_start, end) {
                return (actual_start, end);
            }
        }

        // All segments overlap — pick the one furthest from any previous use
        let best_start = if clip.duration >= target_dur * 2.0 {
            let mut starts: Vec<f64> = prev_ranges.iter().map(|(s, _)| *s).collect();
            starts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let options = [
                0.0,
                (clip.duration - target_dur).max(0.0),
                clip.duration / 3.0,
                clip.duration * 2.0 / 3.0,
            ];

            options
                .iter()
                .max_by(|a, b| {
                    let dist_a: f64 = starts.iter().map(|s| (*a - s).abs()).sum();
                    let dist_b: f64 = starts.iter().map(|s| (*b - s).abs()).sum();
                    dist_a
                        .partial_cmp(&dist_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .copied()
                .unwrap_or(0.0)
        } else {
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
            (ContentType::HighEnergy, SuggestedContent::FlashCut) => 0.4,
            (ContentType::Ambient, SuggestedContent::Resolution) => 0.3,
            (ContentType::HighEnergy, SuggestedContent::HeroMoment) => 0.3,
            (ContentType::Transition, SuggestedContent::FlashCut) => 0.3,
            (ContentType::HighEnergy, SuggestedContent::Building) => 0.25,
            (ContentType::Transition, SuggestedContent::Building) => 0.2,
            (ContentType::Transition, SuggestedContent::Establishing) => 0.2,
            (ContentType::Intimate, SuggestedContent::Resolution) => 0.2,
            (ContentType::Ambient, SuggestedContent::Establishing) => 0.15,
            _ => 0.0,
        };
        score += content_bonus;

        score
    }

    fn energy_match_score(clip: &ClipAnalysis, section: &MusicSection) -> f64 {
        1.0 - (clip.overall_energy - section.energy_level).abs()
    }

    fn snap_to_beat(beats: &[BeatMarker], time: f64) -> f64 {
        beats
            .iter()
            .min_by(|a, b| {
                (a.timestamp - time)
                    .abs()
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
        for p in placements.iter() {
            inputs.push(format!("  -i \"{}\" \\", p.source_path.to_string_lossy()));
        }
        // Music input is the last one
        let music_idx = n;
        inputs.push(format!("  -i \"{music_path}\" \\"));

        // Build filter_complex
        let mut filters = Vec::new();
        let mut segment_labels = Vec::new();

        // Step 1: Trim + scale each clip (with speed ramp support)
        for (i, p) in placements.iter().enumerate() {
            let label = format!("v{i}");
            if (p.speed - 1.0).abs() > 0.01 {
                // Speed ramp: setpts divisor stretches/compresses time
                filters.push(format!(
                    "[{}:v]trim=start={:.3}:end={:.3},setpts=(PTS-STARTPTS)/{:.2},settb=AVTB,scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2[{}]",
                    i, p.source_in, p.source_out, p.speed, label
                ));
            } else {
                filters.push(format!(
                    "[{}:v]trim=start={:.3}:end={:.3},setpts=PTS-STARTPTS,settb=AVTB,scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2[{}]",
                    i, p.source_in, p.source_out, label
                ));
            }
            segment_labels.push(label);
        }

        // Step 2: Chain transitions
        // Group consecutive hard-cut clips, concat them into sections,
        // then xfade between sections.
        let mut sections: Vec<Vec<usize>> = Vec::new();
        let mut current_section: Vec<usize> = vec![0];
        let mut section_transitions: Vec<&EditTransition> = Vec::new();

        for (i, placement) in placements.iter().enumerate().skip(1) {
            match &placement.transition_in {
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
                let concat_inputs: String = group
                    .iter()
                    .map(|&i| format!("[{}]", segment_labels[i]))
                    .collect();
                let label = format!("sec{si}");
                filters.push(format!(
                    "{}concat=n={}:v=1:a=0[{}]",
                    concat_inputs,
                    group.len(),
                    label
                ));
                section_labels.push(label);
            }
        }

        // Calculate section durations for xfade offsets
        // IMPORTANT: Use source_out - source_in (actual trim duration), NOT
        // timeline_out - timeline_in (beat-grid slot duration). When clips are
        // shorter than the requested slot, clamp_range reduces the source range,
        // making source duration < timeline duration. FFmpeg trims to source
        // values, so offsets must be based on actual rendered durations.
        let mut section_durations: Vec<f64> = Vec::new();
        for group in &sections {
            let dur: f64 = group
                .iter()
                .map(|&i| {
                    let source_dur = placements[i].source_out - placements[i].source_in;
                    // When speed < 1.0 (slow-mo), rendered duration is longer
                    if placements[i].speed > 0.01 {
                        source_dur / placements[i].speed
                    } else {
                        source_dur
                    }
                })
                .sum();
            section_durations.push(dur);
        }

        // Chain xfade between section groups
        let total_video_dur;
        if section_labels.len() == 1 {
            // Only one group — add fade in/out, then fps=30000/1001 to convert
            // from AVTB to clean timebase before encoding
            total_video_dur = section_durations[0];
            filters.push(format!(
                "[{}]fade=t=in:d={:.1}:st=0,fade=t=out:d={:.1}:st={:.2},fps=30000/1001[outv]",
                section_labels[0],
                window.fade_in,
                window.fade_out,
                (total_video_dur - window.fade_out).max(0.0)
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
                    EditTransition::WhipDissolve { duration } => ("wipeleft", *duration),
                    EditTransition::AdditiveMix { duration } => ("fadewhite", *duration),
                    EditTransition::WhipRight { duration } => ("wiperight", *duration),
                    EditTransition::SlideLeft { duration } => ("slideleft", *duration),
                    EditTransition::SlideRight { duration } => ("slideright", *duration),
                    EditTransition::CircleOpen { duration } => ("circleopen", *duration),
                    EditTransition::CircleClose { duration } => ("circleclose", *duration),
                    EditTransition::HardCut => ("fade", 0.033),
                };

                let offset = (accumulated_duration - xfade_dur).max(0.0);
                let out_label = if i == section_transitions.len() - 1 {
                    "xfinal".to_string()
                } else {
                    format!("x{i}")
                };

                // All inputs are at AVTB (settb=AVTB on clips), so xfade inputs
                // always match. xfade output stays at AVTB. No intermediate fps
                // conversion needed — fps=30000/1001 is applied only on the final
                // fade line to convert AVTB to clean timebase before encoding.
                filters.push(format!(
                    "[{current_label}][{next_label}]xfade=transition={xfade_type}:duration={xfade_dur:.3}:offset={offset:.3}[{out_label}]"
                ));

                accumulated_duration += section_durations[i + 1] - xfade_dur;
                current_label = out_label;
            }

            // Add fade in/out on the final composited video, then fps=30000/1001
            // to convert from AVTB to clean timebase before encoding
            total_video_dur = accumulated_duration;
            filters.push(format!(
                "[xfinal]fade=t=in:d={:.1}:st=0,fade=t=out:d={:.1}:st={:.2},fps=30000/1001[outv]",
                window.fade_in,
                window.fade_out,
                (total_video_dur - window.fade_out).max(0.0)
            ));
        }

        // Audio: trim music to match actual video duration, fade in/out
        let audio_end = window.start + total_video_dur;
        filters.push(format!(
            "[{}:a]atrim=start={:.3}:end={:.3},asetpts=PTS-STARTPTS,afade=t=in:d={:.1},afade=t=out:d={:.1}:st={:.2}[outa]",
            music_idx,
            window.start, audio_end,
            window.fade_in,
            window.fade_out,
            (total_video_dur - window.fade_out).max(0.0)
        ));

        // Assemble the command
        script.push_str("ffmpeg -y \\\n");
        for input in &inputs {
            script.push_str(&format!("{input}\n"));
        }
        script.push_str("  -filter_complex \"\n");
        for (i, f) in filters.iter().enumerate() {
            if i < filters.len() - 1 {
                script.push_str(&format!("    {f};\n"));
            } else {
                script.push_str(&format!("    {f}\n"));
            }
        }
        script.push_str("  \" \\\n");
        script.push_str("  -map \"[outv]\" -map \"[outa]\" \\\n");
        script.push_str("  -pix_fmt yuv420p -profile:v high -level 4.0 \\\n");
        script.push_str("  -c:v libx264 -preset medium -crf 18 -r 29.97 \\\n");
        script.push_str("  -c:a aac -b:a 192k \\\n");
        script.push_str("  -movflags +faststart \\\n");
        script.push_str(&format!("  \"{output_path}\"\n"));

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
            file_ids
                .entry(key)
                .or_insert_with(|| format!("file-{}", Uuid::new_v4()));
        }
        let music_file_id = format!("file-music-{}", Uuid::new_v4());

        // Deduplicate clips in bin
        let mut seen_files = std::collections::HashSet::new();
        for p in placements {
            let key = p.source_path.to_string_lossy().to_string();
            if !seen_files.insert(key.clone()) {
                continue;
            }
            if let Some(fid) = file_ids.get(&key) {
                let file_url = format!(
                    "file:///{}",
                    p.source_path.to_string_lossy().replace(' ', "%20")
                );
                xml.push_str(&format!(
                    "          <clip id=\"clip-{}\">\n",
                    escape_xml(&p.clip_filename)
                ));
                xml.push_str(&format!(
                    "            <name>{}</name>\n",
                    escape_xml(&p.clip_filename)
                ));
                xml.push_str("            <media>\n");
                xml.push_str("              <video><track><clipitem>\n");
                xml.push_str(&format!("                <file id=\"{fid}\">\n"));
                xml.push_str(&format!(
                    "                  <name>{}</name>\n",
                    escape_xml(&p.clip_filename)
                ));
                xml.push_str(&format!(
                    "                  <pathurl>{}</pathurl>\n",
                    escape_xml(&file_url)
                ));
                xml.push_str(&format!(
                    "                  <rate><timebase>{timebase}</timebase><ntsc>TRUE</ntsc></rate>\n"
                ));
                xml.push_str("                  <media>\n");
                xml.push_str("                    <video><samplecharacteristics>\n");
                xml.push_str(&format!(
                    "                      <width>{}</width>\n",
                    result.width
                ));
                xml.push_str(&format!(
                    "                      <height>{}</height>\n",
                    result.height
                ));
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
        xml.push_str(&format!(
            "            <name>Music - {}</name>\n",
            escape_xml(&result.name)
        ));
        xml.push_str("            <media><audio><track><clipitem>\n");
        xml.push_str(&format!("              <file id=\"{music_file_id}\">\n"));
        xml.push_str(&format!(
            "                <name>{}</name>\n",
            escape_xml(
                Path::new(&result.music_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "music.mp3".to_string())
                    .as_str()
            )
        ));
        xml.push_str(&format!(
            "                <pathurl>{}</pathurl>\n",
            escape_xml(&music_url)
        ));
        xml.push_str(&format!(
            "                <rate><timebase>{timebase}</timebase><ntsc>TRUE</ntsc></rate>\n"
        ));
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
        xml.push_str(&format!(
            "        <name>{}</name>\n",
            escape_xml(&result.name)
        ));
        xml.push_str(&format!("        <duration>{total_frames}</duration>\n"));
        xml.push_str(&format!(
            "        <rate><timebase>{timebase}</timebase><ntsc>TRUE</ntsc></rate>\n"
        ));
        xml.push_str("        <timecode>\n");
        xml.push_str(&format!(
            "          <rate><timebase>{timebase}</timebase><ntsc>TRUE</ntsc></rate>\n"
        ));
        xml.push_str("          <string>01:00:00:00</string>\n");
        xml.push_str("          <frame>108000</frame>\n");
        xml.push_str("          <displayformat>NDF</displayformat>\n");
        xml.push_str("        </timecode>\n");
        xml.push_str("        <media>\n");

        // Video track
        xml.push_str("          <video>\n");
        xml.push_str("            <format><samplecharacteristics>\n");
        xml.push_str(&format!("              <width>{}</width>\n", result.width));
        xml.push_str(&format!(
            "              <height>{}</height>\n",
            result.height
        ));
        xml.push_str("              <anamorphic>FALSE</anamorphic>\n");
        xml.push_str("              <pixelaspectratio>Square</pixelaspectratio>\n");
        xml.push_str("              <fielddominance>none</fielddominance>\n");
        xml.push_str(&format!(
            "              <rate><timebase>{timebase}</timebase><ntsc>TRUE</ntsc></rate>\n"
        ));
        xml.push_str("            </samplecharacteristics></format>\n");
        xml.push_str("            <track>\n");

        for (i, p) in placements.iter().enumerate() {
            let key = p.source_path.to_string_lossy().to_string();
            let fid = file_ids.get(&key).cloned().unwrap_or_default();
            xml.push_str(&format!(
                "              <clipitem id=\"tl-v1-{:02}\">\n",
                i + 1
            ));
            xml.push_str(&format!(
                "                <name>{}</name>\n",
                escape_xml(&p.clip_filename)
            ));
            xml.push_str("                <enabled>TRUE</enabled>\n");
            xml.push_str(&format!(
                "                <rate><timebase>{timebase}</timebase><ntsc>TRUE</ntsc></rate>\n"
            ));
            xml.push_str(&format!(
                "                <start>{}</start>\n",
                secs_to_frames(p.timeline_in)
            ));
            xml.push_str(&format!(
                "                <end>{}</end>\n",
                secs_to_frames(p.timeline_out)
            ));
            xml.push_str(&format!(
                "                <in>{}</in>\n",
                secs_to_frames(p.source_in)
            ));
            xml.push_str(&format!(
                "                <out>{}</out>\n",
                secs_to_frames(p.source_out)
            ));
            xml.push_str(&format!("                <file id=\"{fid}\"/>\n"));

            // Speed/rate for non-1.0 speed clips
            if (p.speed - 1.0).abs() > 0.01 {
                xml.push_str("                <filter>\n");
                xml.push_str("                  <effect>\n");
                xml.push_str("                    <name>Time Remap</name>\n");
                xml.push_str("                    <effectid>timeremap</effectid>\n");
                xml.push_str("                    <effecttype>motion</effecttype>\n");
                xml.push_str("                    <parameter>\n");
                xml.push_str("                      <name>speed</name>\n");
                xml.push_str(&format!(
                    "                      <value>{:.1}</value>\n",
                    p.speed * 100.0
                ));
                xml.push_str("                    </parameter>\n");
                xml.push_str("                  </effect>\n");
                xml.push_str("                </filter>\n");
            }

            // Transition effect for non-HardCut transitions
            match &p.transition_in {
                EditTransition::HardCut => {}
                trans => {
                    let (effect_name, trans_dur) = match trans {
                        EditTransition::Dissolve { duration } => ("Cross Dissolve", *duration),
                        EditTransition::DipToBlack { duration } => ("Dip to Black", *duration),
                        EditTransition::WhipDissolve { duration } => ("Wipe", *duration),
                        EditTransition::AdditiveMix { duration } => {
                            ("Additive Dissolve", *duration)
                        }
                        EditTransition::WhipRight { duration } => ("Wipe", *duration),
                        EditTransition::SlideLeft { duration } => ("Slide", *duration),
                        EditTransition::SlideRight { duration } => ("Slide", *duration),
                        EditTransition::CircleOpen { duration } => ("Iris Round", *duration),
                        EditTransition::CircleClose { duration } => ("Iris Round", *duration),
                        EditTransition::HardCut => unreachable!(),
                    };
                    let trans_frames = secs_to_frames(trans_dur);
                    xml.push_str("                <transitionitem>\n");
                    xml.push_str(&format!(
                        "                  <name>{}</name>\n",
                        escape_xml(effect_name)
                    ));
                    xml.push_str(&format!(
                        "                  <duration>{trans_frames}</duration>\n"
                    ));
                    xml.push_str("                  <alignment>center</alignment>\n");
                    xml.push_str("                </transitionitem>\n");
                }
            }

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
        xml.push_str(&format!(
            "                <name>{}</name>\n",
            escape_xml(&result.name)
        ));
        xml.push_str("                <enabled>TRUE</enabled>\n");
        xml.push_str(&format!(
            "                <rate><timebase>{timebase}</timebase><ntsc>TRUE</ntsc></rate>\n"
        ));
        xml.push_str("                <start>0</start>\n");
        xml.push_str(&format!("                <end>{total_frames}</end>\n"));

        if let Some(ref win) = result.music_window {
            xml.push_str(&format!(
                "                <in>{}</in>\n",
                secs_to_frames(win.start)
            ));
            xml.push_str(&format!(
                "                <out>{}</out>\n",
                secs_to_frames(win.end)
            ));
        } else {
            xml.push_str("                <in>0</in>\n");
            xml.push_str(&format!("                <out>{total_frames}</out>\n"));
        }

        xml.push_str(&format!("                <file id=\"{music_file_id}\"/>\n"));
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
                    xml.push_str(&format!(
                        "          <name>Beat {}</name>\n",
                        beat.beat_number
                    ));
                    xml.push_str(&format!("          <in>{}</in>\n", secs_to_frames(rebased)));
                    xml.push_str(&format!(
                        "          <out>{}</out>\n",
                        secs_to_frames(rebased)
                    ));
                    xml.push_str(&format!(
                        "          <comment>Bar {} | Downbeat</comment>\n",
                        beat.bar_number
                    ));
                    xml.push_str("        </marker>\n");
                }
            }
        }

        // Section markers
        for section in result
            .music_window
            .as_ref()
            .map(|w| &w.sections)
            .unwrap_or(&beat_grid.sections)
        {
            xml.push_str("        <marker>\n");
            xml.push_str(&format!(
                "          <name>{}</name>\n",
                escape_xml(&section.name)
            ));
            xml.push_str(&format!(
                "          <in>{}</in>\n",
                secs_to_frames(section.start)
            ));
            xml.push_str(&format!(
                "          <out>{}</out>\n",
                secs_to_frames(section.end)
            ));
            xml.push_str(&format!(
                "          <comment>Energy: {:.0}%</comment>\n",
                section.energy_level * 100.0
            ));
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
    use crate::services::beat_analysis::EnergyDirection;

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
            start: 0.0,
            end: 10.0,
            energy_level: 0.9,
            suggested_content: SuggestedContent::Peak,
            energy_direction: EnergyDirection::Stable,
        };
        let beats: Vec<BeatMarker> = (0..80)
            .map(|i| make_beat(i as f64 * 0.632, i + 1, i / 4 + 1, i % 4 + 1))
            .collect();

        let cuts = RecapAssemblyEngine::generate_section_cuts(&peak, &beats, 0.632, 59.0);
        // Dynamic pacing: peak section should still have many cuts
        assert!(
            cuts.len() >= 3,
            "Peak section should have fast cuts, got {}",
            cuts.len()
        );
    }

    #[test]
    fn test_pacing_long_shots_for_establishing() {
        let intro = MusicSection {
            name: "Intro".to_string(),
            start: 0.0,
            end: 10.0,
            energy_level: 0.4,
            suggested_content: SuggestedContent::Establishing,
            energy_direction: EnergyDirection::Stable,
        };
        let beats: Vec<BeatMarker> = (0..80)
            .map(|i| make_beat(i as f64 * 0.632, i + 1, i / 4 + 1, i % 4 + 1))
            .collect();

        let cuts = RecapAssemblyEngine::generate_section_cuts(&intro, &beats, 0.632, 59.0);
        // Establishing sections should have fewer, longer shots
        assert!(
            cuts.len() <= 4,
            "Intro should have few long shots, got {}",
            cuts.len()
        );
    }

    #[test]
    fn test_dynamic_pacing_varies_within_section() {
        // Verify that shot durations actually vary (progressive acceleration)
        let section = MusicSection {
            name: "Building".to_string(),
            start: 0.0,
            end: 20.0,
            energy_level: 0.6,
            suggested_content: SuggestedContent::Building,
            energy_direction: EnergyDirection::Rising,
        };
        let beats: Vec<BeatMarker> = (0..160)
            .map(|i| make_beat(i as f64 * 0.632, i + 1, i / 4 + 1, i % 4 + 1))
            .collect();

        let cuts = RecapAssemblyEngine::generate_section_cuts(&section, &beats, 0.632, 59.0);
        assert!(
            cuts.len() >= 3,
            "Should have multiple cuts to verify variation"
        );

        // First shot should be longer than later shots (progressive acceleration)
        if cuts.len() >= 2 {
            let first_dur = cuts[0].1 - cuts[0].0;
            let last_dur = cuts[cuts.len() - 1].1 - cuts[cuts.len() - 1].0;
            // First shot gets +2 beats, last gets -1, so first should tend longer
            // (allowing some tolerance for snapping)
            assert!(
                first_dur >= last_dur * 0.5,
                "First shot ({:.2}s) should not be much shorter than last ({:.2}s)",
                first_dur,
                last_dur
            );
        }
    }

    #[test]
    fn test_default_duration_59s() {
        assert_eq!(DEFAULT_RECAP_DURATION, 59.0);
    }

    #[test]
    fn test_xml_has_no_nat_audio() {
        let result = RecapAssemblyResult {
            id: "test".to_string(),
            name: "Test".to_string(),
            duration: 10.0,
            width: 1920,
            height: 1080,
            fps: 29.97,
            bpm: 120.0,
            placements: vec![],
            music_path: "/music.mp3".to_string(),
            music_window: None,
            xml_path: "/out.xml".to_string(),
            clips_used: 0,
            clips_available: 0,
            beat_locked_cuts: 0,
            processing_time_ms: 0,
        };
        let beat_grid = BeatGridResult {
            audio_path: "/music.mp3".to_string(),
            duration: 10.0,
            bpm: 120.0,
            beat_interval: 0.5,
            total_beats: 20,
            beats_per_bar: 4,
            beats: vec![],
            sections: vec![],
            energy_curve: vec![],
            transition_markers: vec![],
            processing_time_ms: 0,
        };
        let xml = RecapAssemblyEngine::generate_premiere_xml(&result, &[], &beat_grid);
        assert!(xml.contains("numOutputChannels"));
        assert!(!xml.contains("NAT"));
    }

    #[test]
    fn test_transition_types() {
        // Verify WhipDissolve and AdditiveMix serialize correctly
        let whip = EditTransition::WhipDissolve { duration: 0.2 };
        let additive = EditTransition::AdditiveMix { duration: 0.3 };
        let json_w = serde_json::to_string(&whip).unwrap();
        let json_a = serde_json::to_string(&additive).unwrap();
        assert!(json_w.contains("whip_dissolve"));
        assert!(json_a.contains("additive_mix"));
    }
}
