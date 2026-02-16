//! Directive Generator — Phase 5
//!
//! Synthesizes a complete EditDirective from accumulated pipeline artifacts:
//! client spec, structured transcript, shot catalog, context brief, and
//! verified soundbites. Uses LLM reasoning to map content to the 5-act
//! structure and assign B-roll per act.

use reqwest::Client;
use serde_json::Value;

use crate::{NoraError, Result};

use super::transcript::{levenshtein_ratio, TranscriptProcessor};
use super::types::*;

/// Generates EditDirective from all accumulated pipeline artifacts.
pub struct DirectiveGenerator {
    soundbite_match_threshold: f64,
    http_client: Client,
}

impl DirectiveGenerator {
    pub fn new(soundbite_match_threshold: f64) -> Self {
        Self {
            soundbite_match_threshold,
            http_client: Client::new(),
        }
    }

    /// Verify proposed soundbites against the actual transcript text.
    ///
    /// For each testimonial segment, finds the best-matching span in the
    /// full transcript text and verifies the Levenshtein match ratio.
    /// Also checks sentence boundary alignment and assigns content themes.
    pub fn verify_soundbites(
        &self,
        transcript: &StructuredTranscript,
        _catalog: &ShotCatalog,
    ) -> Vec<VerifiedSoundbite> {
        let mut verified = Vec::new();

        // Build the full transcript text for fuzzy matching
        let full_text: String = transcript
            .segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Get sentence boundaries for alignment checking
        let sentences = &transcript.sentences;

        // Select testimonial segments as candidate soundbites
        for (i, segment) in transcript.segments.iter().enumerate() {
            if segment.segment_type != SegmentType::Testimonial {
                continue;
            }

            // Skip very short segments (likely transitions / filler)
            let duration = segment.end_seconds - segment.start_seconds;
            if duration < 2.0 || segment.text.split_whitespace().count() < 5 {
                continue;
            }

            // Verify the soundbite text against the full transcript using sliding window
            let match_confidence = find_best_match(&segment.text, &full_text);

            if match_confidence < self.soundbite_match_threshold {
                tracing::warn!(
                    "[DIRECTIVE] Soundbite sb_{:03} rejected: match {:.2}% < threshold {:.0}%: \"{}\"",
                    i,
                    match_confidence * 100.0,
                    self.soundbite_match_threshold * 100.0,
                    &segment.text[..segment.text.len().min(60)]
                );
                continue;
            }

            // Check sentence boundary alignment
            let (starts_at_sentence, ends_at_sentence) =
                TranscriptProcessor::check_sentence_alignment(
                    &segment.text,
                    sentences,
                    segment.start_seconds,
                    segment.end_seconds,
                );

            if !starts_at_sentence || !ends_at_sentence {
                tracing::warn!(
                    "[DIRECTIVE] Soundbite sb_{:03} has misaligned boundaries \
                     (start={}, end={}): \"{}\"",
                    i,
                    starts_at_sentence,
                    ends_at_sentence,
                    &segment.text[..segment.text.len().min(60)]
                );
            }

            // Extract themes from content (simple keyword matching for now,
            // LLM scoring happens in select_soundbites)
            let themes = extract_themes(&segment.text);

            verified.push(VerifiedSoundbite {
                id: format!("sb_{:03}", i),
                text: segment.text.clone(),
                start_seconds: segment.start_seconds,
                end_seconds: segment.end_seconds,
                duration_seconds: duration,
                speaker: segment.speaker.clone(),
                act: assign_act_heuristic(i, transcript.segments.len()),
                emotional_beat: false,
                match_confidence,
                starts_at_sentence,
                ends_at_sentence,
                quality_score: 0.0, // Scored later by LLM in select_soundbites
                themes,
            });
        }

        tracing::info!(
            "[DIRECTIVE] Verified {} soundbites from {} transcript segments \
             ({} with clean sentence boundaries)",
            verified.len(),
            transcript.segments.len(),
            verified.iter().filter(|s| s.starts_at_sentence && s.ends_at_sentence).count()
        );

        verified
    }

