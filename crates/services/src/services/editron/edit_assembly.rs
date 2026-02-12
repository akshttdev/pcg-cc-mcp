//! Edit Assembly Engine
//!
//! Automated edit creation based on music analysis and footage inventory.
//! Creates intelligent, beat-synced edits from raw footage.

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use super::{EditronError, EditronResult, VideoMetadata};

/// Represents a piece of footage with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FootageClip {
    pub path: PathBuf,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f32,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub energy_level: f32,      // 0.0-1.0
    pub motion_intensity: f32,  // 0.0-1.0
    pub in_point: f64,          // Best starting point
    pub out_point: f64,         // Best ending point
    pub hero_moment: Option<f64>, // Peak visual moment
    pub thumbnail: Option<PathBuf>,
}

impl FootageClip {
    pub fn new(path: PathBuf, metadata: &VideoMetadata) -> Self {
        Self {
            path,
            duration: metadata.duration_seconds,
            width: metadata.width,
            height: metadata.height,
            frame_rate: metadata.frame_rate,
            categories: Vec::new(),
            tags: Vec::new(),
            energy_level: 0.5,
            motion_intensity: 0.5,
            in_point: 0.0,
            out_point: metadata.duration_seconds,
            hero_moment: None,
            thumbnail: None,
        }
    }

    pub fn usable_duration(&self) -> f64 {
        self.out_point - self.in_point
    }

    pub fn with_categories(mut self, categories: Vec<String>) -> Self {
        self.categories = categories;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_energy(mut self, energy: f32) -> Self {
        self.energy_level = energy.clamp(0.0, 1.0);
        self
    }
}

/// Music analysis for edit synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicAnalysis {
    pub path: PathBuf,
    pub duration: f64,
    pub bpm: f32,
    pub beats: Vec<f64>,        // Beat timestamps in seconds
    pub downbeats: Vec<f64>,    // Strong beat timestamps (1 of each bar)
    pub sections: Vec<MusicSection>,
    pub energy_curve: Vec<(f64, f32)>, // (time, energy 0-1)
}

impl MusicAnalysis {
    /// Get the beat interval in seconds
    pub fn beat_interval(&self) -> f64 {
        60.0 / self.bpm as f64
    }

    /// Get the bar duration (assuming 4/4 time)
    pub fn bar_duration(&self) -> f64 {
        self.beat_interval() * 4.0
    }

    /// Find the nearest beat to a given time
    pub fn nearest_beat(&self, time: f64) -> f64 {
        self.beats.iter()
            .min_by(|a, b| {
                let diff_a = (time - *a).abs();
                let diff_b = (time - *b).abs();
                diff_a.partial_cmp(&diff_b).unwrap()
            })
            .copied()
            .unwrap_or(time)
    }

    /// Find the nearest downbeat to a given time
    pub fn nearest_downbeat(&self, time: f64) -> f64 {
        self.downbeats.iter()
            .min_by(|a, b| {
                let diff_a = (time - *a).abs();
                let diff_b = (time - *b).abs();
                diff_a.partial_cmp(&diff_b).unwrap()
            })
            .copied()
            .unwrap_or(time)
    }

    /// Get energy level at a given time
    pub fn energy_at(&self, time: f64) -> f32 {
        if self.energy_curve.is_empty() {
            return 0.5;
        }

        // Find surrounding points and interpolate
        let mut prev = &self.energy_curve[0];
        for point in &self.energy_curve {
            if point.0 > time {
                // Interpolate
                if prev.0 == point.0 {
                    return point.1;
                }
                let t = (time - prev.0) / (point.0 - prev.0);
                return prev.1 + (point.1 - prev.1) * t as f32;
            }
            prev = point;
        }
        self.energy_curve.last().map(|p| p.1).unwrap_or(0.5)
    }

    /// Get section at a given time
    pub fn section_at(&self, time: f64) -> Option<&MusicSection> {
        self.sections.iter().find(|s| time >= s.start && time < s.end)
    }

    /// Generate beat grid from BPM
    pub fn generate_beat_grid(duration: f64, bpm: f32, offset: f64) -> (Vec<f64>, Vec<f64>) {
        let beat_interval = 60.0 / bpm as f64;
        let mut beats = Vec::new();
        let mut downbeats = Vec::new();

        let mut time = offset;
        let mut beat_count = 0;

        while time < duration {
            beats.push(time);
            if beat_count % 4 == 0 {
                downbeats.push(time);
            }
            time += beat_interval;
            beat_count += 1;
        }

        (beats, downbeats)
    }
}

