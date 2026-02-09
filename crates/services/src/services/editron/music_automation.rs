//! Automated Music Selection Pipeline
//!
//! Automates the music selection process by:
//! - Analyzing video content for mood, energy, and pacing
//! - Generating optimal search criteria
//! - Browser automation for MotionArray
//! - Downloading and cataloging tracks
//! - Matching music to video edit points

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use super::{EditronError, EditronResult};
use super::music::{MusicMood, MusicGenre, MusicSearchCriteria, MusicTrack, MusicPlatform, LicenseInfo, LicenseType};
use super::scene_detection::{Scene, SceneDetectionResult};

/// Video content analysis for music matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoContentAnalysis {
    /// Overall energy level (0.0 - 1.0)
    pub energy_level: f32,
    /// Pacing/cut frequency
    pub pacing: PacingLevel,
    /// Detected visual moods
    pub visual_moods: Vec<MusicMood>,
    /// Color temperature (warm/cool)
    pub color_temperature: ColorTemperature,
    /// Motion intensity
    pub motion_intensity: f32,
    /// Content categories detected
    pub content_categories: Vec<String>,
    /// Suggested BPM range
    pub suggested_bpm: (u32, u32),
    /// Total duration
    pub duration: f64,
    /// Key moments (timestamps of high-energy scenes)
    pub key_moments: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PacingLevel {
    Slow,      // < 1 cut per 5 seconds
    Medium,    // 1-2 cuts per 5 seconds
    Fast,      // 2-4 cuts per 5 seconds
    VeryFast,  // > 4 cuts per 5 seconds
}

impl PacingLevel {
    pub fn from_cuts_per_minute(cpm: f32) -> Self {
        match cpm {
            x if x < 12.0 => PacingLevel::Slow,
            x if x < 24.0 => PacingLevel::Medium,
            x if x < 48.0 => PacingLevel::Fast,
            _ => PacingLevel::VeryFast,
        }
    }

