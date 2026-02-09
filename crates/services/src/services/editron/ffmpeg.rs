//! FFmpeg wrapper for video processing operations

use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use serde_json::Value;

use super::{
    AudioCodec, EditOperation, EditronError, EditronResult, ExportPreset, TextPosition,
    VideoCodec, VideoMetadata,
};

/// FFmpeg client for video processing
pub struct FFmpegClient {
    ffmpeg_path: PathBuf,
    ffprobe_path: PathBuf,
}

impl FFmpegClient {
    pub fn new() -> EditronResult<Self> {
        // Try to find ffmpeg in common locations
        let ffmpeg_path = Self::find_executable("ffmpeg")?;
        let ffprobe_path = Self::find_executable("ffprobe")?;

        Ok(Self {
            ffmpeg_path,
            ffprobe_path,
        })
    }

    fn find_executable(name: &str) -> EditronResult<PathBuf> {
        // Check common paths
        let paths = [
            format!("{}/bin/{}", std::env::var("HOME").unwrap_or_default(), name),
            format!("{}/.local/bin/{}", std::env::var("HOME").unwrap_or_default(), name),
            format!("/usr/local/bin/{}", name),
            format!("/opt/homebrew/bin/{}", name),
            name.to_string(),
        ];

        for path in paths {
            let p = PathBuf::from(&path);
            if p.exists() {
                return Ok(p);
            }
        }

        // Try which command
        let output = std::process::Command::new("which")
            .arg(name)
            .output()
            .ok();

        if let Some(output) = output {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(PathBuf::from(path));
                }
            }
        }

        Err(EditronError::FFmpeg(format!("{} not found in PATH", name)))
    }

    /// Probe video file for metadata
    pub async fn probe<P: AsRef<Path>>(&self, path: P) -> EditronResult<VideoMetadata> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(EditronError::FileNotFound(path.to_path_buf()));
        }

        let output = Command::new(&self.ffprobe_path)
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(path)
            .output()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        if !output.status.success() {
            return Err(EditronError::FFmpeg(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let json: Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| EditronError::FFmpeg(e.to_string()))?;

        // Extract video stream info
        let streams = json["streams"].as_array()
            .ok_or_else(|| EditronError::FFmpeg("No streams found".to_string()))?;

        let video_stream = streams.iter()
            .find(|s| s["codec_type"].as_str() == Some("video"))
            .ok_or_else(|| EditronError::FFmpeg("No video stream found".to_string()))?;

        let audio_stream = streams.iter()
            .find(|s| s["codec_type"].as_str() == Some("audio"));

        let format = &json["format"];

        // Parse frame rate (can be "30/1" or "29.97")
        let frame_rate = video_stream["r_frame_rate"]
            .as_str()
            .map(|s| {
                if s.contains('/') {
                    let parts: Vec<&str> = s.split('/').collect();
                    if parts.len() == 2 {
                        let num: f32 = parts[0].parse().unwrap_or(30.0);
                        let den: f32 = parts[1].parse().unwrap_or(1.0);
                        num / den
                    } else {
                        30.0
                    }
                } else {
                    s.parse().unwrap_or(30.0)
                }
            })
            .unwrap_or(30.0);

        let file_size = tokio::fs::metadata(path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(VideoMetadata {
            path: path.to_path_buf(),
            duration_seconds: format["duration"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            width: video_stream["width"].as_u64().unwrap_or(0) as u32,
            height: video_stream["height"].as_u64().unwrap_or(0) as u32,
            frame_rate,
            codec: video_stream["codec_name"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            audio_codec: audio_stream
                .and_then(|s| s["codec_name"].as_str())
                .map(|s| s.to_string()),
            audio_channels: audio_stream
                .and_then(|s| s["channels"].as_u64())
                .map(|c| c as u32),
            audio_sample_rate: audio_stream
                .and_then(|s| s["sample_rate"].as_str())
                .and_then(|s| s.parse().ok()),
            bitrate: format["bit_rate"]
                .as_str()
                .and_then(|s| s.parse().ok()),
            file_size,
        })
    }

    /// Apply edit operations to video
    pub async fn apply_edits<P: AsRef<Path>>(
        &self,
        input: P,
        operations: Vec<EditOperation>,
        output: P,
        preset: Option<ExportPreset>,
    ) -> EditronResult<PathBuf> {
        let input = input.as_ref();
        let output = output.as_ref();

        if !input.exists() {
            return Err(EditronError::FileNotFound(input.to_path_buf()));
        }

        let mut args: Vec<String> = vec![
            "-y".to_string(),
            "-i".to_string(),
            input.to_string_lossy().to_string(),
        ];

        let mut filter_complex: Vec<String> = Vec::new();

        for op in operations {
            match op {
                EditOperation::Trim { start_seconds, end_seconds } => {
                    args.push("-ss".to_string());
                    args.push(format!("{:.3}", start_seconds));
                    args.push("-to".to_string());
                    args.push(format!("{:.3}", end_seconds));
                }
                EditOperation::Scale { width, height, maintain_aspect } => {
                    if maintain_aspect {
                        filter_complex.push(format!(
                            "scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2",
                            width, height, width, height
                        ));
                    } else {
                        filter_complex.push(format!("scale={}:{}", width, height));
                    }
                }
                EditOperation::Fade { fade_in_seconds, fade_out_seconds } => {
                    if let Some(fi) = fade_in_seconds {
                        filter_complex.push(format!("fade=t=in:st=0:d={:.2}", fi));
                    }
                    if let Some(fo) = fade_out_seconds {
                        // Will need duration for proper fade out
                        filter_complex.push(format!("fade=t=out:d={:.2}", fo));
                    }
                }
                EditOperation::Speed { factor, maintain_pitch } => {
                    filter_complex.push(format!("setpts={:.3}*PTS", 1.0 / factor));
                    if maintain_pitch {
                        filter_complex.push(format!("atempo={:.3}", factor));
                    } else {
                        filter_complex.push(format!("asetrate=44100*{:.3},aresample=44100", factor));
                    }
                }
                EditOperation::ColorCorrect { brightness, contrast, saturation, gamma } => {
                    filter_complex.push(format!(
                        "eq=brightness={:.2}:contrast={:.2}:saturation={:.2}:gamma={:.2}",
                        brightness, contrast, saturation, gamma
                    ));
                }
                EditOperation::TextOverlay { text, position, font_size, color, start_seconds, duration_seconds } => {
                    let (x, y) = match position {
                        TextPosition::TopLeft => ("10".to_string(), "10".to_string()),
                        TextPosition::TopCenter => ("(w-text_w)/2".to_string(), "10".to_string()),
                        TextPosition::TopRight => ("w-text_w-10".to_string(), "10".to_string()),
                        TextPosition::MiddleLeft => ("10".to_string(), "(h-text_h)/2".to_string()),
                        TextPosition::Center => ("(w-text_w)/2".to_string(), "(h-text_h)/2".to_string()),
                        TextPosition::MiddleRight => ("w-text_w-10".to_string(), "(h-text_h)/2".to_string()),
                        TextPosition::BottomLeft => ("10".to_string(), "h-text_h-10".to_string()),
                        TextPosition::BottomCenter => ("(w-text_w)/2".to_string(), "h-text_h-10".to_string()),
                        TextPosition::BottomRight => ("w-text_w-10".to_string(), "h-text_h-10".to_string()),
                        TextPosition::Custom { x, y } => (x.to_string(), y.to_string()),
                    };
                    filter_complex.push(format!(
                        "drawtext=text='{}':fontsize={}:fontcolor={}:x={}:y={}:enable='between(t,{:.2},{:.2})'",
                        text.replace("'", "\\'"),
                        font_size,
                        color,
                        x,
                        y,
                        start_seconds,
                        start_seconds + duration_seconds
                    ));
                }
                _ => {}
            }
        }

        if !filter_complex.is_empty() {
            args.push("-vf".to_string());
            args.push(filter_complex.join(","));
        }

        // Apply preset if provided
        if let Some(preset) = preset {
            self.apply_preset_args(&mut args, &preset);
        }

        args.push(output.to_string_lossy().to_string());

        let status = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        if !status.success() {
            return Err(EditronError::FFmpeg(format!(
                "FFmpeg failed with status: {}",
                status
            )));
        }

        Ok(output.to_path_buf())
    }

    /// Concatenate multiple videos
    pub async fn concat<P: AsRef<Path>>(
        &self,
        inputs: Vec<P>,
        output: P,
        preset: Option<ExportPreset>,
    ) -> EditronResult<PathBuf> {
        let output = output.as_ref();

        // Create concat file
        let concat_file = output.parent()
            .unwrap_or(Path::new("."))
            .join("concat_list.txt");

        let mut content = String::new();
        for input in &inputs {
            let path = input.as_ref();
            if !path.exists() {
                return Err(EditronError::FileNotFound(path.to_path_buf()));
            }
            content.push_str(&format!("file '{}'\n", path.to_string_lossy()));
        }
        tokio::fs::write(&concat_file, &content).await?;

        let mut args = vec![
            "-y".to_string(),
            "-f".to_string(),
            "concat".to_string(),
            "-safe".to_string(),
            "0".to_string(),
            "-i".to_string(),
            concat_file.to_string_lossy().to_string(),
        ];

        if let Some(preset) = preset {
            self.apply_preset_args(&mut args, &preset);
        } else {
            args.push("-c".to_string());
            args.push("copy".to_string());
        }

        args.push(output.to_string_lossy().to_string());

        let status = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        // Clean up concat file
        let _ = tokio::fs::remove_file(&concat_file).await;

        if !status.success() {
            return Err(EditronError::FFmpeg(format!(
                "FFmpeg concat failed with status: {}",
                status
            )));
        }

        Ok(output.to_path_buf())
    }

    /// Export video with preset
    pub async fn export<P: AsRef<Path>>(
        &self,
        input: P,
        output: P,
        preset: ExportPreset,
    ) -> EditronResult<PathBuf> {
        let input = input.as_ref();
        let output = output.as_ref();

        if !input.exists() {
            return Err(EditronError::FileNotFound(input.to_path_buf()));
        }

        let mut args = vec![
            "-y".to_string(),
            "-i".to_string(),
            input.to_string_lossy().to_string(),
        ];

        self.apply_preset_args(&mut args, &preset);
        args.push(output.to_string_lossy().to_string());

        let status = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        if !status.success() {
            return Err(EditronError::FFmpeg(format!(
                "FFmpeg export failed with status: {}",
                status
            )));
        }

        Ok(output.to_path_buf())
    }

    /// Extract single frame as image
    pub async fn extract_frame<P: AsRef<Path>>(
        &self,
        input: P,
        output: P,
        time_seconds: f64,
    ) -> EditronResult<PathBuf> {
        let input = input.as_ref();
        let output = output.as_ref();

        if !input.exists() {
            return Err(EditronError::FileNotFound(input.to_path_buf()));
        }

        let args = [
            "-y",
            "-ss", &format!("{:.3}", time_seconds),
            "-i", &input.to_string_lossy(),
            "-vframes", "1",
            "-q:v", "2",
            &output.to_string_lossy(),
        ];

        let status = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        if !status.success() {
            return Err(EditronError::FFmpeg("Frame extraction failed".to_string()));
        }

        Ok(output.to_path_buf())
    }

    /// Extract audio from video
    pub async fn extract_audio<P: AsRef<Path>>(
        &self,
        input: P,
        output: P,
        codec: AudioCodec,
    ) -> EditronResult<PathBuf> {
        let input = input.as_ref();
        let output = output.as_ref();

        if !input.exists() {
            return Err(EditronError::FileNotFound(input.to_path_buf()));
        }

        let args = [
            "-y",
            "-i", &input.to_string_lossy(),
            "-vn",
            "-acodec", codec.ffmpeg_codec(),
            &output.to_string_lossy(),
        ];

        let status = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        if !status.success() {
            return Err(EditronError::FFmpeg("Audio extraction failed".to_string()));
        }

        Ok(output.to_path_buf())
    }

    fn apply_preset_args(&self, args: &mut Vec<String>, preset: &ExportPreset) {
        // Video codec
        args.push("-c:v".to_string());
        args.push(preset.video_codec.ffmpeg_codec().to_string());

        // ProRes profile if applicable
        if let Some(profile) = preset.video_codec.prores_profile() {
            args.push("-profile:v".to_string());
            args.push(profile.to_string());
        }

        // Audio codec
        args.push("-c:a".to_string());
        args.push(preset.audio_codec.ffmpeg_codec().to_string());

        // Scale if specified
        if let (Some(w), Some(h)) = (preset.width, preset.height) {
            args.push("-vf".to_string());
            args.push(format!("scale={}:{}", w, h));
        }

        // Frame rate
        if let Some(fps) = preset.frame_rate {
            args.push("-r".to_string());
            args.push(format!("{:.2}", fps));
        }

        // Video bitrate
        if let Some(ref br) = preset.bitrate {
            args.push("-b:v".to_string());
            args.push(br.clone());
        }

        // Audio bitrate
        if let Some(ref abr) = preset.audio_bitrate {
            args.push("-b:a".to_string());
            args.push(abr.clone());
        }

        // Quality (CRF for x264/x265)
        if let Some(q) = preset.quality {
            match preset.video_codec {
                VideoCodec::H264 | VideoCodec::H265 => {
                    args.push("-crf".to_string());
                    args.push(q.to_string());
                }
                _ => {}
            }
        }
    }

    /// Get the FFmpeg executable path
    pub fn ffmpeg_path(&self) -> &Path {
        &self.ffmpeg_path
    }

    /// Apply a video filter to input
    pub async fn apply_filter<P: AsRef<Path>>(
        &self,
        input: P,
        filter: &str,
        output: P,
    ) -> EditronResult<PathBuf> {
        let input = input.as_ref();
        let output = output.as_ref();

        let status = Command::new(&self.ffmpeg_path)
            .args([
                "-i", &input.to_string_lossy(),
                "-vf", filter,
                "-c:a", "copy",
                "-y",
                &output.to_string_lossy(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        if !status.success() {
            return Err(EditronError::FFmpeg("Filter application failed".to_string()));
        }

        Ok(output.to_path_buf())
    }

    /// Apply an audio filter to input
    pub async fn apply_audio_filter<P: AsRef<Path>>(
        &self,
        input: P,
        filter: &str,
        output: P,
    ) -> EditronResult<PathBuf> {
        let input = input.as_ref();
        let output = output.as_ref();

        let status = Command::new(&self.ffmpeg_path)
            .args([
                "-i", &input.to_string_lossy(),
                "-af", filter,
                "-c:v", "copy",
                "-y",
                &output.to_string_lossy(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        if !status.success() {
            return Err(EditronError::FFmpeg("Audio filter application failed".to_string()));
        }

        Ok(output.to_path_buf())
    }

    /// Process video with both video and audio filters
    pub async fn process_with_filters<P: AsRef<Path>>(
        &self,
        input: P,
        video_filter: Option<&str>,
        audio_filter: Option<&str>,
        output: P,
        preset: Option<ExportPreset>,
    ) -> EditronResult<PathBuf> {
        let input = input.as_ref();
        let output = output.as_ref();

        let mut args = vec![
            "-i".to_string(),
            input.to_string_lossy().to_string(),
        ];

        // Add video filter
        if let Some(vf) = video_filter {
            args.push("-vf".to_string());
            args.push(vf.to_string());
        }

        // Add audio filter
        if let Some(af) = audio_filter {
            args.push("-af".to_string());
            args.push(af.to_string());
        }

        // Apply preset if provided
        if let Some(ref preset) = preset {
            self.apply_preset_args(&mut args, preset);
        } else {
            // Default codecs if no preset
            if video_filter.is_none() {
                args.push("-c:v".to_string());
                args.push("copy".to_string());
            } else {
                args.push("-c:v".to_string());
                args.push("libx264".to_string());
                args.push("-preset".to_string());
                args.push("medium".to_string());
            }

            if audio_filter.is_none() {
                args.push("-c:a".to_string());
                args.push("copy".to_string());
            } else {
                args.push("-c:a".to_string());
                args.push("aac".to_string());
            }
        }

        args.push("-y".to_string());
        args.push(output.to_string_lossy().to_string());

        let status = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| EditronError::Process(e.to_string()))?;

        if !status.success() {
            return Err(EditronError::FFmpeg("Video processing failed".to_string()));
        }

        Ok(output.to_path_buf())
    }
}