/// A section of music (intro, verse, chorus, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicSection {
    pub name: String,
    pub section_type: SectionType,
    pub start: f64,
    pub end: f64,
    pub energy: f32,
    pub suggested_pacing: PacingStyle,
}

impl MusicSection {
    pub fn duration(&self) -> f64 {
        self.end - self.start
    }
}

/// Types of music sections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionType {
    Intro,
    Buildup,
    Verse,
    PreChorus,
    Chorus,
    Drop,
    Bridge,
    Breakdown,
    Outro,
    Transition,
}

/// Pacing styles for different edit feels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PacingStyle {
    Slow,       // Long holds, minimal cuts (4+ seconds per clip)
    Moderate,   // Standard pacing (2-4 seconds per clip)
    Fast,       // Quick cuts (1-2 seconds per clip)
    Rapid,      // Very quick cuts (0.5-1 seconds per clip)
    Dynamic,    // Varies with music energy
}

impl PacingStyle {
    /// Get target clip duration range in seconds
    pub fn duration_range(&self) -> (f64, f64) {
        match self {
            PacingStyle::Slow => (4.0, 8.0),
            PacingStyle::Moderate => (2.0, 4.0),
            PacingStyle::Fast => (1.0, 2.0),
            PacingStyle::Rapid => (0.5, 1.0),
            PacingStyle::Dynamic => (0.5, 6.0),
        }
    }
}

/// A clip placement in the timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineClip {
    pub source: PathBuf,
    pub source_in: f64,     // In point in source clip
    pub source_out: f64,    // Out point in source clip
    pub timeline_in: f64,   // Start on timeline
    pub timeline_out: f64,  // End on timeline
    pub track: u32,         // Video track number
    pub opacity: f32,       // 0.0-1.0
    pub scale: f32,         // 1.0 = 100%
    pub position: (f32, f32), // X, Y offset
    pub speed: f32,         // 1.0 = normal speed
    pub transition_in: Option<TransitionSpec>,
    pub transition_out: Option<TransitionSpec>,
}

impl TimelineClip {
    pub fn new(source: PathBuf, source_in: f64, source_out: f64, timeline_in: f64) -> Self {
        let duration = source_out - source_in;
        Self {
            source,
            source_in,
            source_out,
            timeline_in,
            timeline_out: timeline_in + duration,
            track: 1,
            opacity: 1.0,
            scale: 1.0,
            position: (0.0, 0.0),
            speed: 1.0,
            transition_in: None,
            transition_out: None,
        }
    }

    pub fn duration(&self) -> f64 {
        self.timeline_out - self.timeline_in
    }

    pub fn source_duration(&self) -> f64 {
        self.source_out - self.source_in
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        // Adjust timeline out based on speed
        let source_duration = self.source_out - self.source_in;
        self.timeline_out = self.timeline_in + (source_duration / speed as f64);
        self
    }

    pub fn with_track(mut self, track: u32) -> Self {
        self.track = track;
        self
    }

    pub fn with_transition_in(mut self, transition: TransitionSpec) -> Self {
        self.transition_in = Some(transition);
        self
    }

    pub fn with_transition_out(mut self, transition: TransitionSpec) -> Self {
        self.transition_out = Some(transition);
        self
    }
}

/// Transition specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSpec {
    pub transition_type: String,
    pub duration: f64,
    pub params: HashMap<String, f64>,
}

impl TransitionSpec {
    pub fn dissolve(duration: f64) -> Self {
        Self {
            transition_type: "dissolve".to_string(),
            duration,
            params: HashMap::new(),
        }
    }

    pub fn dip_to_black(duration: f64) -> Self {
        Self {
            transition_type: "dip_to_black".to_string(),
            duration,
            params: HashMap::new(),
        }
    }

    pub fn wipe(duration: f64, angle: f64) -> Self {
        let mut params = HashMap::new();
        params.insert("angle".to_string(), angle);
        Self {
            transition_type: "wipe".to_string(),
            duration,
            params,
        }
    }
}

/// Audio clip on timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioClip {
    pub source: PathBuf,
    pub source_in: f64,
    pub source_out: f64,
    pub timeline_in: f64,
    pub timeline_out: f64,
    pub track: u32,
    pub volume: f32,        // 0.0-2.0 (1.0 = unity)
    pub fade_in: Option<f64>,
    pub fade_out: Option<f64>,
    pub is_music: bool,
}

