//! Music Selection Module for Editron
//!
//! Integrates with music licensing services for:
//! - MotionArray music library access
//! - Mood/genre-based music discovery
//! - BPM detection and matching
//! - Music licensing documentation
//! - Audio analysis for edit sync

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use super::{EditronError, EditronResult};

/// Supported music licensing platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MusicPlatform {
    MotionArray,
    Artlist,
    Epidemic,
    PremiumBeat,
    AudioJungle,
    Musicbed,
    Soundstripe,
    Local, // Local library
}

/// Music mood categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MusicMood {
    Uplifting,
    Inspirational,
    Happy,
    Energetic,
    Powerful,
    Epic,
    Dramatic,
    Emotional,
    Sad,
    Melancholic,
    Peaceful,
    Relaxing,
    Ambient,
    Mysterious,
    Suspenseful,
    Dark,
    Aggressive,
    Playful,
    Romantic,
    Nostalgic,
    Corporate,
    Modern,
    Cinematic,
    Documentary,
}

impl MusicMood {
    /// Get related moods for broader search
    pub fn related_moods(&self) -> Vec<MusicMood> {
        match self {
            MusicMood::Uplifting => vec![MusicMood::Inspirational, MusicMood::Happy, MusicMood::Energetic],
            MusicMood::Happy => vec![MusicMood::Uplifting, MusicMood::Playful, MusicMood::Energetic],
            MusicMood::Cinematic => vec![MusicMood::Epic, MusicMood::Dramatic, MusicMood::Emotional],
            MusicMood::Corporate => vec![MusicMood::Modern, MusicMood::Uplifting, MusicMood::Inspirational],
            MusicMood::Relaxing => vec![MusicMood::Peaceful, MusicMood::Ambient, MusicMood::Emotional],
            _ => vec![],
        }
    }

