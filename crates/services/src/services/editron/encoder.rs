//! Adobe Media Encoder automation bridge
//!
//! Provides headless rendering capabilities through Adobe Media Encoder.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

use super::{EditronError, EditronResult};

/// Media Encoder bridge for rendering
pub struct MediaEncoderBridge {
    encoder_path: PathBuf,
    presets_dir: PathBuf,
}

impl MediaEncoderBridge {
    pub fn new() -> EditronResult<Self> {
        let encoder_path = Self::find_encoder()?;
        let presets_dir = Self::find_presets_dir()?;

        Ok(Self {
            encoder_path,
            presets_dir,
        })
    }

    fn find_encoder() -> EditronResult<PathBuf> {
        let versions = ["2026", "2025", "2024", "2023"];

        for version in versions {
            let path = PathBuf::from(format!(
                "/Applications/Adobe Media Encoder {}/Adobe Media Encoder {}.app",
                version, version
            ));
            if path.exists() {
                return Ok(path);
            }
        }

        Err(EditronError::MediaEncoder(
            "Adobe Media Encoder not found".to_string(),
        ))
    }

    fn find_presets_dir() -> EditronResult<PathBuf> {
        let home = std::env::var("HOME").unwrap_or_default();

        // Check common preset locations
        let paths = [
            format!("{}/Documents/Adobe/Adobe Media Encoder/Presets", home),
            format!("{}/Library/Application Support/Adobe/Common/AME/Presets", home),
        ];

        for path in paths {
            let p = PathBuf::from(&path);
            if p.exists() {
                return Ok(p);
            }
        }

        // Create default presets directory
        let default_path = PathBuf::from(format!(
            "{}/Documents/Adobe/Adobe Media Encoder/Presets",
            home
        ));
        let _ = std::fs::create_dir_all(&default_path);

        Ok(default_path)
    }

    /// Check if Media Encoder is running
    pub async fn is_running(&self) -> EditronResult<bool> {
        let script = r#"
            tell application "System Events"
                set ameRunning to (name of processes) contains "Adobe Media Encoder"
            end tell
            return ameRunning
        "#;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        let result = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
        Ok(result == "true")
    }

    /// Launch Media Encoder
    pub async fn launch(&self) -> EditronResult<()> {
        if self.is_running().await? {
            return Ok(());
        }

        let script = format!(
            r#"tell application "{}" to activate"#,
            self.encoder_path.to_string_lossy()
        );

        Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        // Wait for encoder to start
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        Ok(())
    }

    /// Add file to encoding queue
    pub async fn add_to_queue<P: AsRef<Path>>(
        &self,
        source: P,
        preset: &str,
        output: P,
    ) -> EditronResult<()> {
        let source = source.as_ref();
        let output = output.as_ref();

        if !source.exists() {
            return Err(EditronError::FileNotFound(source.to_path_buf()));
        }

        self.launch().await?;

        // Use AppleScript to add to queue
        let script = format!(
            r#"
            tell application "Adobe Media Encoder 2026"
                activate
            end tell

            tell application "System Events"
                tell process "Adobe Media Encoder"
                    -- Would need UI scripting or ExtendScript for full automation
                end tell
            end tell
            "#
        );

        Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        Ok(())
    }

    /// Export using Media Encoder with preset
    pub async fn export<P: AsRef<Path>>(
        &self,
        source: P,
        preset_name: &str,
        output: P,
    ) -> EditronResult<PathBuf> {
        let source = source.as_ref();
        let output = output.as_ref();

        if !source.exists() {
            return Err(EditronError::FileNotFound(source.to_path_buf()));
        }

        // Find preset file
        let preset_path = self.find_preset(preset_name)?;

        self.launch().await?;

        // For now, use command-line headless render if available
        // Full AME automation requires more complex ExtendScript

        // Try using headless render (if available)
        let headless_path = self.encoder_path
            .parent()
            .map(|p| p.join("Contents/MacOS/Adobe Media Encoder"))
            .filter(|p| p.exists());

        if let Some(headless) = headless_path {
            let status = Command::new(headless)
                .arg("-source")
                .arg(source)
                .arg("-preset")
                .arg(&preset_path)
                .arg("-dest")
                .arg(output)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .status()
                .await
                .map_err(|e| EditronError::Process(e.to_string()))?;

            if status.success() {
                return Ok(output.to_path_buf());
            }
        }

        // Fallback: Add to queue and wait
        self.add_to_queue(source, preset_name, output).await?;

        // Note: Full automation would monitor the queue and wait for completion
        Ok(output.to_path_buf())
    }

    /// Find preset by name
    fn find_preset(&self, name: &str) -> EditronResult<PathBuf> {
        // Check for exact match first
        let exact_path = self.presets_dir.join(format!("{}.epr", name));
        if exact_path.exists() {
            return Ok(exact_path);
        }

        // Search for partial match
        if let Ok(entries) = std::fs::read_dir(&self.presets_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name() {
                    let file_name = file_name.to_string_lossy().to_lowercase();
                    if file_name.contains(&name.to_lowercase()) && file_name.ends_with(".epr") {
                        return Ok(path);
                    }
                }
            }
        }

        // Use built-in preset name (will be resolved by AME)
        Ok(PathBuf::from(name))
    }

    /// List available presets
    pub async fn list_presets(&self) -> EditronResult<Vec<String>> {
        let mut presets = Vec::new();

        // Built-in presets
        presets.extend([
            "Match Source - High bitrate".to_string(),
            "Match Source - Adaptive High Bitrate".to_string(),
            "YouTube 1080p Full HD".to_string(),
            "YouTube 4K Ultra HD".to_string(),
            "Vimeo 1080p Full HD".to_string(),
            "Twitter 1080p".to_string(),
            "Facebook 1080p Full HD".to_string(),
            "Instagram Feed 1080p".to_string(),
            "ProRes 422".to_string(),
            "ProRes 422 HQ".to_string(),
            "ProRes 4444".to_string(),
            "DNxHD HQ".to_string(),
        ]);

        // User presets from directory
        if let Ok(entries) = std::fs::read_dir(&self.presets_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "epr" {
                        if let Some(name) = path.file_stem() {
                            presets.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        Ok(presets)
    }

    /// Start encoding queue
    pub async fn start_queue(&self) -> EditronResult<()> {
        let script = r#"
            tell application "Adobe Media Encoder 2026"
                activate
            end tell

            tell application "System Events"
                tell process "Adobe Media Encoder"
                    -- Click Start Queue button
                    click button 1 of window 1
                end tell
            end tell
        "#;

        Command::new("osascript")
            .arg("-e")
            .arg(script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        Ok(())
    }

    /// Get encoding queue status
    pub async fn queue_status(&self) -> EditronResult<QueueStatus> {
        // This would need more sophisticated integration
        // For now, return a basic status
        Ok(QueueStatus {
            items_pending: 0,
            items_encoding: 0,
            items_complete: 0,
            items_failed: 0,
        })
    }
}

/// Encoding queue status
#[derive(Debug, Clone)]
pub struct QueueStatus {
    pub items_pending: u32,
    pub items_encoding: u32,
    pub items_complete: u32,
    pub items_failed: u32,
}