impl AudioClip {
    pub fn music(source: PathBuf, duration: f64) -> Self {
        Self {
            source,
            source_in: 0.0,
            source_out: duration,
            timeline_in: 0.0,
            timeline_out: duration,
            track: 1,
            volume: 1.0,
            fade_in: Some(0.5),
            fade_out: Some(1.0),
            is_music: true,
        }
    }

    pub fn duration(&self) -> f64 {
        self.timeline_out - self.timeline_in
    }
}

/// Complete assembled edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledEdit {
    pub name: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f32,
    pub video_clips: Vec<TimelineClip>,
    pub audio_clips: Vec<AudioClip>,
    pub markers: Vec<EditMarker>,
}

impl AssembledEdit {
    pub fn new(name: &str, duration: f64, width: u32, height: u32, frame_rate: f32) -> Self {
        Self {
            name: name.to_string(),
            duration,
            width,
            height,
            frame_rate,
            video_clips: Vec::new(),
            audio_clips: Vec::new(),
            markers: Vec::new(),
        }
    }

    pub fn add_video_clip(&mut self, clip: TimelineClip) {
        self.video_clips.push(clip);
    }

    pub fn add_audio_clip(&mut self, clip: AudioClip) {
        self.audio_clips.push(clip);
    }

    pub fn add_marker(&mut self, marker: EditMarker) {
        self.markers.push(marker);
    }

    /// Sort clips by timeline position
    pub fn sort_clips(&mut self) {
        self.video_clips.sort_by(|a, b| {
            a.timeline_in.partial_cmp(&b.timeline_in).unwrap()
        });
        self.audio_clips.sort_by(|a, b| {
            a.timeline_in.partial_cmp(&b.timeline_in).unwrap()
        });
    }
}

/// Timeline marker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditMarker {
    pub time: f64,
    pub name: String,
    pub color: String,
    pub marker_type: MarkerType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkerType {
    Chapter,
    Beat,
    Section,
    Note,
}

/// Edit assembly configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyConfig {
    pub name: String,
    pub target_duration: Option<f64>,  // If None, uses music duration
    pub pacing: PacingStyle,
    pub sync_to_beats: bool,
    pub sync_to_downbeats: bool,       // Major cuts on downbeats
    pub allow_speed_ramping: bool,
    pub max_speed: f32,                // Max speed adjustment
    pub min_clip_duration: f64,
    pub transition_style: TransitionStyle,
    pub category_sequence: Option<Vec<String>>, // Ordered categories to use
    pub energy_matching: bool,         // Match clip energy to music energy
}

impl Default for AssemblyConfig {
    fn default() -> Self {
        Self {
            name: "Untitled Edit".to_string(),
            target_duration: None,
            pacing: PacingStyle::Dynamic,
            sync_to_beats: true,
            sync_to_downbeats: true,
            allow_speed_ramping: false,
            max_speed: 1.5,
            min_clip_duration: 0.5,
            transition_style: TransitionStyle::Cut,
            category_sequence: None,
            energy_matching: true,
        }
    }
}

/// Transition style preference
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionStyle {
    Cut,            // Hard cuts only
    Dissolve,       // Cross dissolves
    Mixed,          // Variety of transitions
    Cinematic,      // Film-style transitions
}

/// The main edit assembly engine
pub struct EditAssemblyEngine {
    footage: Vec<FootageClip>,
    music: Option<MusicAnalysis>,
}

impl EditAssemblyEngine {
    pub fn new() -> Self {
        Self {
            footage: Vec::new(),
            music: None,
        }
    }

    /// Add footage to the assembly pool
    pub fn add_footage(&mut self, clip: FootageClip) {
        self.footage.push(clip);
    }

    /// Add multiple footage clips
    pub fn add_footage_batch(&mut self, clips: Vec<FootageClip>) {
        self.footage.extend(clips);
    }

    /// Set the music track for synchronization
    pub fn set_music(&mut self, analysis: MusicAnalysis) {
        self.music = Some(analysis);
    }

    /// Get footage by category
    pub fn footage_by_category(&self, category: &str) -> Vec<&FootageClip> {
        self.footage.iter()
            .filter(|c| c.categories.iter().any(|cat| cat.eq_ignore_ascii_case(category)))
            .collect()
    }

    /// Get footage by energy level range
    pub fn footage_by_energy(&self, min: f32, max: f32) -> Vec<&FootageClip> {
        self.footage.iter()
            .filter(|c| c.energy_level >= min && c.energy_level <= max)
            .collect()
    }