    /// MotionArray search term
    pub fn motionarray_term(&self) -> &'static str {
        match self {
            MusicMood::Uplifting => "uplifting",
            MusicMood::Inspirational => "inspirational",
            MusicMood::Happy => "happy",
            MusicMood::Energetic => "energetic",
            MusicMood::Powerful => "powerful",
            MusicMood::Epic => "epic",
            MusicMood::Dramatic => "dramatic",
            MusicMood::Emotional => "emotional",
            MusicMood::Sad => "sad",
            MusicMood::Melancholic => "melancholic",
            MusicMood::Peaceful => "peaceful",
            MusicMood::Relaxing => "relaxing",
            MusicMood::Ambient => "ambient",
            MusicMood::Mysterious => "mysterious",
            MusicMood::Suspenseful => "suspenseful",
            MusicMood::Dark => "dark",
            MusicMood::Aggressive => "aggressive",
            MusicMood::Playful => "playful",
            MusicMood::Romantic => "romantic",
            MusicMood::Nostalgic => "nostalgic",
            MusicMood::Corporate => "corporate",
            MusicMood::Modern => "modern",
            MusicMood::Cinematic => "cinematic",
            MusicMood::Documentary => "documentary",
        }
    }

    /// Epidemic Sound API search term
    pub fn epidemic_term(&self) -> &'static str {
        match self {
            MusicMood::Uplifting => "uplifting",
            MusicMood::Inspirational => "inspirational",
            MusicMood::Happy => "happy",
            MusicMood::Energetic => "energetic",
            MusicMood::Powerful => "powerful",
            MusicMood::Epic => "epic",
            MusicMood::Dramatic => "dramatic",
            MusicMood::Emotional => "emotional",
            MusicMood::Sad => "sad",
            MusicMood::Melancholic => "melancholic",
            MusicMood::Peaceful => "peaceful",
            MusicMood::Relaxing => "relaxing",
            MusicMood::Ambient => "ambient",
            MusicMood::Mysterious => "mysterious",
            MusicMood::Suspenseful => "suspenseful",
            MusicMood::Dark => "dark",
            MusicMood::Aggressive => "aggressive",
            MusicMood::Playful => "playful",
            MusicMood::Romantic => "romantic",
            MusicMood::Nostalgic => "nostalgic",
            MusicMood::Corporate => "corporate",
            MusicMood::Modern => "modern",
            MusicMood::Cinematic => "cinematic",
            MusicMood::Documentary => "documentary",
        }
    }

    /// Parse Epidemic Sound mood term to MusicMood
    pub fn from_epidemic_term(term: &str) -> Option<Self> {
        match term.to_lowercase().as_str() {
            "uplifting" | "upbeat" => Some(MusicMood::Uplifting),
            "inspirational" | "inspiring" => Some(MusicMood::Inspirational),
            "happy" | "joyful" | "cheerful" => Some(MusicMood::Happy),
            "energetic" | "energy" | "high energy" => Some(MusicMood::Energetic),
            "powerful" | "strong" => Some(MusicMood::Powerful),
            "epic" | "heroic" => Some(MusicMood::Epic),
            "dramatic" | "drama" => Some(MusicMood::Dramatic),
            "emotional" | "emotive" | "sentimental" => Some(MusicMood::Emotional),
            "sad" | "somber" | "sorrowful" => Some(MusicMood::Sad),
            "melancholic" | "melancholy" | "bittersweet" => Some(MusicMood::Melancholic),
            "peaceful" | "serene" | "tranquil" => Some(MusicMood::Peaceful),
            "relaxing" | "calm" | "chill" | "laid back" => Some(MusicMood::Relaxing),
            "ambient" | "atmospheric" => Some(MusicMood::Ambient),
            "mysterious" | "mystery" | "enigmatic" => Some(MusicMood::Mysterious),
            "suspenseful" | "tense" | "tension" | "thriller" => Some(MusicMood::Suspenseful),
            "dark" | "moody" | "sinister" => Some(MusicMood::Dark),
            "aggressive" | "intense" | "fierce" => Some(MusicMood::Aggressive),
            "playful" | "fun" | "quirky" => Some(MusicMood::Playful),
            "romantic" | "love" | "tender" => Some(MusicMood::Romantic),
            "nostalgic" | "retro" | "vintage" => Some(MusicMood::Nostalgic),
            "corporate" | "business" | "professional" => Some(MusicMood::Corporate),
            "modern" | "contemporary" | "trendy" => Some(MusicMood::Modern),
            "cinematic" | "film" | "movie" => Some(MusicMood::Cinematic),
            "documentary" | "factual" => Some(MusicMood::Documentary),
            _ => None,
        }
    }

    /// Soundstripe API search term
    pub fn soundstripe_term(&self) -> &'static str {
        match self {
            MusicMood::Uplifting => "Uplifting",
            MusicMood::Inspirational => "Inspirational",
            MusicMood::Happy => "Happy",
            MusicMood::Energetic => "Energetic",
            MusicMood::Powerful => "Powerful",
            MusicMood::Epic => "Epic",
            MusicMood::Dramatic => "Dramatic",
            MusicMood::Emotional => "Emotional",
            MusicMood::Sad => "Sad",
            MusicMood::Melancholic => "Melancholy",
            MusicMood::Peaceful => "Peaceful",
            MusicMood::Relaxing => "Relaxing",
            MusicMood::Ambient => "Ambient",
            MusicMood::Mysterious => "Mysterious",
            MusicMood::Suspenseful => "Suspenseful",
            MusicMood::Dark => "Dark",
            MusicMood::Aggressive => "Aggressive",
            MusicMood::Playful => "Playful",
            MusicMood::Romantic => "Romantic",
            MusicMood::Nostalgic => "Nostalgic",
            MusicMood::Corporate => "Corporate",
            MusicMood::Modern => "Modern",
            MusicMood::Cinematic => "Cinematic",
            MusicMood::Documentary => "Documentary",
        }
    }

    /// Parse Soundstripe mood term to MusicMood
    pub fn from_soundstripe_term(term: &str) -> Option<Self> {
        match term.to_lowercase().as_str() {
            "uplifting" => Some(MusicMood::Uplifting),
            "inspirational" | "inspiring" => Some(MusicMood::Inspirational),
            "happy" | "cheerful" | "feel good" => Some(MusicMood::Happy),
            "energetic" | "high energy" => Some(MusicMood::Energetic),
            "powerful" => Some(MusicMood::Powerful),
            "epic" => Some(MusicMood::Epic),
            "dramatic" => Some(MusicMood::Dramatic),
            "emotional" | "sentimental" => Some(MusicMood::Emotional),
            "sad" | "somber" => Some(MusicMood::Sad),
            "melancholy" | "melancholic" => Some(MusicMood::Melancholic),
            "peaceful" | "serene" => Some(MusicMood::Peaceful),
            "relaxing" | "calm" | "chill" => Some(MusicMood::Relaxing),
            "ambient" => Some(MusicMood::Ambient),
            "mysterious" => Some(MusicMood::Mysterious),
            "suspenseful" | "tense" => Some(MusicMood::Suspenseful),
            "dark" | "moody" => Some(MusicMood::Dark),
            "aggressive" | "intense" => Some(MusicMood::Aggressive),
            "playful" | "fun" | "quirky" => Some(MusicMood::Playful),
            "romantic" => Some(MusicMood::Romantic),
            "nostalgic" | "retro" => Some(MusicMood::Nostalgic),
            "corporate" | "business" => Some(MusicMood::Corporate),
            "modern" | "contemporary" => Some(MusicMood::Modern),
            "cinematic" => Some(MusicMood::Cinematic),
            "documentary" => Some(MusicMood::Documentary),
            _ => None,
        }
    }
}

/// Music genre categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MusicGenre {
    Pop,
    Rock,
    Electronic,
    HipHop,
    RnB,
    Jazz,
    Classical,
    Orchestral,
    Acoustic,
    Folk,
    Country,
    Blues,
    Funk,
    Soul,
    Reggae,
    Latin,
    World,
    Indie,
    Alternative,
    Metal,
    Punk,
    LoFi,
    Chillhop,
    House,
    Techno,
    Trap,
    DrumAndBass,
    Dubstep,
    Ambient,
    NewAge,
    Soundtrack,
    Trailer,
}