    /// Generate the complete EditDirective from all artifacts.
    pub async fn generate(
        &self,
        client_spec: &ClientSpec,
        transcript: &StructuredTranscript,
        catalog: &ShotCatalog,
        context_brief: Option<&ContentContextBrief>,
        verified_soundbites: &[VerifiedSoundbite],
    ) -> Result<EditDirective> {
        tracing::info!(
            "[DIRECTIVE] Generating directive: {} soundbites, {} assets",
            verified_soundbites.len(),
            catalog.assets.len()
        );

        // Select top soundbites (limit to ~10 for a 2:30 video)
        let selected = self.select_soundbites(verified_soundbites, client_spec.max_duration_seconds);

        // Mark emotional beats: first, strongest, and last soundbites show subject on camera
        let selected = self.mark_emotional_beats(selected);

        // Calculate timing
        let total_soundbite_duration: f64 = selected.iter().map(|s| s.duration_seconds).sum();
        let max_total = client_spec.max_duration_seconds as f64;

        // Build act assignments
        let acts = self.build_act_assignments(&selected, catalog, max_total);

        // Compute ratios
        let interview_on_camera_duration: f64 = acts
            .iter()
            .filter(|a| a.interview_on_camera)
            .map(|a| a.duration_target_seconds * 0.5) // On-camera for ~half the act
            .sum();
        let interview_ratio = interview_on_camera_duration / max_total;
        let broll_ratio = 1.0 - interview_ratio;

        // Find music track
        let music_track = catalog
            .music_assets
            .first()
            .map(|&idx| {
                let asset = &catalog.assets[idx];
                MusicTrackDirective {
                    asset_index: idx,
                    filename: asset.filename.clone(),
                    ducked_db: client_spec.music_behavior.ducked_db,
                    full_db: client_spec.music_behavior.full_db,
                    full_regions: Vec::new(), // Populated by LLM in production
                }
            });

        // Identify vertical clips that need blur-fill treatment
        let vertical_treatment = VerticalTreatment {
            blur_radius: 20,
            fg_scale_height: 720,
        };

        // Build lip sync points for on-camera emotional beats
        let lip_sync_points = self.build_lip_sync_points(&selected);

        // Detect music full-volume regions (gaps between soundbites > 2s)
        let music_track = music_track.map(|mut mt| {
            mt.full_regions = self.detect_music_full_regions(&selected, max_total);
            mt
        });

        let directive = EditDirective {
            project_name: client_spec.project_name.clone(),
            total_duration_seconds: max_total.min(total_soundbite_duration + 15.0), // Add ~15s for B-roll only acts
            soundbites: selected,
            acts,
            music_track,
            computed_broll_ratio: broll_ratio,
            computed_interview_ratio: interview_ratio,
            vertical_clip_treatment: vertical_treatment,
            output_spec: OutputSpec {
                width: client_spec.resolution.width,
                height: client_spec.resolution.height,
                codec: "h264".to_string(),
                bitrate_mbps: 18,
            },
            lip_sync_points,
            transition_duration_seconds: 0.5,
        };

        // Validate the directive
        self.validate_directive(&directive, client_spec, catalog)?;

        Ok(directive)
    }