    /// Create an automated edit based on music and footage
    pub fn assemble(&self, config: &AssemblyConfig) -> EditronResult<AssembledEdit> {
        let music = self.music.as_ref()
            .ok_or_else(|| EditronError::InvalidFormat("No music set for assembly".to_string()))?;

        let duration = config.target_duration.unwrap_or(music.duration);

        // Determine sequence dimensions (use first clip or default to 4K)
        let (width, height, frame_rate) = self.footage.first()
            .map(|c| (c.width, c.height, c.frame_rate))
            .unwrap_or((3840, 2160, 23.976));

        let mut edit = AssembledEdit::new(&config.name, duration, width, height, frame_rate);

        // Add music track
        let music_clip = AudioClip::music(music.path.clone(), duration);
        edit.add_audio_clip(music_clip);

        // Add section markers
        for section in &music.sections {
            edit.add_marker(EditMarker {
                time: section.start,
                name: section.name.clone(),
                color: section_color(&section.section_type),
                marker_type: MarkerType::Section,
            });
        }

        // Build the video timeline
        let video_clips = self.build_video_timeline(music, config, duration)?;
        for clip in video_clips {
            edit.add_video_clip(clip);
        }

        edit.sort_clips();
        Ok(edit)
    }

    /// Build the video timeline based on music structure
    fn build_video_timeline(
        &self,
        music: &MusicAnalysis,
        config: &AssemblyConfig,
        duration: f64,
    ) -> EditronResult<Vec<TimelineClip>> {
        let mut clips = Vec::new();
        let mut current_time = 0.0;
        let mut used_footage: Vec<usize> = Vec::new(); // Track used clips to avoid repetition

        while current_time < duration {
            // Determine clip duration based on pacing and music
            let clip_duration = self.calculate_clip_duration(music, current_time, config);

            // Snap to beat if enabled
            let end_time = if config.sync_to_beats {
                let raw_end = current_time + clip_duration;
                if config.sync_to_downbeats && self.should_cut_on_downbeat(music, raw_end) {
                    music.nearest_downbeat(raw_end)
                } else {
                    music.nearest_beat(raw_end)
                }
            } else {
                current_time + clip_duration
            };

            // Don't exceed duration
            let end_time = end_time.min(duration);
            let actual_duration = end_time - current_time;

            if actual_duration < config.min_clip_duration {
                break;
            }

            // Select appropriate footage
            let footage_idx = self.select_footage(
                music,
                current_time,
                actual_duration,
                config,
                &used_footage,
            );

            if let Some(idx) = footage_idx {
                let footage = &self.footage[idx];

                // Determine in/out points in source
                let (source_in, source_out) = self.calculate_source_range(
                    footage,
                    actual_duration,
                    config,
                );

                let mut timeline_clip = TimelineClip::new(
                    footage.path.clone(),
                    source_in,
                    source_out,
                    current_time,
                );

                // Apply speed if needed
                if config.allow_speed_ramping {
                    let speed = self.calculate_speed(footage, actual_duration, source_out - source_in);
                    if speed != 1.0 && speed <= config.max_speed {
                        timeline_clip = timeline_clip.with_speed(speed);
                    }
                }

                // Add transitions based on style
                if let Some(transition) = self.create_transition(config, current_time, music) {
                    timeline_clip = timeline_clip.with_transition_in(transition);
                }

                clips.push(timeline_clip);
                used_footage.push(idx);

                // Reset used footage if we've used everything
                if used_footage.len() >= self.footage.len() {
                    used_footage.clear();
                }
            }

            current_time = end_time;
        }

        Ok(clips)
    }

    /// Calculate target clip duration based on music and pacing
    fn calculate_clip_duration(&self, music: &MusicAnalysis, time: f64, config: &AssemblyConfig) -> f64 {
        let (min_dur, max_dur) = config.pacing.duration_range();

        match config.pacing {
            PacingStyle::Dynamic => {
                // Vary duration based on music energy
                let energy = music.energy_at(time);
                // Higher energy = shorter clips
                let t = 1.0 - energy;
                min_dur + (max_dur - min_dur) * t as f64
            }
            _ => {
                // Use middle of range
                (min_dur + max_dur) / 2.0
            }
        }
    }

    /// Determine if we should cut on a downbeat
    fn should_cut_on_downbeat(&self, music: &MusicAnalysis, time: f64) -> bool {
        // Cut on downbeat if we're near a section change or at high energy moments
        if let Some(section) = music.section_at(time) {
            // Always cut on downbeat for chorus/drop
            matches!(section.section_type, SectionType::Chorus | SectionType::Drop)
        } else {
            false
        }
    }