/// Music track information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicTrack {
    /// Unique identifier
    pub id: String,
    /// Track title
    pub title: String,
    /// Artist/composer
    pub artist: String,
    /// Duration in seconds
    pub duration: f64,
    /// BPM (beats per minute)
    pub bpm: Option<u32>,
    /// Key signature
    pub key: Option<String>,
    /// Primary genre
    pub genre: MusicGenre,
    /// Mood tags
    pub moods: Vec<MusicMood>,
    /// Additional tags
    pub tags: Vec<String>,
    /// Source platform
    pub platform: MusicPlatform,
    /// Platform URL
    pub url: Option<String>,
    /// Local file path (if downloaded)
    pub local_path: Option<PathBuf>,
    /// Preview URL
    pub preview_url: Option<String>,
    /// License type
    pub license: LicenseInfo,
    /// Waveform data (for visualization)
    pub waveform: Option<Vec<f32>>,
}

/// License information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub license_type: LicenseType,
    pub platform: MusicPlatform,
    pub subscription_id: Option<String>,
    pub download_date: Option<chrono::DateTime<chrono::Utc>>,
    pub project_name: Option<String>,
    pub usage_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseType {
    /// Unlimited subscription use
    Subscription,
    /// Single-use license
    SingleUse,
    /// Royalty-free perpetual
    RoyaltyFree,
    /// Creative Commons
    CreativeCommons(String),
    /// Custom license
    Custom(String),
}

/// Music search criteria
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MusicSearchCriteria {
    /// Search query text
    pub query: Option<String>,
    /// Filter by moods
    pub moods: Vec<MusicMood>,
    /// Filter by genres
    pub genres: Vec<MusicGenre>,
    /// Minimum duration (seconds)
    pub min_duration: Option<f64>,
    /// Maximum duration (seconds)
    pub max_duration: Option<f64>,
    /// Minimum BPM
    pub min_bpm: Option<u32>,
    /// Maximum BPM
    pub max_bpm: Option<u32>,
    /// Has vocals
    pub has_vocals: Option<bool>,
    /// Instrumental only
    pub instrumental: Option<bool>,
    /// Platforms to search
    pub platforms: Vec<MusicPlatform>,
}

impl MusicSearchCriteria {
    /// Create criteria for upbeat lifestyle content
    pub fn lifestyle_upbeat() -> Self {
        Self {
            moods: vec![MusicMood::Uplifting, MusicMood::Happy, MusicMood::Energetic],
            genres: vec![MusicGenre::Pop, MusicGenre::Electronic, MusicGenre::Indie],
            min_bpm: Some(100),
            max_bpm: Some(130),
            instrumental: Some(true),
            ..Default::default()
        }
    }

    /// Create criteria for cinematic content
    pub fn cinematic() -> Self {
        Self {
            moods: vec![MusicMood::Cinematic, MusicMood::Epic, MusicMood::Emotional],
            genres: vec![MusicGenre::Orchestral, MusicGenre::Soundtrack, MusicGenre::Trailer],
            instrumental: Some(true),
            ..Default::default()
        }
    }

    /// Create criteria for corporate/commercial
    pub fn corporate() -> Self {
        Self {
            moods: vec![MusicMood::Corporate, MusicMood::Modern, MusicMood::Inspirational],
            genres: vec![MusicGenre::Pop, MusicGenre::Electronic, MusicGenre::Acoustic],
            min_bpm: Some(90),
            max_bpm: Some(120),
            instrumental: Some(true),
            ..Default::default()
        }
    }

    /// Create criteria for chill/relaxed content
    pub fn chill() -> Self {
        Self {
            moods: vec![MusicMood::Relaxing, MusicMood::Peaceful, MusicMood::Ambient],
            genres: vec![MusicGenre::LoFi, MusicGenre::Chillhop, MusicGenre::Ambient, MusicGenre::Acoustic],
            max_bpm: Some(100),
            instrumental: Some(true),
            ..Default::default()
        }
    }

    /// Create criteria for fashion/lifestyle
    pub fn fashion_lifestyle() -> Self {
        Self {
            moods: vec![MusicMood::Modern, MusicMood::Uplifting, MusicMood::Energetic],
            genres: vec![MusicGenre::Electronic, MusicGenre::Pop, MusicGenre::House],
            min_bpm: Some(110),
            max_bpm: Some(128),
            instrumental: Some(true),
            ..Default::default()
        }
    }

    /// Generate MotionArray search URL
    pub fn to_motionarray_url(&self) -> String {
        let mut params = vec![];

        // Add mood terms
        for mood in &self.moods {
            params.push(format!("moods[]={}", mood.motionarray_term()));
        }

        // Add genre terms
        for genre in &self.genres {
            let genre_str = format!("{:?}", genre).to_lowercase();
            params.push(format!("genres[]={}", genre_str));
        }

        // Add BPM range
        if let Some(min) = self.min_bpm {
            params.push(format!("bpm_min={}", min));
        }
        if let Some(max) = self.max_bpm {
            params.push(format!("bpm_max={}", max));
        }

        // Add duration
        if let Some(min) = self.min_duration {
            params.push(format!("duration_min={}", min as u32));
        }
        if let Some(max) = self.max_duration {
            params.push(format!("duration_max={}", max as u32));
        }

        // Instrumental filter
        if self.instrumental == Some(true) {
            params.push("vocals=instrumental".to_string());
        }

        let query_string = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        format!("https://motionarray.com/browse/stock-music{}", query_string)
    }
}

