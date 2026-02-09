//! Proxy Workflow Module for Editron
//!
//! Efficient editing workflow with proxy files:
//! - Generate lightweight proxy files from high-res footage
//! - Manage proxy/original file associations
//! - Automatic proxy switching for export
//! - Multiple proxy quality presets

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use super::{EditronError, EditronResult, VideoCodec, AudioCodec};

/// Proxy quality presets
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyPreset {
    /// 1/4 resolution, low bitrate (fastest editing)
    QuarterRes,
    /// 1/2 resolution, medium bitrate
    HalfRes,
    /// 720p fixed resolution
    Res720p,
    /// 1080p fixed resolution
    Res1080p,
    /// Custom settings
    Custom(ProxySettings),
}

impl ProxyPreset {
    pub fn settings(&self) -> ProxySettings {
        match self {
            ProxyPreset::QuarterRes => ProxySettings {
                scale_factor: Some(0.25),
                max_width: None,
                max_height: None,
                video_codec: VideoCodec::H264,
                video_bitrate: "2M".to_string(),
                audio_codec: AudioCodec::AAC,
                audio_bitrate: "128k".to_string(),
                frame_rate: None, // Keep original
                suffix: "_proxy_quarter".to_string(),
            },
            ProxyPreset::HalfRes => ProxySettings {
                scale_factor: Some(0.5),
                max_width: None,
                max_height: None,
                video_codec: VideoCodec::H264,
                video_bitrate: "5M".to_string(),
                audio_codec: AudioCodec::AAC,
                audio_bitrate: "192k".to_string(),
                frame_rate: None,
                suffix: "_proxy_half".to_string(),
            },
            ProxyPreset::Res720p => ProxySettings {
                scale_factor: None,
                max_width: Some(1280),
                max_height: Some(720),
                video_codec: VideoCodec::H264,
                video_bitrate: "5M".to_string(),
                audio_codec: AudioCodec::AAC,
                audio_bitrate: "192k".to_string(),
                frame_rate: None,
                suffix: "_proxy_720p".to_string(),
            },
            ProxyPreset::Res1080p => ProxySettings {
                scale_factor: None,
                max_width: Some(1920),
                max_height: Some(1080),
                video_codec: VideoCodec::H264,
                video_bitrate: "8M".to_string(),
                audio_codec: AudioCodec::AAC,
                audio_bitrate: "256k".to_string(),
                frame_rate: None,
                suffix: "_proxy_1080p".to_string(),
            },
            ProxyPreset::Custom(settings) => settings.clone(),
        }
    }
}

/// Detailed proxy generation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxySettings {
    /// Scale factor (e.g., 0.5 for half size)
    pub scale_factor: Option<f32>,
    /// Maximum width (if scale_factor not set)
    pub max_width: Option<u32>,
    /// Maximum height (if scale_factor not set)
    pub max_height: Option<u32>,
    /// Video codec
    pub video_codec: VideoCodec,
    /// Video bitrate
    pub video_bitrate: String,
    /// Audio codec
    pub audio_codec: AudioCodec,
    /// Audio bitrate
    pub audio_bitrate: String,
    /// Target frame rate (None = keep original)
    pub frame_rate: Option<f32>,
    /// Filename suffix
    pub suffix: String,
}

/// Proxy file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyFile {
    /// Original file path
    pub original: PathBuf,
    /// Proxy file path
    pub proxy: PathBuf,
    /// Preset used
    pub preset: ProxyPreset,
    /// Original resolution
    pub original_resolution: (u32, u32),
    /// Proxy resolution
    pub proxy_resolution: (u32, u32),
    /// Original file size
    pub original_size: u64,
    /// Proxy file size
    pub proxy_size: u64,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Proxy generation status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyStatus {
    Pending,
    InProgress { progress: f32 },
    Complete,
    Failed { error: String },
}

/// Proxy generation job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyJob {
    pub id: String,
    pub original: PathBuf,
    pub output: PathBuf,
    pub preset: ProxyPreset,
    pub status: ProxyStatus,
}

/// Proxy Workflow Manager
pub struct ProxyWorkflowManager {
    proxy_directory: PathBuf,
    ffmpeg_path: PathBuf,
    /// Maps original file to proxy info
    proxy_map: HashMap<PathBuf, ProxyFile>,
    /// Active jobs
    jobs: HashMap<String, ProxyJob>,
}