    /// Select the best footage for a given moment
    fn select_footage(
        &self,
        music: &MusicAnalysis,
        time: f64,
        duration: f64,
        config: &AssemblyConfig,
        used: &[usize],
    ) -> Option<usize> {
        if self.footage.is_empty() {
            return None;
        }

        let music_energy = music.energy_at(time);
        let section = music.section_at(time);

        // Build candidate list
        let mut candidates: Vec<(usize, f32)> = self.footage.iter()
            .enumerate()
            .filter(|(idx, clip)| {
                // Filter out already used (unless we've used everything)
                !used.contains(idx) &&
                // Must have enough usable duration
                clip.usable_duration() >= duration * 0.5
            })
            .map(|(idx, clip)| {
                let mut score = 1.0f32;

                // Energy matching
                if config.energy_matching {
                    let energy_diff = (clip.energy_level - music_energy).abs();
                    score *= 1.0 - energy_diff * 0.5;
                }

                // Category matching based on section
                if let Some(section) = section {
                    if let Some(ref sequence) = config.category_sequence {
                        // Check if clip category matches desired sequence
                        let section_idx = music.sections.iter()
                            .position(|s| s.start == section.start)
                            .unwrap_or(0);
                        if let Some(desired_cat) = sequence.get(section_idx % sequence.len()) {
                            if clip.categories.iter().any(|c| c.eq_ignore_ascii_case(desired_cat)) {
                                score *= 1.5;
                            }
                        }
                    }

                    // Boost hero shots for chorus/drop
                    if matches!(section.section_type, SectionType::Chorus | SectionType::Drop) {
                        if clip.hero_moment.is_some() {
                            score *= 1.3;
                        }
                        if clip.energy_level > 0.7 {
                            score *= 1.2;
                        }
                    }
                }

                (idx, score)
            })
            .collect();

        // Sort by score descending
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        candidates.first().map(|(idx, _)| *idx)
    }

    /// Calculate source in/out points
    ///
    /// Priority order:
    /// 1. If in_point was set by Visual QC (> 0.0), use it as the starting point
    /// 2. If a hero_moment exists, center around it
    /// 3. Fallback to beginning of usable range
    fn calculate_source_range(&self, footage: &FootageClip, target_duration: f64, _config: &AssemblyConfig) -> (f64, f64) {
        let usable = footage.usable_duration();

        if usable <= target_duration {
            // Use full clip
            (footage.in_point, footage.out_point)
        } else if footage.in_point > 0.0 {
            // QC-optimized in-point: start from this point
            let end = (footage.in_point + target_duration).min(footage.out_point);
            (footage.in_point, end)
        } else if let Some(hero) = footage.hero_moment {
            // Center around hero moment
            let half = target_duration / 2.0;
            let start = (hero - half).max(footage.in_point);
            let end = (start + target_duration).min(footage.out_point);
            (start, end)
        } else {
            // Use beginning of usable range
            (footage.in_point, footage.in_point + target_duration)
        }
    }

    /// Calculate speed adjustment
    fn calculate_speed(&self, footage: &FootageClip, target_duration: f64, source_duration: f64) -> f32 {
        if source_duration <= 0.0 || target_duration <= 0.0 {
            return 1.0;
        }
        (source_duration / target_duration) as f32
    }

    /// Create transition based on style
    fn create_transition(&self, config: &AssemblyConfig, time: f64, music: &MusicAnalysis) -> Option<TransitionSpec> {
        match config.transition_style {
            TransitionStyle::Cut => None,
            TransitionStyle::Dissolve => {
                let duration = music.beat_interval() * 0.5;
                Some(TransitionSpec::dissolve(duration))
            }
            TransitionStyle::Mixed | TransitionStyle::Cinematic => {
                // Vary transition based on section
                if let Some(section) = music.section_at(time) {
                    match section.section_type {
                        SectionType::Intro | SectionType::Outro => {
                            Some(TransitionSpec::dissolve(music.beat_interval()))
                        }
                        SectionType::Breakdown => {
                            Some(TransitionSpec::dip_to_black(music.beat_interval() * 2.0))
                        }
                        _ => None, // Hard cut
                    }
                } else {
                    None
                }
            }
        }
    }