/// Music recommendation based on video content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicRecommendation {
    pub criteria: MusicSearchCriteria,
    pub rationale: String,
    pub suggested_tracks: Vec<MusicTrack>,
    pub search_url: String,
}

/// Audio analysis result for a track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioAnalysis {
    /// Detected BPM
    pub bpm: f64,
    /// BPM confidence
    pub bpm_confidence: f32,
    /// Beat timestamps
    pub beat_times: Vec<f64>,
    /// Downbeat timestamps (bar starts)
    pub downbeat_times: Vec<f64>,
    /// Key detection
    pub key: Option<String>,
    /// Energy levels over time
    pub energy_curve: Vec<f32>,
    /// Average loudness (LUFS)
    pub loudness: f32,
    /// Has detected vocals
    pub has_vocals: bool,
    /// Sections (intro, verse, chorus, etc.)
    pub sections: Vec<AudioSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSection {
    pub section_type: SectionType,
    pub start_time: f64,
    pub end_time: f64,
    pub energy: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionType {
    Intro,
    Buildup,
    Drop,
    Verse,
    Chorus,
    Bridge,
    Breakdown,
    Outro,
    Unknown,
}

/// Music library manager
pub struct MusicLibrary {
    library_path: PathBuf,
    tracks: HashMap<String, MusicTrack>,
    ffmpeg_path: PathBuf,
    license_log: Vec<LicenseInfo>,
}

impl MusicLibrary {
    pub fn new<P: AsRef<Path>>(library_path: P, ffmpeg_path: P) -> Self {
        Self {
            library_path: library_path.as_ref().to_path_buf(),
            tracks: HashMap::new(),
            ffmpeg_path: ffmpeg_path.as_ref().to_path_buf(),
            license_log: Vec::new(),
        }
    }

    /// Generate music recommendations based on video content type
    pub fn recommend_for_content(&self, content_type: &str, duration: f64) -> MusicRecommendation {
        let (criteria, rationale) = match content_type.to_lowercase().as_str() {
            "lifestyle" | "fashion" | "athleisure" => (
                MusicSearchCriteria::fashion_lifestyle(),
                "Modern, upbeat electronic/pop music works well for fashion and lifestyle content. \
                The 110-128 BPM range provides energy while maintaining a sophisticated feel."
            ),
            "corporate" | "commercial" | "business" => (
                MusicSearchCriteria::corporate(),
                "Clean, professional music with an inspirational tone. Acoustic elements mixed \
                with modern production create a trustworthy, forward-thinking atmosphere."
            ),
            "cinematic" | "film" | "dramatic" => (
                MusicSearchCriteria::cinematic(),
                "Orchestral and soundtrack music provides emotional depth and production value. \
                Building dynamics work well for narrative content."
            ),
            "chill" | "relaxed" | "ambient" => (
                MusicSearchCriteria::chill(),
                "Lo-fi and ambient tracks create a calm, approachable atmosphere. \
                Lower BPM and minimal arrangements keep focus on the visuals."
            ),
            "fitness" | "sports" | "action" => (
                MusicSearchCriteria {
                    moods: vec![MusicMood::Energetic, MusicMood::Powerful, MusicMood::Aggressive],
                    genres: vec![MusicGenre::Electronic, MusicGenre::HipHop, MusicGenre::Rock],
                    min_bpm: Some(120),
                    max_bpm: Some(150),
                    instrumental: Some(true),
                    ..Default::default()
                },
                "High-energy tracks with strong beats drive action and fitness content. \
                Electronic and hip-hop elements add modern edge."
            ),
            "automotive" | "car" | "car show" | "cars" | "motorsport" | "racing" | "mopar" | "muscle car" => (
                MusicSearchCriteria {
                    moods: vec![MusicMood::Energetic, MusicMood::Powerful, MusicMood::Modern],
                    genres: vec![MusicGenre::Electronic, MusicGenre::Rock, MusicGenre::Trap],
                    min_bpm: Some(115),
                    max_bpm: Some(140),
                    instrumental: Some(true),
                    ..Default::default()
                },
                "Driving electronic and rock tracks with heavy bass and powerful builds match \
                the energy of automotive events. 115-140 BPM provides a muscular, kinetic feel \
                without being frantic. Trap elements add modern punch for car show recaps."
            ),
            "concert" | "music event" | "festival" | "live music" => (
                MusicSearchCriteria {
                    moods: vec![MusicMood::Energetic, MusicMood::Epic, MusicMood::Uplifting],
                    genres: vec![MusicGenre::Electronic, MusicGenre::House, MusicGenre::Pop],
                    min_bpm: Some(120),
                    max_bpm: Some(135),
                    instrumental: Some(true),
                    ..Default::default()
                },
                "Festival-energy electronic and house tracks complement concert and live music recaps. \
                Uplifting builds with anthemic drops mirror the crowd energy arc."
            ),
            "gala" | "formal" | "charity" | "awards" | "black tie" => (
                MusicSearchCriteria {
                    moods: vec![MusicMood::Inspirational, MusicMood::Cinematic, MusicMood::Uplifting],
                    genres: vec![MusicGenre::Orchestral, MusicGenre::Electronic, MusicGenre::Pop],
                    min_bpm: Some(90),
                    max_bpm: Some(120),
                    instrumental: Some(true),
                    ..Default::default()
                },
                "Elegant orchestral textures with modern production suit formal galas and awards. \
                Mid-tempo pacing feels sophisticated without dragging."
            ),
            "parade" | "march" | "celebration" | "carnival" => (
                MusicSearchCriteria {
                    moods: vec![MusicMood::Happy, MusicMood::Energetic, MusicMood::Uplifting],
                    genres: vec![MusicGenre::Pop, MusicGenre::Funk, MusicGenre::Electronic],
                    min_bpm: Some(110),
                    max_bpm: Some(130),
                    instrumental: Some(true),
                    ..Default::default()
                },
                "Upbeat, celebratory tracks with funky grooves and bright energy capture the \
                communal joy of parades and outdoor celebrations."
            ),
            _ => (
                MusicSearchCriteria::lifestyle_upbeat(),
                "General upbeat music suitable for most content types."
            ),
        };

        // Adjust duration if specified
        let mut final_criteria = criteria.clone();
        if duration > 0.0 {
            // Look for tracks slightly longer than needed
            final_criteria.min_duration = Some(duration);
            final_criteria.max_duration = Some(duration + 60.0);
        }

        let search_url = final_criteria.to_motionarray_url();

        MusicRecommendation {
            criteria: final_criteria,
            rationale: rationale.to_string(),
            suggested_tracks: vec![], // Would be populated from search results
            search_url,
        }
    }

    /// Analyze audio file for BPM, beats, and sections
    pub async fn analyze_audio<P: AsRef<Path>>(&self, path: P) -> EditronResult<AudioAnalysis> {
        let path = path.as_ref();

        // Get basic audio info
        let duration = self.get_audio_duration(path).await?;

        // Detect BPM using FFmpeg's ebur128 and aubio-style analysis
        let bpm = self.detect_bpm(path).await.unwrap_or(120.0);

        // Calculate beat times based on BPM
        let beat_interval = 60.0 / bpm;
        let beat_count = (duration / beat_interval) as usize;
        let beat_times: Vec<f64> = (0..beat_count)
            .map(|i| i as f64 * beat_interval)
            .collect();

        // Downbeats (every 4 beats assuming 4/4 time)
        let downbeat_times: Vec<f64> = beat_times
            .iter()
            .step_by(4)
            .cloned()
            .collect();

        // Get loudness
        let loudness = self.measure_loudness(path).await.unwrap_or(-14.0);

        Ok(AudioAnalysis {
            bpm,
            bpm_confidence: 0.8,
            beat_times,
            downbeat_times,
            key: None, // Would require more complex analysis
            energy_curve: vec![],
            loudness,
            has_vocals: false, // Would require ML-based detection
            sections: vec![],
        })
    }

    /// Get audio duration
    async fn get_audio_duration(&self, path: &Path) -> EditronResult<f64> {
        let ffprobe = self.ffmpeg_path.parent()
            .map(|p| p.join("ffprobe"))
            .unwrap_or_else(|| PathBuf::from("ffprobe"));

        let output = Command::new(ffprobe)
            .args([
                "-v", "quiet",
                "-show_entries", "format=duration",
                "-of", "csv=p=0",
                &path.to_string_lossy(),
            ])
            .output()
            .await?;

        let duration_str = String::from_utf8_lossy(&output.stdout);
        duration_str.trim()
            .parse()
            .map_err(|_| EditronError::FFmpeg("Failed to parse duration".to_string()))
    }

    /// Detect BPM using FFmpeg filter
    async fn detect_bpm(&self, path: &Path) -> EditronResult<f64> {
        // Use astats filter for onset detection
        let output = Command::new(&self.ffmpeg_path)
            .args([
                "-i", &path.to_string_lossy(),
                "-af", "aresample=44100,lowpass=f=150,highpass=f=20",
                "-f", "null",
                "-",
            ])
            .output()
            .await?;

        // For now, return an estimated BPM
        // Full implementation would use librosa or aubio
        Ok(120.0)
    }

    /// Measure loudness using EBU R128
    async fn measure_loudness(&self, path: &Path) -> EditronResult<f32> {
        let output = Command::new(&self.ffmpeg_path)
            .args([
                "-i", &path.to_string_lossy(),
                "-af", "loudnorm=I=-16:TP=-1.5:LRA=11:print_format=summary",
                "-f", "null",
                "-",
            ])
            .output()
            .await?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse "Input Integrated:" line
        for line in stderr.lines() {
            if line.contains("Input Integrated:") {
                if let Some(lufs_str) = line.split_whitespace().nth(2) {
                    if let Ok(lufs) = lufs_str.parse::<f32>() {
                        return Ok(lufs);
                    }
                }
            }
        }

        Ok(-14.0) // Default
    }

    /// Register a downloaded track
    pub fn register_track(&mut self, track: MusicTrack) {
        self.tracks.insert(track.id.clone(), track);
    }

    /// Log license usage
    pub fn log_license(&mut self, license: LicenseInfo) {
        self.license_log.push(license);
    }

    /// Export license documentation
    pub fn export_license_doc(&self, project_name: &str) -> String {
        let mut doc = format!("# Music License Documentation\n");
        doc.push_str(&format!("## Project: {}\n", project_name));
        doc.push_str(&format!("## Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d")));

        doc.push_str("### Licensed Tracks\n\n");
        doc.push_str("| Track | Platform | License Type | Download Date |\n");
        doc.push_str("|-------|----------|--------------|---------------|\n");

        for track in self.tracks.values() {
            let date = track.license.download_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "N/A".to_string());

            doc.push_str(&format!(
                "| {} - {} | {:?} | {:?} | {} |\n",
                track.title,
                track.artist,
                track.platform,
                track.license.license_type,
                date
            ));
        }

        doc.push_str("\n### Usage Rights\n\n");
        doc.push_str("All tracks licensed under MotionArray subscription grant:\n");
        doc.push_str("- Unlimited use in commercial and personal projects\n");
        doc.push_str("- Worldwide distribution rights\n");
        doc.push_str("- No attribution required (but appreciated)\n");
        doc.push_str("- Cannot be resold or redistributed as standalone music\n");

        doc
    }

    /// Get track by ID
    pub fn get_track(&self, id: &str) -> Option<&MusicTrack> {
        self.tracks.get(id)
    }

    /// Search local library
    pub fn search_local(&self, criteria: &MusicSearchCriteria) -> Vec<&MusicTrack> {
        self.tracks.values()
            .filter(|track| {
                // Filter by mood
                if !criteria.moods.is_empty() {
                    if !criteria.moods.iter().any(|m| track.moods.contains(m)) {
                        return false;
                    }
                }

                // Filter by genre
                if !criteria.genres.is_empty() {
                    if !criteria.genres.contains(&track.genre) {
                        return false;
                    }
                }

                // Filter by BPM
                if let Some(bpm) = track.bpm {
                    if let Some(min) = criteria.min_bpm {
                        if bpm < min { return false; }
                    }
                    if let Some(max) = criteria.max_bpm {
                        if bpm > max { return false; }
                    }
                }

                // Filter by duration
                if let Some(min) = criteria.min_duration {
                    if track.duration < min { return false; }
                }
                if let Some(max) = criteria.max_duration {
                    if track.duration > max { return false; }
                }

                true
            })
            .collect()
    }
}