    pub fn suggested_bpm_range(&self) -> (u32, u32) {
        match self {
            PacingLevel::Slow => (60, 90),
            PacingLevel::Medium => (90, 110),
            PacingLevel::Fast => (110, 130),
            PacingLevel::VeryFast => (130, 160),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorTemperature {
    Warm,
    Neutral,
    Cool,
}

/// Automated music selection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicSelectionResult {
    pub analysis: VideoContentAnalysis,
    pub search_criteria: MusicSearchCriteria,
    pub search_urls: HashMap<String, String>,
    pub recommended_tracks: Vec<RecommendedTrack>,
    pub automation_script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedTrack {
    pub title: String,
    pub artist: String,
    pub url: String,
    pub bpm: Option<u32>,
    pub duration: f64,
    pub match_score: f32,
    pub match_reasons: Vec<String>,
}

/// Browser automation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAutomationConfig {
    pub platform: MusicPlatform,
    pub headless: bool,
    pub download_path: PathBuf,
    pub max_results: usize,
    pub auto_download: bool,
    pub credentials: Option<PlatformCredentials>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCredentials {
    pub email: String,
    pub password_env_var: String, // Name of env var containing password
}

/// Music Automation Engine
pub struct MusicAutomationEngine {
    ffmpeg_path: PathBuf,
    download_path: PathBuf,
    catalog: Vec<MusicTrack>,
}

impl MusicAutomationEngine {
    pub fn new<P: AsRef<Path>>(ffmpeg_path: P, download_path: P) -> Self {
        Self {
            ffmpeg_path: ffmpeg_path.as_ref().to_path_buf(),
            download_path: download_path.as_ref().to_path_buf(),
            catalog: Vec::new(),
        }
    }

    /// Analyze video content to determine optimal music parameters
    pub async fn analyze_video_for_music<P: AsRef<Path>>(
        &self,
        video_path: P,
        scenes: Option<&SceneDetectionResult>,
    ) -> EditronResult<VideoContentAnalysis> {
        let video_path = video_path.as_ref();

        // Get video duration
        let duration = self.get_video_duration(video_path).await?;

        // Analyze motion/energy
        let (energy_level, motion_intensity) = self.analyze_motion_energy(video_path).await?;

        // Calculate pacing from scenes
        let (pacing, key_moments) = if let Some(scene_result) = scenes {
            let cuts_per_minute = scene_result.scenes.len() as f32 / (duration / 60.0) as f32;
            let pacing = PacingLevel::from_cuts_per_minute(cuts_per_minute);

            // Find high-energy moments (short cuts in succession)
            let key_moments: Vec<f64> = scene_result.scenes
                .windows(3)
                .filter(|w| {
                    // Look for rapid cuts
                    w.iter().all(|s| s.duration < 3.0)
                })
                .map(|w| w[1].start_time)
                .collect();

            (pacing, key_moments)
        } else {
            (PacingLevel::Medium, vec![])
        };

        // Analyze color temperature
        let color_temperature = self.analyze_color_temperature(video_path).await
            .unwrap_or(ColorTemperature::Neutral);

        // Determine visual moods based on analysis
        let visual_moods = self.determine_visual_moods(
            energy_level,
            &pacing,
            &color_temperature,
            motion_intensity,
        );

        // Detect content categories from filename/path
        let content_categories = self.detect_content_categories(video_path);

        // Calculate suggested BPM
        let base_bpm = pacing.suggested_bpm_range();
        let suggested_bpm = if energy_level > 0.7 {
            (base_bpm.0 + 10, base_bpm.1 + 10)
        } else if energy_level < 0.3 {
            (base_bpm.0.saturating_sub(10), base_bpm.1.saturating_sub(10))
        } else {
            base_bpm
        };

        Ok(VideoContentAnalysis {
            energy_level,
            pacing,
            visual_moods,
            color_temperature,
            motion_intensity,
            content_categories,
            suggested_bpm,
            duration,
            key_moments,
        })
    }

    /// Analyze multiple clips and aggregate results
    pub async fn analyze_clip_collection<P: AsRef<Path>>(
        &self,
        clips: &[P],
        scenes_map: &HashMap<PathBuf, SceneDetectionResult>,
    ) -> EditronResult<VideoContentAnalysis> {
        let mut total_energy = 0.0;
        let mut total_motion = 0.0;
        let mut total_duration = 0.0;
        let mut all_moods = Vec::new();
        let mut all_categories = Vec::new();
        let mut cut_count = 0;

        for clip in clips {
            let path = clip.as_ref().to_path_buf();
            let scenes = scenes_map.get(&path);

            if let Ok(analysis) = self.analyze_video_for_music(&path, scenes).await {
                total_energy += analysis.energy_level * analysis.duration as f32;
                total_motion += analysis.motion_intensity * analysis.duration as f32;
                total_duration += analysis.duration;
                all_moods.extend(analysis.visual_moods);
                all_categories.extend(analysis.content_categories);

                if let Some(s) = scenes {
                    cut_count += s.scenes.len();
                }
            }
        }

        // Weighted averages
        let avg_energy = if total_duration > 0.0 {
            total_energy / total_duration as f32
        } else {
            0.5
        };

        let avg_motion = if total_duration > 0.0 {
            total_motion / total_duration as f32
        } else {
            0.5
        };

        // Most common moods
        let mut mood_counts: HashMap<MusicMood, usize> = HashMap::new();
        for mood in &all_moods {
            *mood_counts.entry(mood.clone()).or_insert(0) += 1;
        }
        let mut mood_vec: Vec<_> = mood_counts.into_iter().collect();
        mood_vec.sort_by(|a, b| b.1.cmp(&a.1));
        let top_moods: Vec<MusicMood> = mood_vec.into_iter().take(3).map(|(m, _)| m).collect();

        // Calculate pacing
        let cuts_per_minute = cut_count as f32 / (total_duration / 60.0) as f32;
        let pacing = PacingLevel::from_cuts_per_minute(cuts_per_minute);
        let suggested_bpm = pacing.suggested_bpm_range();

        // Deduplicate categories
        all_categories.sort();
        all_categories.dedup();

        Ok(VideoContentAnalysis {
            energy_level: avg_energy,
            pacing,
            visual_moods: top_moods,
            color_temperature: ColorTemperature::Neutral, // Would need more analysis
            motion_intensity: avg_motion,
            content_categories: all_categories,
            suggested_bpm,
            duration: total_duration,
            key_moments: vec![],
        })
    }

    /// Generate optimal search criteria from video analysis
    pub fn generate_search_criteria(&self, analysis: &VideoContentAnalysis) -> MusicSearchCriteria {
        let mut criteria = MusicSearchCriteria::default();

        // Set moods from visual analysis
        criteria.moods = analysis.visual_moods.clone();

        // Determine genres based on content categories and moods
        criteria.genres = self.suggest_genres(&analysis.content_categories, &analysis.visual_moods);

        // Set BPM range
        criteria.min_bpm = Some(analysis.suggested_bpm.0);
        criteria.max_bpm = Some(analysis.suggested_bpm.1);

        // Set duration (slightly longer than video for editing flexibility)
        criteria.min_duration = Some(analysis.duration);
        criteria.max_duration = Some(analysis.duration + 60.0);

        // Most video content needs instrumental
        criteria.instrumental = Some(true);

        criteria
    }

    /// Suggest genres based on content and moods
    fn suggest_genres(&self, categories: &[String], moods: &[MusicMood]) -> Vec<MusicGenre> {
        let mut genres = Vec::new();

        for category in categories {
            match category.to_lowercase().as_str() {
                "fashion" | "lifestyle" | "beauty" => {
                    genres.extend(vec![MusicGenre::Electronic, MusicGenre::Pop, MusicGenre::House]);
                }
                "fitness" | "sports" | "workout" => {
                    genres.extend(vec![MusicGenre::Electronic, MusicGenre::HipHop, MusicGenre::Trap]);
                }
                "corporate" | "business" | "tech" => {
                    genres.extend(vec![MusicGenre::Electronic, MusicGenre::Ambient, MusicGenre::Pop]);
                }
                "travel" | "nature" | "documentary" => {
                    genres.extend(vec![MusicGenre::Acoustic, MusicGenre::World, MusicGenre::Orchestral]);
                }
                "food" | "cooking" | "restaurant" => {
                    genres.extend(vec![MusicGenre::Jazz, MusicGenre::Acoustic, MusicGenre::LoFi]);
                }
                "wedding" | "romantic" | "love" => {
                    genres.extend(vec![MusicGenre::Acoustic, MusicGenre::Classical, MusicGenre::Indie]);
                }
                _ => {}
            }
        }

        // Add genres based on moods
        for mood in moods {
            match mood {
                MusicMood::Epic | MusicMood::Cinematic => {
                    genres.push(MusicGenre::Orchestral);
                    genres.push(MusicGenre::Trailer);
                }
                MusicMood::Relaxing | MusicMood::Peaceful => {
                    genres.push(MusicGenre::Ambient);
                    genres.push(MusicGenre::LoFi);
                }
                MusicMood::Energetic | MusicMood::Powerful => {
                    genres.push(MusicGenre::Electronic);
                    genres.push(MusicGenre::Rock);
                }
                _ => {}
            }
        }

        // Deduplicate
        genres.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
        genres.dedup();
        genres.truncate(4); // Limit to top 4 genres

        if genres.is_empty() {
            genres = vec![MusicGenre::Pop, MusicGenre::Electronic];
        }

        genres
    }

    /// Generate browser automation script for MotionArray
    pub fn generate_motionarray_script(
        &self,
        criteria: &MusicSearchCriteria,
        config: &BrowserAutomationConfig,
    ) -> String {
        let search_url = criteria.to_motionarray_url();

        format!(r#"
// MotionArray Music Selection Automation Script
// Generated by Editron
// Platform: Playwright/Puppeteer compatible

const {{ chromium }} = require('playwright');

async function selectMusic() {{
    const browser = await chromium.launch({{ headless: {} }});
    const context = await browser.newContext({{
        userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36'
    }});
    const page = await context.newPage();

    // Navigate to MotionArray
    console.log('Navigating to MotionArray...');
    await page.goto('https://motionarray.com/login');

    // Login if credentials provided
    {}

    // Navigate to search
    console.log('Searching for music...');
    await page.goto('{}');
    await page.waitForSelector('.music-item', {{ timeout: 10000 }});

    // Collect track information
    const tracks = await page.$$eval('.music-item', items => {{
        return items.slice(0, {}).map(item => ({{
            title: item.querySelector('.track-title')?.textContent?.trim() || '',
            artist: item.querySelector('.track-artist')?.textContent?.trim() || '',
            duration: item.querySelector('.track-duration')?.textContent?.trim() || '',
            bpm: item.querySelector('.track-bpm')?.textContent?.trim() || '',
            url: item.querySelector('a')?.href || '',
            previewUrl: item.querySelector('audio')?.src || ''
        }}));
    }});

    console.log(`Found ${{tracks.length}} tracks`);

    // Save results
    const fs = require('fs');
    fs.writeFileSync(
        '{}',
        JSON.stringify(tracks, null, 2)
    );

    {}

    await browser.close();
    console.log('Done!');
}}

selectMusic().catch(console.error);
"#,
            config.headless,
            if config.credentials.is_some() {
                r#"
    // Login
    await page.fill('input[name="email"]', process.env.MOTIONARRAY_EMAIL);
    await page.fill('input[name="password"]', process.env.MOTIONARRAY_PASSWORD);
    await page.click('button[type="submit"]');
    await page.waitForNavigation();
"#
            } else {
                "// No auto-login configured"
            },
            search_url,
            config.max_results,
            config.download_path.join("search_results.json").display(),
            if config.auto_download {
                r#"
    // Auto-download top tracks
    for (const track of tracks.slice(0, 3)) {
        await page.goto(track.url);
        await page.click('.download-button');
        await page.waitForTimeout(2000);
    }
"#
            } else {
                "// Auto-download disabled"
            }
        )
    }

    /// Generate Python automation script (alternative to JS)
    pub fn generate_python_script(
        &self,
        criteria: &MusicSearchCriteria,
        config: &BrowserAutomationConfig,
    ) -> String {
        let search_url = criteria.to_motionarray_url();

        format!(r#"#!/usr/bin/env python3
"""
MotionArray Music Selection Automation
Generated by Editron
"""

import os
import json
import time
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.chrome.options import Options

def setup_driver(headless={}):
    options = Options()
    if headless:
        options.add_argument('--headless')
    options.add_argument('--disable-gpu')
    options.add_argument('--window-size=1920,1080')

    # Set download directory
    prefs = {{
        "download.default_directory": "{}",
        "download.prompt_for_download": False,
    }}
    options.add_experimental_option("prefs", prefs)

    return webdriver.Chrome(options=options)

def login(driver, email, password):
    driver.get('https://motionarray.com/login')

    email_field = WebDriverWait(driver, 10).until(
        EC.presence_of_element_located((By.NAME, "email"))
    )
    email_field.send_keys(email)

    password_field = driver.find_element(By.NAME, "password")
    password_field.send_keys(password)

    submit_btn = driver.find_element(By.CSS_SELECTOR, "button[type='submit']")
    submit_btn.click()

    time.sleep(3)  # Wait for login

def search_music(driver, url):
    driver.get(url)

    # Wait for results
    WebDriverWait(driver, 15).until(
        EC.presence_of_element_located((By.CSS_SELECTOR, ".music-item, .track-item, [data-track]"))
    )

    time.sleep(2)  # Let page fully load

    # Extract track info
    tracks = []
    items = driver.find_elements(By.CSS_SELECTOR, ".music-item, .track-item, [data-track]")

    for item in items[:{}]:
        try:
            track = {{
                'title': item.find_element(By.CSS_SELECTOR, ".track-title, .title").text,
                'artist': item.find_element(By.CSS_SELECTOR, ".track-artist, .artist").text,
                'url': item.find_element(By.TAG_NAME, "a").get_attribute("href"),
            }}

            # Try to get BPM and duration
            try:
                track['bpm'] = item.find_element(By.CSS_SELECTOR, ".bpm, [data-bpm]").text
            except:
                track['bpm'] = None

            try:
                track['duration'] = item.find_element(By.CSS_SELECTOR, ".duration, [data-duration]").text
            except:
                track['duration'] = None

            tracks.append(track)
        except Exception as e:
            print(f"Error extracting track: {{e}}")

    return tracks

def download_track(driver, track_url):
    driver.get(track_url)
    time.sleep(2)

    try:
        download_btn = WebDriverWait(driver, 10).until(
            EC.element_to_be_clickable((By.CSS_SELECTOR, ".download-button, [data-download]"))
        )
        download_btn.click()
        time.sleep(5)  # Wait for download
        return True
    except Exception as e:
        print(f"Download failed: {{e}}")
        return False

def main():
    driver = setup_driver(headless={})

    try:
        # Login if credentials available
        email = os.environ.get('MOTIONARRAY_EMAIL')
        password = os.environ.get('MOTIONARRAY_PASSWORD')

        if email and password:
            print("Logging in...")
            login(driver, email, password)

        # Search for music
        print("Searching for music...")
        search_url = "{}"
        tracks = search_music(driver, search_url)

        print(f"Found {{len(tracks)}} tracks")

        # Save results
        output_path = "{}"
        with open(output_path, 'w') as f:
            json.dump(tracks, f, indent=2)
        print(f"Results saved to {{output_path}}")

        # Download if enabled
        {}

    finally:
        driver.quit()

if __name__ == "__main__":
    main()
"#,
            config.headless.to_string().to_lowercase(),
            config.download_path.display(),
            config.max_results,
            config.headless.to_string().to_lowercase(),
            search_url,
            config.download_path.join("search_results.json").display(),
            if config.auto_download {
                r#"
        print("Downloading top tracks...")
        for track in tracks[:3]:
            if download_track(driver, track['url']):
                print(f"Downloaded: {track['title']}")
"#
            } else {
                "# Auto-download disabled"
            }
        )
    }

    /// Generate shell script for quick CLI automation
    pub fn generate_shell_script(
        &self,
        criteria: &MusicSearchCriteria,
        download_path: &Path,
    ) -> String {
        let search_url = criteria.to_motionarray_url();

        format!(r#"#!/bin/bash
# MotionArray Music Selection Helper
# Generated by Editron

DOWNLOAD_DIR="{}"
SEARCH_URL="{}"

echo "======================================"
echo "  MotionArray Music Selection Helper"
echo "======================================"
echo ""
echo "Recommended search criteria:"
echo "  Moods: {:?}"
echo "  Genres: {:?}"
echo "  BPM: {}-{}"
echo "  Duration: {}+ seconds"
echo ""
echo "Opening MotionArray in browser..."
echo ""

# Open search URL in default browser
if [[ "$OSTYPE" == "darwin"* ]]; then
    open "$SEARCH_URL"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    xdg-open "$SEARCH_URL"
fi

echo "Instructions:"
echo "1. Log in to your MotionArray account"
echo "2. Browse the search results"
echo "3. Click 'Download' on tracks you like"
echo "4. Move downloaded files to: $DOWNLOAD_DIR"
echo ""
echo "After downloading, run this to catalog:"
echo "  editron catalog-music $DOWNLOAD_DIR"
echo ""

# Create download directory
mkdir -p "$DOWNLOAD_DIR"

# Watch for new downloads (macOS)
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "Watching ~/Downloads for new music files..."
    echo "(Press Ctrl+C to stop)"
    echo ""

    fswatch -0 ~/Downloads | while read -d "" event; do
        if [[ "$event" == *.mp3 ]] || [[ "$event" == *.wav ]]; then
            echo "New file detected: $event"
            mv "$event" "$DOWNLOAD_DIR/"
            echo "Moved to $DOWNLOAD_DIR/"
        fi
    done
fi
"#,
            download_path.display(),
            search_url,
            criteria.moods,
            criteria.genres,
            criteria.min_bpm.unwrap_or(80),
            criteria.max_bpm.unwrap_or(140),
            criteria.min_duration.unwrap_or(60.0) as u32,
        )
    }

    /// Full automated music selection pipeline
    pub async fn run_automated_selection<P: AsRef<Path>>(
        &self,
        video_path: P,
        scenes: Option<&SceneDetectionResult>,
        config: BrowserAutomationConfig,
    ) -> EditronResult<MusicSelectionResult> {
        // Analyze video content
        let analysis = self.analyze_video_for_music(video_path, scenes).await?;

        // Generate search criteria
        let search_criteria = self.generate_search_criteria(&analysis);

        // Generate search URLs for different platforms
        let mut search_urls = HashMap::new();
        search_urls.insert(
            "motionarray".to_string(),
            search_criteria.to_motionarray_url(),
        );
        search_urls.insert(
            "artlist".to_string(),
            self.generate_artlist_url(&search_criteria),
        );

        // Generate automation script
        let automation_script = Some(match config.platform {
            MusicPlatform::MotionArray => {
                self.generate_python_script(&search_criteria, &config)
            }
            _ => self.generate_shell_script(&search_criteria, &config.download_path),
        });

        Ok(MusicSelectionResult {
            analysis,
            search_criteria,
            search_urls,
            recommended_tracks: vec![], // Would be populated after running script
            automation_script,
        })
    }

    // Helper methods

    async fn get_video_duration(&self, path: &Path) -> EditronResult<f64> {
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

        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .map_err(|_| EditronError::FFmpeg("Failed to parse duration".to_string()))
    }

    async fn analyze_motion_energy(&self, path: &Path) -> EditronResult<(f32, f32)> {
        // Use FFmpeg's mpdecimate filter to detect motion
        let output = Command::new(&self.ffmpeg_path)
            .args([
                "-i", &path.to_string_lossy(),
                "-vf", "mpdecimate,metadata=print:file=-",
                "-an", "-f", "null", "-",
            ])
            .output()
            .await?;

        // Parse motion values (simplified)
        // Full implementation would analyze frame differences
        Ok((0.6, 0.5)) // Default moderate energy/motion
    }

    async fn analyze_color_temperature(&self, path: &Path) -> EditronResult<ColorTemperature> {
        // Sample a frame and analyze color
        // Simplified - would need histogram analysis
        Ok(ColorTemperature::Neutral)
    }

    fn determine_visual_moods(
        &self,
        energy: f32,
        pacing: &PacingLevel,
        color_temp: &ColorTemperature,
        motion: f32,
    ) -> Vec<MusicMood> {
        let mut moods = Vec::new();

        // Energy-based moods
        if energy > 0.7 {
            moods.push(MusicMood::Energetic);
            moods.push(MusicMood::Powerful);
        } else if energy > 0.4 {
            moods.push(MusicMood::Uplifting);
            moods.push(MusicMood::Modern);
        } else {
            moods.push(MusicMood::Relaxing);
            moods.push(MusicMood::Peaceful);
        }

        // Pacing-based moods
        match pacing {
            PacingLevel::VeryFast => moods.push(MusicMood::Aggressive),
            PacingLevel::Fast => moods.push(MusicMood::Energetic),
            PacingLevel::Slow => moods.push(MusicMood::Emotional),
            _ => {}
        }

        // Color-based moods
        match color_temp {
            ColorTemperature::Warm => moods.push(MusicMood::Happy),
            ColorTemperature::Cool => moods.push(MusicMood::Mysterious),
            _ => {}
        }

        moods.truncate(3);
        moods
    }

    fn detect_content_categories(&self, path: &Path) -> Vec<String> {
        let path_str = path.to_string_lossy().to_lowercase();
        let mut categories = Vec::new();

        let keywords = [
            ("fashion", "fashion"),
            ("lifestyle", "lifestyle"),
            ("fitness", "fitness"),
            ("workout", "fitness"),
            ("gym", "fitness"),
            ("food", "food"),
            ("travel", "travel"),
            ("wedding", "wedding"),
            ("corporate", "corporate"),
            ("business", "corporate"),
            ("tech", "tech"),
            ("product", "product"),
            ("beauty", "beauty"),
            ("makeup", "beauty"),
        ];

        for (keyword, category) in keywords {
            if path_str.contains(keyword) {
                categories.push(category.to_string());
            }
        }

        if categories.is_empty() {
            categories.push("lifestyle".to_string());
        }

        categories
    }

    fn generate_artlist_url(&self, criteria: &MusicSearchCriteria) -> String {
        let mood_str = criteria.moods.iter()
            .map(|m| format!("{:?}", m).to_lowercase())
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "https://artlist.io/royalty-free-music?mood={}&instrumental=true",
            mood_str
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pacing_bpm_suggestion() {
        assert_eq!(PacingLevel::Slow.suggested_bpm_range(), (60, 90));
        assert_eq!(PacingLevel::Fast.suggested_bpm_range(), (110, 130));
    }

    #[test]
    fn test_pacing_from_cuts() {
        assert!(matches!(PacingLevel::from_cuts_per_minute(5.0), PacingLevel::Slow));
        assert!(matches!(PacingLevel::from_cuts_per_minute(30.0), PacingLevel::Fast));
    }
}