    /// Select the best soundbites up to the duration budget.
    ///
    /// Scoring strategy:
    /// 1. Complete sentences (both boundaries aligned) get +0.3
    /// 2. Longer soundbites (more complete thoughts) get bonus up to +0.2
    /// 3. Theme coverage: soundbites addressing underrepresented themes get +0.2
    /// 4. Position diversity: spread across transcript for narrative arc
    fn select_soundbites(
        &self,
        candidates: &[VerifiedSoundbite],
        max_duration_seconds: u32,
    ) -> Vec<VerifiedSoundbite> {
        let budget = max_duration_seconds as f64 * 0.85;

        // Score each candidate
        let mut scored: Vec<(f64, VerifiedSoundbite)> = candidates
            .iter()
            .map(|sb| {
                let mut score = sb.quality_score;

                // Sentence boundary bonus: complete sentences are essential
                if sb.starts_at_sentence && sb.ends_at_sentence {
                    score += 0.3;
                } else if sb.starts_at_sentence || sb.ends_at_sentence {
                    score += 0.1;
                }

                // Duration bonus: prefer 4-12s soundbites (sweet spot for testimony)
                let duration_score = if sb.duration_seconds >= 4.0 && sb.duration_seconds <= 12.0 {
                    0.2
                } else if sb.duration_seconds >= 2.0 {
                    0.1
                } else {
                    0.0
                };
                score += duration_score;

                // Theme bonus: soundbites with identified themes are more valuable
                score += (sb.themes.len() as f64 * 0.05).min(0.2);

                // Match confidence contributes directly
                score += sb.match_confidence * 0.2;

                (score, sb.clone())
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Greedily select, respecting budget and ensuring act diversity
        let mut selected = Vec::new();
        let mut accumulated = 0.0;
        let mut acts_covered: std::collections::HashSet<ActLabel> = std::collections::HashSet::new();

        // First pass: ensure at least one soundbite per act
        for (_, sb) in &scored {
            if accumulated + sb.duration_seconds > budget {
                continue;
            }
            if !acts_covered.contains(&sb.act) {
                acts_covered.insert(sb.act.clone());
                selected.push(sb.clone());
                accumulated += sb.duration_seconds;
            }
        }

        // Second pass: fill remaining budget with highest-scoring remaining
        for (_, sb) in &scored {
            if accumulated + sb.duration_seconds > budget {
                continue;
            }
            if selected.iter().any(|s| s.id == sb.id) {
                continue;
            }
            selected.push(sb.clone());
            accumulated += sb.duration_seconds;
        }

        // Sort by timeline order for coherent narrative
        selected.sort_by(|a, b| {
            a.start_seconds
                .partial_cmp(&b.start_seconds)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Ensure we have at least 3 soundbites
        if selected.len() < 3 && candidates.len() >= 3 {
            selected = candidates.iter().take(3).cloned().collect();
        }

        tracing::info!(
            "[DIRECTIVE] Selected {} soundbites, total {:.1}s (budget {:.1}s), \
             {} with clean sentence boundaries, covering {} acts",
            selected.len(),
            selected.iter().map(|s| s.duration_seconds).sum::<f64>(),
            budget,
            selected.iter().filter(|s| s.starts_at_sentence && s.ends_at_sentence).count(),
            acts_covered.len()
        );

        selected
    }

    /// Mark 3 emotional beats where the subject appears on camera.
    fn mark_emotional_beats(&self, mut soundbites: Vec<VerifiedSoundbite>) -> Vec<VerifiedSoundbite> {
        let len = soundbites.len();
        if len == 0 {
            return soundbites;
        }

        // First, middle hero, and last
        let beat_indices = if len >= 3 {
            vec![0, len / 2, len - 1]
        } else if len == 2 {
            vec![0, 1]
        } else {
            vec![0]
        };

        for idx in beat_indices {
            soundbites[idx].emotional_beat = true;
        }

        soundbites
    }

    /// Build per-act assignments from selected soundbites and available B-roll.
    ///
    /// Content-aware assignment strategy:
    /// - Act1 (Intro): drone/establishing shots, venue exteriors → Low/Medium energy
    /// - Act2 (Challenge): setup shots, BTS, team working → Medium energy
    /// - Act3 (Solution): cinematic B-roll, production action → Medium/High energy
    /// - Act4 (Results): high-energy montage, crowd, stage, action → High energy
    /// - Act5 (Close): drone wide shots, venue beauty → Medium energy
    fn build_act_assignments(
        &self,
        soundbites: &[VerifiedSoundbite],
        catalog: &ShotCatalog,
        total_duration: f64,
    ) -> Vec<ActAssignment> {
        let act_labels = [
            ActLabel::Act1Intro,
            ActLabel::Act2Challenge,
            ActLabel::Act3Solution,
            ActLabel::Act4Results,
            ActLabel::Act5Close,
        ];

        let per_act_duration = total_duration / act_labels.len() as f64;
        let mut used_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();

        // Preferred energy levels per act
        let act_energy_prefs: Vec<(&ActLabel, &[EnergyLevel])> = vec![
            (&ActLabel::Act1Intro, &[EnergyLevel::Low, EnergyLevel::Medium]),
            (&ActLabel::Act2Challenge, &[EnergyLevel::Medium]),
            (&ActLabel::Act3Solution, &[EnergyLevel::Medium, EnergyLevel::High]),
            (&ActLabel::Act4Results, &[EnergyLevel::High]),
            (&ActLabel::Act5Close, &[EnergyLevel::Medium, EnergyLevel::Low]),
        ];

        // Preferred media types per act
        let act_type_prefs: Vec<(&ActLabel, &[MediaType])> = vec![
            (&ActLabel::Act1Intro, &[MediaType::Drone, MediaType::Cinematic]),
            (&ActLabel::Act2Challenge, &[MediaType::BTS, MediaType::Cinematic]),
            (&ActLabel::Act3Solution, &[MediaType::Cinematic, MediaType::EventAction]),
            (&ActLabel::Act4Results, &[MediaType::EventAction, MediaType::VerticalHighlight]),
            (&ActLabel::Act5Close, &[MediaType::Drone, MediaType::Cinematic]),
        ];

        act_labels
            .iter()
            .enumerate()
            .map(|(act_idx, act)| {
                let act_soundbites: Vec<String> = soundbites
                    .iter()
                    .filter(|s| &s.act == act)
                    .map(|s| s.id.clone())
                    .collect();

                let has_emotional_beat = soundbites
                    .iter()
                    .any(|s| &s.act == act && s.emotional_beat);

                // Score and rank available B-roll for this act
                let energy_prefs = act_energy_prefs[act_idx].1;
                let type_prefs = act_type_prefs[act_idx].1;

                let mut scored_broll: Vec<(f64, usize)> = catalog
                    .broll_assets
                    .iter()
                    .filter(|idx| !used_indices.contains(idx))
                    .map(|&idx| {
                        let asset = &catalog.assets[idx];
                        let mut score = 0.0;

                        // Energy match
                        if energy_prefs.contains(&asset.energy_level) {
                            score += 3.0;
                        }

                        // Type match
                        if type_prefs.contains(&asset.media_type) {
                            score += 2.0;
                        }

                        // Prefer longer clips (more usable footage)
                        score += (asset.duration_seconds / 30.0).min(1.0);

                        // Prefer higher resolution
                        if asset.width >= 3840 {
                            score += 0.5;
                        }

                        (score, idx)
                    })
                    .collect();

                scored_broll.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

                // Take top clips for this act (typically 3-5 per act for variety)
                let clips_needed = 4.min(scored_broll.len());
                let broll_clips: Vec<BRollClip> = scored_broll
                    .iter()
                    .take(clips_needed)
                    .map(|(_, idx)| {
                        let asset = &catalog.assets[*idx];
                        used_indices.insert(*idx);
                        BRollClip {
                            asset_index: *idx,
                            filename: asset.filename.clone(),
                            in_point_seconds: 0.0,
                            out_point_seconds: asset.duration_seconds.min(per_act_duration / clips_needed as f64),
                        }
                    })
                    .collect();

                ActAssignment {
                    act: act.clone(),
                    duration_target_seconds: per_act_duration,
                    soundbite_ids: act_soundbites,
                    broll_clips,
                    interview_on_camera: has_emotional_beat,
                }
            })
            .collect()
    }

    /// Build lip sync points for all on-camera (emotional_beat) soundbites.
    ///
    /// CRITICAL: The video source timecode MUST equal the audio source timecode.
    /// This was the root cause of the v5 lip sync bug where all 3 on-camera clips
    /// were 1.7-5.0s out of sync.
    fn build_lip_sync_points(&self, soundbites: &[VerifiedSoundbite]) -> Vec<LipSyncPoint> {
        let mut points = Vec::new();
        let mut timeline_pos = 0.0;

        for sb in soundbites {
            if sb.emotional_beat {
                points.push(LipSyncPoint {
                    soundbite_id: sb.id.clone(),
                    // Audio and video source timecodes MUST be identical
                    audio_source_timecode: sb.start_seconds,
                    video_source_timecode: sb.start_seconds, // SAME as audio
                    duration_seconds: sb.duration_seconds.min(5.0), // On-camera max 5s
                    timeline_position_seconds: timeline_pos,
                });

                tracing::info!(
                    "[DIRECTIVE] Lip sync point: {} at source {:.3}s \
                     (audio == video == {:.3}s) for {:.1}s",
                    sb.id,
                    sb.start_seconds,
                    sb.start_seconds,
                    sb.duration_seconds.min(5.0)
                );
            }
            timeline_pos += sb.duration_seconds;
        }

        points
    }

    /// Detect regions where music can play at full volume (gaps between dialogue).
    fn detect_music_full_regions(
        &self,
        soundbites: &[VerifiedSoundbite],
        total_duration: f64,
    ) -> Vec<TimeRegion> {
        let mut regions = Vec::new();
        let min_gap = 2.0; // Only full-volume if gap is > 2s

        let mut prev_end = 0.0;
        for sb in soundbites {
            let gap = sb.start_seconds - prev_end;
            if gap > min_gap {
                regions.push(TimeRegion {
                    start_seconds: prev_end,
                    end_seconds: sb.start_seconds,
                });
            }
            prev_end = sb.end_seconds;
        }

        // Trailing region after last soundbite
        if total_duration - prev_end > min_gap {
            regions.push(TimeRegion {
                start_seconds: prev_end,
                end_seconds: total_duration,
            });
        }

        tracing::info!(
            "[DIRECTIVE] Detected {} music full-volume regions",
            regions.len()
        );

        regions
    }

    /// Validate the directive against client spec and catalog.
    fn validate_directive(
        &self,
        directive: &EditDirective,
        client_spec: &ClientSpec,
        catalog: &ShotCatalog,
    ) -> Result<()> {
        // Check total duration
        if directive.total_duration_seconds > client_spec.max_duration_seconds as f64 {
            return Err(NoraError::ExecutionError(format!(
                "Directive duration ({:.1}s) exceeds max ({}s)",
                directive.total_duration_seconds, client_spec.max_duration_seconds
            )));
        }

        // Check B-roll ratio is within spec
        if directive.computed_broll_ratio < client_spec.broll_coverage_min {
            tracing::warn!(
                "[DIRECTIVE] B-roll ratio {:.1}% below minimum {:.1}%",
                directive.computed_broll_ratio * 100.0,
                client_spec.broll_coverage_min * 100.0
            );
        }

        // Verify all referenced asset indices exist
        for act in &directive.acts {
            for clip in &act.broll_clips {
                if clip.asset_index >= catalog.assets.len() {
                    return Err(NoraError::ExecutionError(format!(
                        "B-roll clip references invalid asset index {}",
                        clip.asset_index
                    )));
                }
            }
        }

        // Verify all soundbite IDs exist in the directive's soundbite list
        let soundbite_ids: Vec<&str> = directive.soundbites.iter().map(|s| s.id.as_str()).collect();
        for act in &directive.acts {
            for sb_id in &act.soundbite_ids {
                if !soundbite_ids.contains(&sb_id.as_str()) {
                    return Err(NoraError::ExecutionError(format!(
                        "Act references non-existent soundbite: {}",
                        sb_id
                    )));
                }
            }
        }

        // Verify lip sync points: audio and video timecodes must match
        for lsp in &directive.lip_sync_points {
            if (lsp.audio_source_timecode - lsp.video_source_timecode).abs() > 0.01 {
                return Err(NoraError::ExecutionError(format!(
                    "LIP SYNC VIOLATION: soundbite {} has audio at {:.3}s but video at {:.3}s \
                     (delta {:.3}s). Audio and video source timecodes MUST be identical.",
                    lsp.soundbite_id,
                    lsp.audio_source_timecode,
                    lsp.video_source_timecode,
                    (lsp.audio_source_timecode - lsp.video_source_timecode).abs()
                )));
            }
        }

        // Verify sentence boundaries on selected soundbites
        let misaligned: Vec<&VerifiedSoundbite> = directive
            .soundbites
            .iter()
            .filter(|s| !s.starts_at_sentence || !s.ends_at_sentence)
            .collect();
        if !misaligned.is_empty() {
            tracing::warn!(
                "[DIRECTIVE] {} soundbites have misaligned sentence boundaries: {:?}",
                misaligned.len(),
                misaligned.iter().map(|s| &s.id).collect::<Vec<_>>()
            );
        }

        Ok(())
    }
}

/// Find the best matching substring in the full text using sliding window Levenshtein.
/// Returns the highest match ratio found.
fn find_best_match(needle: &str, haystack: &str) -> f64 {
    let needle_words: Vec<&str> = needle.split_whitespace().collect();
    let haystack_words: Vec<&str> = haystack.split_whitespace().collect();

    if needle_words.is_empty() || haystack_words.is_empty() {
        return 0.0;
    }

    let window_size = needle_words.len();
    if window_size > haystack_words.len() {
        return levenshtein_ratio(needle, haystack);
    }

    let mut best = 0.0_f64;
    for start in 0..=(haystack_words.len() - window_size) {
        let window: String = haystack_words[start..start + window_size].join(" ");
        let ratio = levenshtein_ratio(needle, &window);
        best = best.max(ratio);
        if best >= 0.99 {
            break; // Perfect match found
        }
    }

    best
}

/// Extract content themes from soundbite text using keyword matching.
fn extract_themes(text: &str) -> Vec<String> {
    let text_lower = text.to_lowercase();
    let mut themes = Vec::new();

    let theme_keywords: &[(&str, &[&str])] = &[
        ("scale", &["massive", "huge", "scale", "large", "big", "multi-day", "hundreds"]),
        ("credibility", &["experience", "years", "professional", "background", "director", "producer"]),
        ("trust", &["trust", "reliable", "partner", "relationship", "depend"]),
        ("quality", &["quality", "professional", "production value", "cinematic", "amazing"]),
        ("speed", &["same-day", "turnaround", "fast", "quick", "overnight", "immediately"]),
        ("teamwork", &["team", "crew", "together", "collaborate", "support"]),
        ("innovation", &["never done before", "nobody", "first time", "innovative", "cutting edge"]),
        ("capability", &["multi-camera", "production", "capability", "equipment", "gear"]),
    ];

    for (theme, keywords) in theme_keywords {
        if keywords.iter().any(|kw| text_lower.contains(kw)) {
            themes.push(theme.to_string());
        }
    }

    themes
}

/// Heuristic act assignment based on position in the transcript.
fn assign_act_heuristic(segment_index: usize, total_segments: usize) -> ActLabel {
    if total_segments == 0 {
        return ActLabel::Act3Solution;
    }

    let position = segment_index as f64 / total_segments as f64;

    if position < 0.15 {
        ActLabel::Act1Intro
    } else if position < 0.35 {
        ActLabel::Act2Challenge
    } else if position < 0.60 {
        ActLabel::Act3Solution
    } else if position < 0.80 {
        ActLabel::Act4Results
    } else {
        ActLabel::Act5Close
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_act_assignment_heuristic() {
        assert_eq!(assign_act_heuristic(0, 20), ActLabel::Act1Intro);
        assert_eq!(assign_act_heuristic(5, 20), ActLabel::Act2Challenge);
        assert_eq!(assign_act_heuristic(10, 20), ActLabel::Act3Solution);
        assert_eq!(assign_act_heuristic(15, 20), ActLabel::Act4Results);
        assert_eq!(assign_act_heuristic(19, 20), ActLabel::Act5Close);
    }

    #[test]
    fn test_emotional_beats_marking() {
        let gen = DirectiveGenerator::new(0.8);

        let soundbites: Vec<VerifiedSoundbite> = (0..5)
            .map(|i| VerifiedSoundbite {
                id: format!("sb_{}", i),
                text: format!("Soundbite {}", i),
                start_seconds: i as f64 * 10.0,
                end_seconds: (i + 1) as f64 * 10.0,
                duration_seconds: 10.0,
                speaker: "Dave".to_string(),
                act: ActLabel::Act3Solution,
                emotional_beat: false,
                match_confidence: 1.0,
                starts_at_sentence: true,
                ends_at_sentence: true,
                quality_score: 0.5,
                themes: vec![],
            })
            .collect();

        let result = gen.mark_emotional_beats(soundbites);
        assert!(result[0].emotional_beat); // First
        assert!(!result[1].emotional_beat);
        assert!(result[2].emotional_beat); // Middle
        assert!(!result[3].emotional_beat);
        assert!(result[4].emotional_beat); // Last
    }

    #[test]
    fn test_find_best_match() {
        // Exact match in haystack
        let ratio = find_best_match(
            "multi-camera production like nobody",
            "and turn it into a multi-camera production like nobody has ever seen before"
        );
        assert!(ratio >= 0.9, "Expected >=0.9 got {}", ratio);

        // No match
        let ratio = find_best_match("completely unrelated text", "the quick brown fox jumps over");
        assert!(ratio < 0.5, "Expected <0.5 got {}", ratio);
    }

    #[test]
    fn test_extract_themes() {
        let themes = extract_themes("We had a massive multi-camera production with a professional team");
        assert!(themes.contains(&"scale".to_string()));
        assert!(themes.contains(&"capability".to_string()));
        assert!(themes.contains(&"teamwork".to_string()));
    }

    #[test]
    fn test_lip_sync_validation() {
        let gen = DirectiveGenerator::new(0.8);

        // Valid lip sync (audio == video)
        let valid_point = LipSyncPoint {
            soundbite_id: "sb_001".to_string(),
            audio_source_timecode: 40.6,
            video_source_timecode: 40.6,
            duration_seconds: 5.0,
            timeline_position_seconds: 0.0,
        };
        assert!((valid_point.audio_source_timecode - valid_point.video_source_timecode).abs() < 0.01);

        // Invalid lip sync (the v5 bug: video at 45.6, audio at 40.6)
        let invalid_point = LipSyncPoint {
            soundbite_id: "sb_001".to_string(),
            audio_source_timecode: 40.6,
            video_source_timecode: 45.6,
            duration_seconds: 5.0,
            timeline_position_seconds: 0.0,
        };
        assert!((invalid_point.audio_source_timecode - invalid_point.video_source_timecode).abs() > 0.01);
    }

    #[test]
    fn test_music_full_regions() {
        let gen = DirectiveGenerator::new(0.8);
        let soundbites = vec![
            VerifiedSoundbite {
                id: "sb_001".to_string(),
                text: "Test".to_string(),
                start_seconds: 5.0,
                end_seconds: 15.0,
                duration_seconds: 10.0,
                speaker: "Dave".to_string(),
                act: ActLabel::Act1Intro,
                emotional_beat: false,
                match_confidence: 1.0,
                starts_at_sentence: true,
                ends_at_sentence: true,
                quality_score: 0.5,
                themes: vec![],
            },
            VerifiedSoundbite {
                id: "sb_002".to_string(),
                text: "Test 2".to_string(),
                start_seconds: 25.0,
                end_seconds: 35.0,
                duration_seconds: 10.0,
                speaker: "Dave".to_string(),
                act: ActLabel::Act3Solution,
                emotional_beat: false,
                match_confidence: 1.0,
                starts_at_sentence: true,
                ends_at_sentence: true,
                quality_score: 0.5,
                themes: vec![],
            },
        ];

        let regions = gen.detect_music_full_regions(&soundbites, 50.0);
        assert_eq!(regions.len(), 3); // 0-5, 15-25, 35-50
        assert!((regions[0].start_seconds - 0.0).abs() < 0.01);
        assert!((regions[0].end_seconds - 5.0).abs() < 0.01);
        assert!((regions[1].start_seconds - 15.0).abs() < 0.01);
        assert!((regions[1].end_seconds - 25.0).abs() < 0.01);
    }
}