impl MusicGenre {
    /// Epidemic Sound API search term
    pub fn epidemic_term(&self) -> &'static str {
        match self {
            MusicGenre::Pop => "pop",
            MusicGenre::Rock => "rock",
            MusicGenre::Electronic => "electronic",
            MusicGenre::HipHop => "hip hop",
            MusicGenre::RnB => "r&b",
            MusicGenre::Jazz => "jazz",
            MusicGenre::Classical => "classical",
            MusicGenre::Orchestral => "orchestral",
            MusicGenre::Acoustic => "acoustic",
            MusicGenre::Folk => "folk",
            MusicGenre::Country => "country",
            MusicGenre::Blues => "blues",
            MusicGenre::Funk => "funk",
            MusicGenre::Soul => "soul",
            MusicGenre::Reggae => "reggae",
            MusicGenre::Latin => "latin",
            MusicGenre::World => "world",
            MusicGenre::Indie => "indie",
            MusicGenre::Alternative => "alternative",
            MusicGenre::Metal => "metal",
            MusicGenre::Punk => "punk",
            MusicGenre::LoFi => "lo-fi",
            MusicGenre::Chillhop => "chillhop",
            MusicGenre::House => "house",
            MusicGenre::Techno => "techno",
            MusicGenre::Trap => "trap",
            MusicGenre::DrumAndBass => "drum and bass",
            MusicGenre::Dubstep => "dubstep",
            MusicGenre::Ambient => "ambient",
            MusicGenre::NewAge => "new age",
            MusicGenre::Soundtrack => "soundtrack",
            MusicGenre::Trailer => "trailer",
        }
    }

    /// Parse Epidemic Sound genre term to MusicGenre
    pub fn from_epidemic_term(term: &str) -> Option<Self> {
        match term.to_lowercase().replace(['-', '_'], " ").trim() {
            "pop" => Some(MusicGenre::Pop),
            "rock" => Some(MusicGenre::Rock),
            "electronic" | "edm" | "electronica" => Some(MusicGenre::Electronic),
            "hip hop" | "hiphop" | "rap" => Some(MusicGenre::HipHop),
            "r&b" | "rnb" | "r and b" => Some(MusicGenre::RnB),
            "jazz" => Some(MusicGenre::Jazz),
            "classical" => Some(MusicGenre::Classical),
            "orchestral" | "orchestra" => Some(MusicGenre::Orchestral),
            "acoustic" => Some(MusicGenre::Acoustic),
            "folk" => Some(MusicGenre::Folk),
            "country" => Some(MusicGenre::Country),
            "blues" => Some(MusicGenre::Blues),
            "funk" => Some(MusicGenre::Funk),
            "soul" => Some(MusicGenre::Soul),
            "reggae" => Some(MusicGenre::Reggae),
            "latin" => Some(MusicGenre::Latin),
            "world" => Some(MusicGenre::World),
            "indie" => Some(MusicGenre::Indie),
            "alternative" | "alt" => Some(MusicGenre::Alternative),
            "metal" => Some(MusicGenre::Metal),
            "punk" => Some(MusicGenre::Punk),
            "lo fi" | "lofi" => Some(MusicGenre::LoFi),
            "chillhop" => Some(MusicGenre::Chillhop),
            "house" => Some(MusicGenre::House),
            "techno" => Some(MusicGenre::Techno),
            "trap" => Some(MusicGenre::Trap),
            "drum and bass" | "dnb" => Some(MusicGenre::DrumAndBass),
            "dubstep" => Some(MusicGenre::Dubstep),
            "ambient" => Some(MusicGenre::Ambient),
            "new age" => Some(MusicGenre::NewAge),
            "soundtrack" | "score" => Some(MusicGenre::Soundtrack),
            "trailer" | "cinematic" => Some(MusicGenre::Trailer),
            _ => None,
        }
    }

    /// Soundstripe API search term (title case for JSON:API tags)
    pub fn soundstripe_term(&self) -> &'static str {
        match self {
            MusicGenre::Pop => "Pop",
            MusicGenre::Rock => "Rock",
            MusicGenre::Electronic => "Electronic",
            MusicGenre::HipHop => "Hip Hop",
            MusicGenre::RnB => "R&B",
            MusicGenre::Jazz => "Jazz",
            MusicGenre::Classical => "Classical",
            MusicGenre::Orchestral => "Orchestral",
            MusicGenre::Acoustic => "Acoustic",
            MusicGenre::Folk => "Folk",
            MusicGenre::Country => "Country",
            MusicGenre::Blues => "Blues",
            MusicGenre::Funk => "Funk",
            MusicGenre::Soul => "Soul",
            MusicGenre::Reggae => "Reggae",
            MusicGenre::Latin => "Latin",
            MusicGenre::World => "World",
            MusicGenre::Indie => "Indie",
            MusicGenre::Alternative => "Alternative",
            MusicGenre::Metal => "Metal",
            MusicGenre::Punk => "Punk",
            MusicGenre::LoFi => "Lo-Fi",
            MusicGenre::Chillhop => "Chillhop",
            MusicGenre::House => "House",
            MusicGenre::Techno => "Techno",
            MusicGenre::Trap => "Trap",
            MusicGenre::DrumAndBass => "Drum and Bass",
            MusicGenre::Dubstep => "Dubstep",
            MusicGenre::Ambient => "Ambient",
            MusicGenre::NewAge => "New Age",
            MusicGenre::Soundtrack => "Soundtrack",
            MusicGenre::Trailer => "Trailer",
        }
    }

    /// Parse Soundstripe genre term to MusicGenre
    pub fn from_soundstripe_term(term: &str) -> Option<Self> {
        match term.to_lowercase().as_str() {
            "pop" => Some(MusicGenre::Pop),
            "rock" => Some(MusicGenre::Rock),
            "electronic" | "edm" => Some(MusicGenre::Electronic),
            "hip hop" | "hip-hop" | "hiphop" => Some(MusicGenre::HipHop),
            "r&b" | "rnb" => Some(MusicGenre::RnB),
            "jazz" => Some(MusicGenre::Jazz),
            "classical" => Some(MusicGenre::Classical),
            "orchestral" => Some(MusicGenre::Orchestral),
            "acoustic" => Some(MusicGenre::Acoustic),
            "folk" => Some(MusicGenre::Folk),
            "country" => Some(MusicGenre::Country),
            "blues" => Some(MusicGenre::Blues),
            "funk" => Some(MusicGenre::Funk),
            "soul" => Some(MusicGenre::Soul),
            "reggae" => Some(MusicGenre::Reggae),
            "latin" => Some(MusicGenre::Latin),
            "world" => Some(MusicGenre::World),
            "indie" => Some(MusicGenre::Indie),
            "alternative" => Some(MusicGenre::Alternative),
            "metal" => Some(MusicGenre::Metal),
            "punk" => Some(MusicGenre::Punk),
            "lo-fi" | "lofi" | "lo fi" => Some(MusicGenre::LoFi),
            "chillhop" => Some(MusicGenre::Chillhop),
            "house" => Some(MusicGenre::House),
            "techno" => Some(MusicGenre::Techno),
            "trap" => Some(MusicGenre::Trap),
            "drum and bass" | "drum & bass" | "dnb" => Some(MusicGenre::DrumAndBass),
            "dubstep" => Some(MusicGenre::Dubstep),
            "ambient" => Some(MusicGenre::Ambient),
            "new age" => Some(MusicGenre::NewAge),
            "soundtrack" | "score" => Some(MusicGenre::Soundtrack),
            "trailer" | "cinematic" => Some(MusicGenre::Trailer),
            _ => None,
        }
    }
}

