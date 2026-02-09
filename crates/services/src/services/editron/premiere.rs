//! Adobe Premiere Pro automation bridge
//!
//! Uses AppleScript/osascript to communicate with Premiere Pro on macOS.
//! ExtendScript commands are executed through the Adobe CEP extension system.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

use super::{EditronError, EditronResult};

/// Premiere Pro bridge for automation
pub struct PremiereProBridge {
    premiere_path: PathBuf,
}

impl PremiereProBridge {
    pub fn new() -> EditronResult<Self> {
        // Find Premiere Pro installation
        let premiere_path = Self::find_premiere()?;

        Ok(Self { premiere_path })
    }

    fn find_premiere() -> EditronResult<PathBuf> {
        // Check for Premiere Pro installations (prefer latest)
        let versions = ["2026", "2025", "2024", "2023"];

        for version in versions {
            let path = PathBuf::from(format!(
                "/Applications/Adobe Premiere Pro {}/Adobe Premiere Pro {}.app",
                version, version
            ));
            if path.exists() {
                return Ok(path);
            }
        }

        Err(EditronError::PremierePro(
            "Adobe Premiere Pro not found".to_string(),
        ))
    }

    /// Check if Premiere Pro is currently running
    pub async fn is_running(&self) -> EditronResult<bool> {
        let script = r#"
            tell application "System Events"
                set prRunning to (name of processes) contains "Adobe Premiere Pro"
            end tell
            return prRunning
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

    /// Launch Premiere Pro if not running
    pub async fn launch(&self) -> EditronResult<()> {
        if self.is_running().await? {
            return Ok(());
        }

        let script = format!(
            r#"tell application "{}" to activate"#,
            self.premiere_path.to_string_lossy()
        );

        Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        // Wait for Premiere to start
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        Ok(())
    }

    /// Open a Premiere Pro project file
    pub async fn open_project<P: AsRef<Path>>(&self, project: P) -> EditronResult<()> {
        let project = project.as_ref();
        if !project.exists() {
            return Err(EditronError::FileNotFound(project.to_path_buf()));
        }

        // Use open command to open project file with Premiere
        let status = Command::new("open")
            .arg("-a")
            .arg(&self.premiere_path)
            .arg(project)
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        if !status.success() {
            return Err(EditronError::PremierePro(
                "Failed to open project".to_string(),
            ));
        }

        Ok(())
    }

    /// Import media files into the current Premiere Pro project
    pub async fn import_media<P: AsRef<Path>>(&self, files: Vec<P>) -> EditronResult<()> {
        self.launch().await?;

        // Build file list for ExtendScript
        let file_list: Vec<String> = files
            .iter()
            .map(|f| format!("\"{}\"", f.as_ref().to_string_lossy()))
            .collect();

        let jsx_script = format!(
            r#"
            var files = [{}];
            var project = app.project;
            if (project) {{
                var rootItem = project.rootItem;
                for (var i = 0; i < files.length; i++) {{
                    var importResult = project.importFiles([files[i]], true, rootItem, false);
                }}
            }}
            "#,
            file_list.join(", ")
        );

        self.execute_jsx(&jsx_script).await
    }

    /// Create a new sequence with specified settings
    pub async fn create_sequence(
        &self,
        name: &str,
        width: u32,
        height: u32,
        frame_rate: f32,
    ) -> EditronResult<()> {
        self.launch().await?;

        let jsx_script = format!(
            r#"
            var project = app.project;
            if (project) {{
                var sequence = project.createNewSequence("{}", "sequenceID");
                if (sequence) {{
                    // Note: Full sequence settings require preset files
                    // This creates a basic sequence
                }}
            }}
            "#,
            name
        );

        self.execute_jsx(&jsx_script).await
    }

    /// Export current sequence via Adobe Media Encoder
    pub async fn queue_export(
        &self,
        preset_path: &str,
        output_path: &str,
    ) -> EditronResult<()> {
        let jsx_script = format!(
            r#"
            var sequence = app.project.activeSequence;
            if (sequence) {{
                var outputFile = new File("{}");
                var presetFile = new File("{}");
                if (presetFile.exists) {{
                    app.encoder.encodeSequence(
                        sequence,
                        outputFile.fsName,
                        presetFile.fsName,
                        app.encoder.ENCODE_WORKAREA,
                        1
                    );
                }}
            }}
            "#,
            output_path, preset_path
        );

        self.execute_jsx(&jsx_script).await
    }

    /// Get list of clips in current sequence
    pub async fn get_sequence_clips(&self) -> EditronResult<Vec<String>> {
        let jsx_script = r#"
            var result = [];
            var sequence = app.project.activeSequence;
            if (sequence) {
                var tracks = sequence.videoTracks;
                for (var i = 0; i < tracks.numTracks; i++) {
                    var track = tracks[i];
                    for (var j = 0; j < track.clips.numItems; j++) {
                        var clip = track.clips[j];
                        result.push(clip.name + "|" + clip.start.seconds + "|" + clip.end.seconds);
                    }
                }
            }
            result.join("\n");
        "#;

        let output = self.execute_jsx_with_result(jsx_script).await?;
        Ok(output.lines().map(|s| s.to_string()).collect())
    }

    /// Execute ExtendScript in Premiere Pro
    async fn execute_jsx(&self, script: &str) -> EditronResult<()> {
        // Write script to temp file
        let temp_path = std::env::temp_dir().join("editron_script.jsx");
        tokio::fs::write(&temp_path, script).await?;

        // Execute via osascript
        let applescript = format!(
            r#"
            tell application "Adobe Premiere Pro 2026"
                DoScript "{}"
            end tell
            "#,
            temp_path.to_string_lossy().replace("\"", "\\\"")
        );

        let status = Command::new("osascript")
            .arg("-e")
            .arg(&applescript)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        // Clean up
        let _ = tokio::fs::remove_file(&temp_path).await;

        if !status.success() {
            return Err(EditronError::PremierePro(
                "ExtendScript execution failed".to_string(),
            ));
        }

        Ok(())
    }

    /// Execute ExtendScript and return result
    async fn execute_jsx_with_result(&self, script: &str) -> EditronResult<String> {
        let temp_path = std::env::temp_dir().join("editron_script.jsx");
        let result_path = std::env::temp_dir().join("editron_result.txt");

        // Wrap script to write result to file
        let wrapped_script = format!(
            r#"
            var result = (function() {{
                {}
            }})();
            var f = new File("{}");
            f.open("w");
            f.write(result);
            f.close();
            "#,
            script,
            result_path.to_string_lossy().replace("\\", "/")
        );

        tokio::fs::write(&temp_path, &wrapped_script).await?;

        let applescript = format!(
            r#"
            tell application "Adobe Premiere Pro 2026"
                DoScript "{}"
            end tell
            "#,
            temp_path.to_string_lossy().replace("\"", "\\\"")
        );

        let status = Command::new("osascript")
            .arg("-e")
            .arg(&applescript)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        let _ = tokio::fs::remove_file(&temp_path).await;

        if !status.success() {
            return Err(EditronError::PremierePro(
                "ExtendScript execution failed".to_string(),
            ));
        }

        // Read result
        let result = tokio::fs::read_to_string(&result_path)
            .await
            .unwrap_or_default();

        let _ = tokio::fs::remove_file(&result_path).await;

        Ok(result)
    }

    /// Save current project
    pub async fn save_project(&self) -> EditronResult<()> {
        let jsx_script = r#"
            var project = app.project;
            if (project) {
                project.save();
            }
        "#;

        self.execute_jsx(jsx_script).await
    }

    /// Close current project
    pub async fn close_project(&self, save: bool) -> EditronResult<()> {
        let jsx_script = format!(
            r#"
            var project = app.project;
            if (project) {{
                project.closeDocument({});
            }}
            "#,
            if save { "1" } else { "0" }
        );

        self.execute_jsx(&jsx_script).await
    }
}