impl ProxyWorkflowManager {
    pub fn new<P: AsRef<Path>>(proxy_directory: P, ffmpeg_path: P) -> Self {
        Self {
            proxy_directory: proxy_directory.as_ref().to_path_buf(),
            ffmpeg_path: ffmpeg_path.as_ref().to_path_buf(),
            proxy_map: HashMap::new(),
            jobs: HashMap::new(),
        }
    }

    /// Generate proxy file path from original
    pub fn proxy_path(&self, original: &Path, preset: &ProxyPreset) -> PathBuf {
        let settings = preset.settings();
        let stem = original.file_stem().unwrap_or_default().to_string_lossy();
        let filename = format!("{}{}.mp4", stem, settings.suffix);
        self.proxy_directory.join(filename)
    }

    /// Generate FFmpeg command for proxy creation
    pub fn proxy_command(&self, original: &Path, preset: &ProxyPreset) -> (PathBuf, Vec<String>) {
        let settings = preset.settings();
        let output = self.proxy_path(original, preset);

        let mut args = vec![
            "-i".to_string(),
            original.to_string_lossy().to_string(),
            "-y".to_string(), // Overwrite
        ];

        // Video filter for scaling
        let mut vf = Vec::new();
        if let Some(factor) = settings.scale_factor {
            vf.push(format!("scale=iw*{}:ih*{}", factor, factor));
        } else if let (Some(w), Some(h)) = (settings.max_width, settings.max_height) {
            vf.push(format!("scale='min({},iw)':min'({},ih)':force_original_aspect_ratio=decrease", w, h));
        }

        if !vf.is_empty() {
            args.push("-vf".to_string());
            args.push(vf.join(","));
        }

        // Video codec
        args.push("-c:v".to_string());
        args.push(settings.video_codec.ffmpeg_codec().to_string());

        // Video bitrate
        args.push("-b:v".to_string());
        args.push(settings.video_bitrate.clone());

        // Preset for encoding speed
        args.push("-preset".to_string());
        args.push("fast".to_string());

        // Audio codec
        args.push("-c:a".to_string());
        args.push(settings.audio_codec.ffmpeg_codec().to_string());

        // Audio bitrate
        args.push("-b:a".to_string());
        args.push(settings.audio_bitrate.clone());

        // Frame rate if specified
        if let Some(fps) = settings.frame_rate {
            args.push("-r".to_string());
            args.push(fps.to_string());
        }

        // Output
        args.push(output.to_string_lossy().to_string());

        (output, args)
    }

    /// Generate proxy for a single file
    pub async fn generate_proxy(&self, original: &Path, preset: ProxyPreset) -> EditronResult<ProxyFile> {
        if !original.exists() {
            return Err(EditronError::FileNotFound(original.to_path_buf()));
        }

        // Create proxy directory if needed
        tokio::fs::create_dir_all(&self.proxy_directory).await?;

        let (output, args) = self.proxy_command(original, &preset);

        let result = Command::new(&self.ffmpeg_path)
            .args(&args)
            .output()
            .await?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            return Err(EditronError::FFmpeg(stderr.to_string()));
        }

        // Get file info
        let original_meta = tokio::fs::metadata(original).await?;
        let proxy_meta = tokio::fs::metadata(&output).await?;

        // Get video resolution via ffprobe (simplified)
        let original_res = self.get_resolution(original).await.unwrap_or((0, 0));
        let proxy_res = self.get_resolution(&output).await.unwrap_or((0, 0));