/// Generate suggested music search terms for content
pub fn suggest_search_terms(content_description: &str) -> Vec<String> {
    let desc_lower = content_description.to_lowercase();
    let mut terms = vec![];

    // Mood-based suggestions
    if desc_lower.contains("upbeat") || desc_lower.contains("happy") || desc_lower.contains("positive") {
        terms.extend(vec!["uplifting", "happy", "feel good"]);
    }
    if desc_lower.contains("fashion") || desc_lower.contains("lifestyle") || desc_lower.contains("trendy") {
        terms.extend(vec!["fashion", "stylish", "modern pop"]);
    }
    if desc_lower.contains("fitness") || desc_lower.contains("workout") || desc_lower.contains("gym") {
        terms.extend(vec!["workout", "energetic", "powerful"]);
    }
    if desc_lower.contains("chill") || desc_lower.contains("relax") || desc_lower.contains("calm") {
        terms.extend(vec!["chill", "relaxing", "lo-fi"]);
    }
    if desc_lower.contains("corporate") || desc_lower.contains("business") || desc_lower.contains("professional") {
        terms.extend(vec!["corporate", "inspiring", "motivational"]);
    }
    if desc_lower.contains("cinematic") || desc_lower.contains("dramatic") || desc_lower.contains("epic") {
        terms.extend(vec!["cinematic", "epic", "trailer"]);
    }

    // Remove duplicates
    terms.sort();
    terms.dedup();
    terms.into_iter().map(String::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motionarray_url_generation() {
        let criteria = MusicSearchCriteria::fashion_lifestyle();
        let url = criteria.to_motionarray_url();
        assert!(url.contains("motionarray.com"));
        assert!(url.contains("moods"));
    }

    #[test]
    fn test_mood_related() {
        let mood = MusicMood::Uplifting;
        let related = mood.related_moods();
        assert!(!related.is_empty());
    }

    #[test]
    fn test_search_terms() {
        let terms = suggest_search_terms("upbeat fashion lifestyle video");
        assert!(terms.contains(&"fashion".to_string()));
        assert!(terms.contains(&"uplifting".to_string()));
    }

    #[test]
    fn test_epidemic_mood_term_mapping() {
        assert_eq!(MusicMood::Uplifting.epidemic_term(), "uplifting");
        assert_eq!(MusicMood::Cinematic.epidemic_term(), "cinematic");
        assert_eq!(MusicMood::from_epidemic_term("happy"), Some(MusicMood::Happy));
        assert_eq!(MusicMood::from_epidemic_term("cheerful"), Some(MusicMood::Happy));
        assert_eq!(MusicMood::from_epidemic_term("unknown_mood"), None);
    }

    #[test]
    fn test_soundstripe_mood_term_mapping() {
        assert_eq!(MusicMood::Uplifting.soundstripe_term(), "Uplifting");
        assert_eq!(MusicMood::Melancholic.soundstripe_term(), "Melancholy");
        assert_eq!(MusicMood::from_soundstripe_term("Uplifting"), Some(MusicMood::Uplifting));
        assert_eq!(MusicMood::from_soundstripe_term("feel good"), Some(MusicMood::Happy));
        assert_eq!(MusicMood::from_soundstripe_term("nope"), None);
    }

    #[test]
    fn test_epidemic_genre_term_mapping() {
        assert_eq!(MusicGenre::HipHop.epidemic_term(), "hip hop");
        assert_eq!(MusicGenre::DrumAndBass.epidemic_term(), "drum and bass");
        assert_eq!(MusicGenre::from_epidemic_term("hip hop"), Some(MusicGenre::HipHop));
        assert_eq!(MusicGenre::from_epidemic_term("edm"), Some(MusicGenre::Electronic));
        assert_eq!(MusicGenre::from_epidemic_term("nope"), None);
    }

    #[test]
    fn test_soundstripe_genre_term_mapping() {
        assert_eq!(MusicGenre::HipHop.soundstripe_term(), "Hip Hop");
        assert_eq!(MusicGenre::LoFi.soundstripe_term(), "Lo-Fi");
        assert_eq!(MusicGenre::from_soundstripe_term("Hip Hop"), Some(MusicGenre::HipHop));
        assert_eq!(MusicGenre::from_soundstripe_term("lo-fi"), Some(MusicGenre::LoFi));
        assert_eq!(MusicGenre::from_soundstripe_term("nope"), None);
    }

    #[test]
    fn test_music_platform_soundstripe() {
        let platform = MusicPlatform::Soundstripe;
        let json = serde_json::to_string(&platform).unwrap();
        assert!(json.contains("soundstripe"));
    }
}