    /// Create a simple music analysis from BPM (when full analysis isn't available)
    pub fn simple_music_analysis(path: PathBuf, duration: f64, bpm: f32) -> MusicAnalysis {
        let (beats, downbeats) = MusicAnalysis::generate_beat_grid(duration, bpm, 0.0);

        // Create basic sections
        let bar_duration = 60.0 / bpm as f64 * 4.0;
        let mut sections = Vec::new();

        // Estimate sections based on typical song structure
        // Intro: 0-8 bars, Build: 8-16 bars, Drop/Chorus: 16-32 bars, etc.
        let intro_end = (bar_duration * 8.0).min(duration);
        let build_end = (bar_duration * 16.0).min(duration);
        let chorus_end = (bar_duration * 32.0).min(duration);
        let bridge_end = (bar_duration * 40.0).min(duration);

        if intro_end > 0.0 {
            sections.push(MusicSection {
                name: "Intro".to_string(),
                section_type: SectionType::Intro,
                start: 0.0,
                end: intro_end,
                energy: 0.4,
                suggested_pacing: PacingStyle::Slow,
            });
        }

        if build_end > intro_end {
            sections.push(MusicSection {
                name: "Build".to_string(),
                section_type: SectionType::Buildup,
                start: intro_end,
                end: build_end,
                energy: 0.6,
                suggested_pacing: PacingStyle::Moderate,
            });
        }

        if chorus_end > build_end {
            sections.push(MusicSection {
                name: "Chorus".to_string(),
                section_type: SectionType::Chorus,
                start: build_end,
                end: chorus_end,
                energy: 0.9,
                suggested_pacing: PacingStyle::Fast,
            });
        }

        if bridge_end > chorus_end && bridge_end < duration {
            sections.push(MusicSection {
                name: "Bridge".to_string(),
                section_type: SectionType::Bridge,
                start: chorus_end,
                end: bridge_end,
                energy: 0.5,
                suggested_pacing: PacingStyle::Moderate,
            });
        }

        if duration > bridge_end {
            sections.push(MusicSection {
                name: "Outro".to_string(),
                section_type: SectionType::Outro,
                start: bridge_end,
                end: duration,
                energy: 0.3,
                suggested_pacing: PacingStyle::Slow,
            });
        }

        // Generate energy curve from sections
        let energy_curve: Vec<(f64, f32)> = sections.iter()
            .flat_map(|s| vec![(s.start, s.energy), (s.end - 0.01, s.energy)])
            .collect();

        MusicAnalysis {
            path,
            duration,
            bpm,
            beats,
            downbeats,
            sections,
            energy_curve,
        }
    }
}

impl Default for EditAssemblyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Get marker color for section type
fn section_color(section_type: &SectionType) -> String {
    match section_type {
        SectionType::Intro => "#4A90D9".to_string(),      // Blue
        SectionType::Buildup => "#F5A623".to_string(),    // Orange
        SectionType::Verse => "#7ED321".to_string(),      // Green
        SectionType::PreChorus => "#BD10E0".to_string(),  // Purple
        SectionType::Chorus => "#D0021B".to_string(),     // Red
        SectionType::Drop => "#FF2D55".to_string(),       // Pink
        SectionType::Bridge => "#50E3C2".to_string(),     // Teal
        SectionType::Breakdown => "#9013FE".to_string(),  // Violet
        SectionType::Outro => "#4A4A4A".to_string(),      // Gray
        SectionType::Transition => "#FFCC00".to_string(), // Yellow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beat_grid_generation() {
        let (beats, downbeats) = MusicAnalysis::generate_beat_grid(10.0, 120.0, 0.0);

        // At 120 BPM, beat interval is 0.5 seconds
        assert!(!beats.is_empty());
        assert_eq!(beats[0], 0.0);
        assert!((beats[1] - 0.5).abs() < 0.01);

        // Downbeats should be every 4 beats
        assert!(!downbeats.is_empty());
        assert_eq!(downbeats[0], 0.0);
        assert!((downbeats[1] - 2.0).abs() < 0.01); // 4 beats at 0.5s each
    }

    #[test]
    fn test_pacing_duration_range() {
        assert_eq!(PacingStyle::Slow.duration_range(), (4.0, 8.0));
        assert_eq!(PacingStyle::Fast.duration_range(), (1.0, 2.0));
    }

    #[test]
    fn test_simple_music_analysis() {
        let analysis = EditAssemblyEngine::simple_music_analysis(
            PathBuf::from("/test/music.mp3"),
            104.0,
            125.0,
        );

        assert_eq!(analysis.duration, 104.0);
        assert_eq!(analysis.bpm, 125.0);
        assert!(!analysis.beats.is_empty());
        assert!(!analysis.sections.is_empty());
    }
}