        Ok(ProxyFile {
            original: original.to_path_buf(),
            proxy: output,
            preset,
            original_resolution: original_res,
            proxy_resolution: proxy_res,
            original_size: original_meta.len(),
            proxy_size: proxy_meta.len(),
            created_at: chrono::Utc::now(),
        })
    }

    /// Generate proxies for multiple files in parallel
    pub async fn generate_proxies_batch(
        &self,
        files: Vec<PathBuf>,
        preset: ProxyPreset,
        max_concurrent: usize,
    ) -> Vec<EditronResult<ProxyFile>> {
        use futures::stream::{self, StreamExt};

        stream::iter(files)
            .map(|file| {
                let preset = preset.clone();
                async move {
                    self.generate_proxy(&file, preset).await
                }
            })
            .buffer_unordered(max_concurrent)
            .collect()
            .await
    }

    /// Get video resolution using ffprobe
    async fn get_resolution(&self, path: &Path) -> Option<(u32, u32)> {
        let ffprobe_path = self.ffmpeg_path.parent()?.join("ffprobe");

        let output = Command::new(ffprobe_path)
            .args([
                "-v", "quiet",
                "-select_streams", "v:0",
                "-show_entries", "stream=width,height",
                "-of", "csv=p=0",
                &path.to_string_lossy(),
            ])
            .output()
            .await
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split(',').collect();
        if parts.len() == 2 {
            let width = parts[0].parse().ok()?;
            let height = parts[1].parse().ok()?;
            Some((width, height))
        } else {
            None
        }
    }

    /// Check if proxy exists for a file
    pub fn has_proxy(&self, original: &Path) -> bool {
        self.proxy_map.contains_key(original)
    }

    /// Get proxy for original file
    pub fn get_proxy(&self, original: &Path) -> Option<&ProxyFile> {
        self.proxy_map.get(original)
    }

    /// Register a proxy file
    pub fn register_proxy(&mut self, proxy: ProxyFile) {
        self.proxy_map.insert(proxy.original.clone(), proxy);
    }

    /// Generate Premiere Pro proxy attachment script
    pub fn premiere_attach_proxies_script(&self, proxies: &[ProxyFile]) -> String {
        let mut script = String::from(r#"
// Attach Proxy Files to Original Media
var project = app.project;

function attachProxy(originalPath, proxyPath) {
    // Find the clip in project
    for (var i = 0; i < project.rootItem.children.numItems; i++) {
        var item = project.rootItem.children[i];
        if (item.getMediaPath && item.getMediaPath() === originalPath) {
            // Attach proxy
            item.attachProxy(proxyPath, 0);
            $.writeln("Attached proxy for: " + originalPath);
            return true;
        }
    }
    return false;
}

"#);

        for proxy in proxies {
            script.push_str(&format!(
                "attachProxy(\"{}\", \"{}\");\n",
                proxy.original.display(),
                proxy.proxy.display()
            ));
        }

        script.push_str("\n$.writeln(\"Proxy attachment complete\");\n");
        script
    }

    /// Scan directory for existing proxies
    pub async fn scan_existing_proxies(&mut self) -> EditronResult<usize> {
        let mut count = 0;

        if self.proxy_directory.exists() {
            let mut entries = tokio::fs::read_dir(&self.proxy_directory).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().map(|e| e == "mp4").unwrap_or(false) {
                    // Check if this is a proxy file by suffix
                    let filename = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");

                    for suffix in ["_proxy_quarter", "_proxy_half", "_proxy_720p", "_proxy_1080p"] {
                        if filename.ends_with(suffix) {
                            count += 1;
                            break;
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    /// Estimate proxy size reduction
    pub fn estimate_size_reduction(original_size: u64, preset: &ProxyPreset) -> u64 {
        let factor = match preset {
            ProxyPreset::QuarterRes => 0.05,  // ~5% of original
            ProxyPreset::HalfRes => 0.15,     // ~15% of original
            ProxyPreset::Res720p => 0.10,     // ~10% of original
            ProxyPreset::Res1080p => 0.20,    // ~20% of original
            ProxyPreset::Custom(_) => 0.15,   // Estimate
        };
        (original_size as f64 * factor) as u64
    }

    /// Get storage savings summary
    pub fn storage_summary(&self) -> (u64, u64, f32) {
        let mut original_total: u64 = 0;
        let mut proxy_total: u64 = 0;

        for proxy in self.proxy_map.values() {
            original_total += proxy.original_size;
            proxy_total += proxy.proxy_size;
        }

        let savings = if original_total > 0 {
            (1.0 - (proxy_total as f32 / original_total as f32)) * 100.0
        } else {
            0.0
        };

        (original_total, proxy_total, savings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_preset_settings() {
        let settings = ProxyPreset::HalfRes.settings();
        assert_eq!(settings.scale_factor, Some(0.5));
    }

    #[test]
    fn test_proxy_path_generation() {
        let manager = ProxyWorkflowManager::new("/tmp/proxies", "/usr/local/bin/ffmpeg");
        let path = manager.proxy_path(
            Path::new("/videos/test.mp4"),
            &ProxyPreset::HalfRes,
        );
        assert!(path.to_string_lossy().contains("_proxy_half"));
    }

    #[test]
    fn test_size_estimation() {
        let estimate = ProxyWorkflowManager::estimate_size_reduction(
            1_000_000_000, // 1GB
            &ProxyPreset::QuarterRes,
        );
        assert!(estimate < 100_000_000); // Should be less than 100MB
    }
}
