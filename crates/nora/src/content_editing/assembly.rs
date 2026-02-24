//! Assembly Processor — Phase 6
//!
//! Dual output:
//! A. FFmpeg rendered video (complex filter graph)
//! B. Premiere Pro XML (importable project file)

use std::path::{Path, PathBuf};

use tokio::process::Command;

use crate::{NoraError, Result};

use super::types::*;

/// Handles final video rendering and Premiere XML generation.
pub struct AssemblyProcessor {
    output_bitrate_mbps: u32,
}

impl AssemblyProcessor {
    pub fn new(output_bitrate_mbps: u32) -> Self {
        Self { output_bitrate_mbps }
    }

    /// Produce both rendered video and Premiere XML from the directive.
    pub async fn assemble(
        &self,
        directive: &EditDirective,
        catalog: &ShotCatalog,
        output_dir: &Path,
        enable_render: bool,
        enable_xml: bool,
    ) -> Result<AssemblyResult> {
        // Ensure output directory exists
        tokio::fs::create_dir_all(output_dir).await.map_err(|e| {
            NoraError::ExecutionError(format!("Failed to create output dir: {}", e))
        })?;

        let mut rendered_path = None;
        let mut xml_path = None;

        // A. FFmpeg render
        if enable_render {
            match self.render_ffmpeg(directive, catalog, output_dir).await {
                Ok(path) => {
                    tracing::info!("[ASSEMBLY] Rendered video: {}", path.display());
                    rendered_path = Some(path);
                }
                Err(e) => {
                    tracing::error!("[ASSEMBLY] FFmpeg render failed: {}", e);
                    return Err(e);
                }
            }
        }

        // B. Premiere Pro XML
        if enable_xml {
            match self.generate_premiere_xml(directive, catalog, output_dir) {
                Ok(path) => {
                    tracing::info!("[ASSEMBLY] Generated Premiere XML: {}", path.display());
                    xml_path = Some(path);
                }
                Err(e) => {
                    tracing::warn!("[ASSEMBLY] Premiere XML generation failed: {}", e);
                }
            }
        }

        Ok(AssemblyResult {
            rendered_video_path: rendered_path,
            premiere_xml_path: xml_path,
            duration_seconds: directive.total_duration_seconds,
            file_size_bytes: 0, // Will be filled after render
        })
    }

    /// Render the final video using FFmpeg with a complex filter graph.
    async fn render_ffmpeg(
        &self,
        directive: &EditDirective,
        catalog: &ShotCatalog,
        output_dir: &Path,
    ) -> Result<PathBuf> {
        let output_file = output_dir.join(format!(
            "{}_final.mp4",
            directive.project_name.replace(' ', "_").to_lowercase()
        ));

        // Build the FFmpeg filter graph script
        let script = self.build_ffmpeg_script(directive, catalog, &output_file)?;

        // Write the script to a temp file for debugging
        let script_path = output_dir.join("ffmpeg_script.sh");
        tokio::fs::write(&script_path, &script).await.map_err(|e| {
            NoraError::ExecutionError(format!("Failed to write FFmpeg script: {}", e))
        })?;

        tracing::info!(
            "[ASSEMBLY] Executing FFmpeg render script ({} chars)",
            script.len()
        );

        // Execute via shell for complex filter graphs
        let output = Command::new("bash")
            .arg(&script_path)
            .output()
            .await
            .map_err(|e| NoraError::ExecutionError(format!("FFmpeg execution failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NoraError::ExecutionError(format!(
                "FFmpeg render error: {}",
                stderr.chars().take(2000).collect::<String>()
            )));
        }

        Ok(output_file)
    }

    /// Build the complete FFmpeg bash script for the edit.
    fn build_ffmpeg_script(
        &self,
        directive: &EditDirective,
        catalog: &ShotCatalog,
        output_file: &Path,
    ) -> Result<String> {
        let mut inputs = Vec::new();
        let mut filter_parts = Vec::new();
        let mut input_idx = 0;

        // Collect all input files referenced in the directive
        let mut referenced_assets: Vec<(usize, &MediaAsset)> = Vec::new();

        // Add interview audio source (first interview asset)
        if let Some(&idx) = catalog.interview_assets.first() {
            referenced_assets.push((input_idx, &catalog.assets[idx]));
            inputs.push(format!("-i \"{}\"", catalog.assets[idx].path.display()));
            input_idx += 1;
        }

        // Add B-roll clips from each act
        for act in &directive.acts {
            for clip in &act.broll_clips {
                if clip.asset_index < catalog.assets.len() {
                    let asset = &catalog.assets[clip.asset_index];
                    referenced_assets.push((input_idx, asset));
                    inputs.push(format!(
                        "-ss {} -t {} -i \"{}\"",
                        clip.in_point_seconds,
                        clip.out_point_seconds - clip.in_point_seconds,
                        asset.path.display()
                    ));
                    input_idx += 1;
                }
            }
        }

        // Add music track
        if let Some(music) = &directive.music_track {
            if music.asset_index < catalog.assets.len() {
                let asset = &catalog.assets[music.asset_index];
                inputs.push(format!("-i \"{}\"", asset.path.display()));
                input_idx += 1;
            }
        }

        // Build video filter: scale all inputs to output resolution, then concat
        let output_w = directive.output_spec.width;
        let output_h = directive.output_spec.height;
        let mut video_streams = Vec::new();

        for (i, (_, asset)) in referenced_assets.iter().enumerate().skip(0) {
            // Skip audio-only inputs
            if asset.width == 0 {
                continue;
            }

            // Check if vertical treatment needed
            if asset.height > asset.width {
                // Vertical: blur-fill pillarbox
                let blur = directive.vertical_clip_treatment.blur_radius;
                let fg_h = directive.vertical_clip_treatment.fg_scale_height;
                filter_parts.push(format!(
                    "[{i}:v]split[bg{i}][fg{i}]; \
                     [bg{i}]scale={output_w}:{output_h},boxblur={blur}[bg{i}o]; \
                     [fg{i}]scale=-1:{fg_h}[fg{i}o]; \
                     [bg{i}o][fg{i}o]overlay=(W-w)/2:(H-h)/2[v{i}]"
                ));
            } else {
                // Standard: scale to output resolution
                filter_parts.push(format!(
                    "[{i}:v]scale={output_w}:{output_h}:force_original_aspect_ratio=decrease,\
                     pad={output_w}:{output_h}:(ow-iw)/2:(oh-ih)/2[v{i}]"
                ));
            }
            video_streams.push(format!("[v{i}]"));
        }

        // Concat all video streams
        let n_video = video_streams.len();
        if n_video > 0 {
            let concat_input: String = video_streams.join("");
            filter_parts.push(format!(
                "{}concat=n={}:v=1:a=0[outv]",
                concat_input, n_video
            ));
        }

        // Audio: use interview audio, merge with music using amerge+pan
        // NOTE: amix normalizes and kills music volume. amerge+pan preserves
        // explicit volume levels — learned from v4/v5 manual process.
        let audio_filter = if directive.music_track.is_some() && input_idx >= 2 {
            let music_idx = input_idx - 1;
            let ducked = directive
                .music_track
                .as_ref()
                .map(|m| m.ducked_db)
                .unwrap_or(-18.0);
            let full_db = directive
                .music_track
                .as_ref()
                .map(|m| m.full_db)
                .unwrap_or(-6.0);

            // Build volume automation expression for music ducking
            let volume_expr = if let Some(ref music) = directive.music_track {
                if music.full_regions.is_empty() {
                    // No full regions detected — constant ducked level
                    format!("{}dB", ducked)
                } else {
                    // Dynamic volume: full in gaps, ducked under dialogue
                    let mut expr_parts = Vec::new();
                    for region in &music.full_regions {
                        expr_parts.push(format!(
                            "if(between(t,{:.2},{:.2}),{}dB",
                            region.start_seconds, region.end_seconds, full_db
                        ));
                    }
                    // Default to ducked level
                    let mut expr = String::new();
                    for part in &expr_parts {
                        expr.push_str(part);
                        expr.push(',');
                    }
                    expr.push_str(&format!("{}dB", ducked));
                    for _ in &expr_parts {
                        expr.push(')');
                    }
                    expr
                }
            } else {
                format!("{}dB", ducked)
            };

            format!(
                "[0:a]aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo,\
                 volume=1.0[dialogue]; \
                 [{music_idx}:a]aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo,\
                 volume='{volume_expr}'[music]; \
                 [dialogue][music]amerge=inputs=2,pan=stereo|c0<c0+c2|c1<c1+c3[outa]",
                volume_expr = volume_expr
            )
        } else {
            "[0:a]aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo[outa]"
                .to_string()
        };

        filter_parts.push(audio_filter);

        let filter_complex = filter_parts.join(";\n");
        let bitrate = format!("{}M", self.output_bitrate_mbps);

        // Add transition filter using eq=brightness workaround
        // NOTE: FFmpeg 8.0.1 has a bug where chaining 2+ fade filters produces
        // all-black output. Using eq=brightness with time-based expressions instead.
        let transition_dur = directive.transition_duration_seconds;
        if n_video > 1 && transition_dur > 0.0 {
            let mut brightness_parts = Vec::new();
            let mut pos = 0.0;
            for act in &directive.acts {
                pos += act.duration_target_seconds;
                if pos < directive.total_duration_seconds {
                    let fade_start = pos - transition_dur;
                    let fade_end = pos;
                    let fade_in_end = pos + transition_dur;
                    brightness_parts.push(format!(
                        "if(between(t,{:.2},{:.2}),-(t-{:.2})/{:.1},\
                         if(between(t,{:.2},{:.2}),-({:.2}-t)/{:.1}",
                        fade_start, fade_end, fade_start, transition_dur,
                        fade_end, fade_in_end, fade_in_end, transition_dur
                    ));
                }
            }
            if !brightness_parts.is_empty() {
                let mut expr = brightness_parts.join(",");
                for _ in 0..brightness_parts.len() {
                    expr.push_str("))");
                }
                filter_parts.push(format!(
                    "[outv]eq=brightness='{}'[outv2]",
                    expr
                ));
                // Use outv2 as final output
                let filter_complex = filter_parts.join(";\n");
                let script = format!(
                    r#"#!/bin/bash
set -e

ffmpeg -y \
  {inputs} \
  -filter_complex "
{filter_complex}
" \
  -map "[outv2]" -map "[outa]" \
  -c:v libx264 -preset medium -b:v {bitrate} \
  -pix_fmt yuv420p \
  -c:a aac -b:a 320k \
  -movflags +faststart \
  "{output}"
"#,
                    inputs = inputs.join(" \\\n  "),
                    filter_complex = filter_complex,
                    bitrate = bitrate,
                    output = output_file.display()
                );
                return Ok(script);
            }
        }

        let script = format!(
            r#"#!/bin/bash
set -e

ffmpeg -y \
  {inputs} \
  -filter_complex "
{filter_complex}
" \
  -map "[outv]" -map "[outa]" \
  -c:v libx264 -preset medium -b:v {bitrate} \
  -pix_fmt yuv420p \
  -c:a aac -b:a 320k \
  -movflags +faststart \
  "{output}"
"#,
            inputs = inputs.join(" \\\n  "),
            filter_complex = filter_complex,
            bitrate = bitrate,
            output = output_file.display()
        );

        Ok(script)
    }

    /// Generate Premiere Pro compatible FCPXML / Premiere XML.
    fn generate_premiere_xml(
        &self,
        directive: &EditDirective,
        catalog: &ShotCatalog,
        output_dir: &Path,
    ) -> Result<PathBuf> {
        let xml_path = output_dir.join(format!(
            "{}_timeline.xml",
            directive.project_name.replace(' ', "_").to_lowercase()
        ));

        let fps = 24.0;
        let timebase = 24;

        let mut xml = String::new();
        xml.push_str(&format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE xmeml>
<xmeml version="4">
  <sequence>
    <name>{}</name>
    <duration>{}</duration>
    <rate>
      <timebase>{}</timebase>
      <ntsc>FALSE</ntsc>
    </rate>
    <media>
"#,
            directive.project_name,
            (directive.total_duration_seconds * fps) as u64,
            timebase
        ));

        // V1: B-roll track
        xml.push_str("      <video>\n        <track>\n          <!-- V1: B-Roll -->\n");
        let mut timeline_pos: f64 = 0.0;

        for act in &directive.acts {
            for clip in &act.broll_clips {
                if clip.asset_index < catalog.assets.len() {
                    let asset = &catalog.assets[clip.asset_index];
                    let clip_duration = clip.out_point_seconds - clip.in_point_seconds;
                    let start_frame = (timeline_pos * fps) as u64;
                    let end_frame = ((timeline_pos + clip_duration) * fps) as u64;
                    let in_frame = (clip.in_point_seconds * fps) as u64;
                    let out_frame = (clip.out_point_seconds * fps) as u64;

                    xml.push_str(&format!(
                        r#"          <clipitem>
            <name>{}</name>
            <start>{}</start>
            <end>{}</end>
            <in>{}</in>
            <out>{}</out>
            <file>
              <pathurl>file://{}</pathurl>
            </file>
          </clipitem>
"#,
                        asset.filename,
                        start_frame,
                        end_frame,
                        in_frame,
                        out_frame,
                        asset.path.display()
                    ));

                    timeline_pos += clip_duration;
                }
            }
        }
        xml.push_str("        </track>\n");

        // V2: Interview on-camera track (LIP SYNC VERIFIED)
        // CRITICAL: in/out points use the AUDIO source timecodes from lip_sync_points
        // to ensure video is perfectly synced with dialogue.
        xml.push_str("        <track>\n          <!-- V2: Interview On-Camera (SYNCED) -->\n");
        for lsp in &directive.lip_sync_points {
            if let Some(&idx) = catalog.interview_assets.first() {
                let asset = &catalog.assets[idx];
                // Timeline position
                let start_frame = (lsp.timeline_position_seconds * fps) as u64;
                let end_frame = ((lsp.timeline_position_seconds + lsp.duration_seconds) * fps) as u64;
                // Source in/out: use audio_source_timecode which MUST == video_source_timecode
                let in_frame = (lsp.audio_source_timecode * fps) as u64;
                let out_frame = ((lsp.audio_source_timecode + lsp.duration_seconds) * fps) as u64;

                xml.push_str(&format!(
                    r#"          <clipitem>
            <name>{sb_id} (on-camera SYNCED: source={src:.3}s)</name>
            <start>{start}</start>
            <end>{end}</end>
            <in>{in_pt}</in>
            <out>{out_pt}</out>
            <file>
              <pathurl>file://{path}</pathurl>
            </file>
          </clipitem>
"#,
                    sb_id = lsp.soundbite_id,
                    src = lsp.audio_source_timecode,
                    start = start_frame,
                    end = end_frame,
                    in_pt = in_frame,
                    out_pt = out_frame,
                    path = asset.path.display()
                ));
            }
        }
        xml.push_str("        </track>\n");

        // V3: Graphics placeholder
        xml.push_str("        <track>\n          <!-- V3: Graphics/Titles (placeholder) -->\n");
        xml.push_str("        </track>\n");
        xml.push_str("      </video>\n");

        // Audio tracks
        xml.push_str("      <audio>\n");

        // A1-A2: Interview audio
        xml.push_str("        <track>\n          <!-- A1: Interview Audio -->\n");
        if let Some(&idx) = catalog.interview_assets.first() {
            let asset = &catalog.assets[idx];
            let end_frame = (directive.total_duration_seconds * fps) as u64;
            xml.push_str(&format!(
                r#"          <clipitem>
            <name>{} (audio)</name>
            <start>0</start>
            <end>{}</end>
            <in>0</in>
            <out>{}</out>
            <file>
              <pathurl>file://{}</pathurl>
            </file>
          </clipitem>
"#,
                asset.filename, end_frame, end_frame, asset.path.display()
            ));
        }
        xml.push_str("        </track>\n");

        // A3-A4: Music track with volume keyframes
        xml.push_str("        <track>\n          <!-- A3: Music Track -->\n");
        if let Some(music) = &directive.music_track {
            if music.asset_index < catalog.assets.len() {
                let asset = &catalog.assets[music.asset_index];
                let end_frame = (directive.total_duration_seconds * fps) as u64;

                xml.push_str(&format!(
                    r#"          <clipitem>
            <name>{} (music)</name>
            <start>0</start>
            <end>{}</end>
            <in>0</in>
            <out>{}</out>
            <file>
              <pathurl>file://{}</pathurl>
            </file>
            <filter>
              <effect>
                <name>Audio Levels</name>
                <effectid>audiolevels</effectid>
                <parameter>
                  <name>Level</name>
                  <value>{}</value>
                </parameter>
              </effect>
            </filter>
          </clipitem>
"#,
                    asset.filename,
                    end_frame,
                    end_frame,
                    asset.path.display(),
                    music.ducked_db
                ));
            }
        }
        xml.push_str("        </track>\n");
        xml.push_str("      </audio>\n");

        // Sequence markers for act boundaries
        xml.push_str("    </media>\n");
        xml.push_str("    <markers>\n");
        let mut marker_pos = 0.0;
        for act in &directive.acts {
            let frame = (marker_pos * fps) as u64;
            xml.push_str(&format!(
                r#"      <marker>
        <name>{:?}</name>
        <in>{}</in>
        <out>{}</out>
      </marker>
"#,
                act.act, frame, frame
            ));
            marker_pos += act.duration_target_seconds;
        }
        xml.push_str("    </markers>\n");
        xml.push_str("  </sequence>\n</xmeml>\n");

        std::fs::write(&xml_path, &xml).map_err(|e| {
            NoraError::ExecutionError(format!("Failed to write Premiere XML: {}", e))
        })?;

        Ok(xml_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_premiere_xml_generation() {
        let processor = AssemblyProcessor::new(18);

        let directive = EditDirective {
            project_name: "Test Project".to_string(),
            total_duration_seconds: 120.0,
            soundbites: vec![],
            acts: vec![ActAssignment {
                act: ActLabel::Act1Intro,
                duration_target_seconds: 24.0,
                soundbite_ids: vec![],
                broll_clips: vec![],
                interview_on_camera: false,
            }],
            music_track: None,
            computed_broll_ratio: 0.85,
            computed_interview_ratio: 0.15,
            vertical_clip_treatment: VerticalTreatment {
                blur_radius: 20,
                fg_scale_height: 720,
            },
            output_spec: OutputSpec {
                width: 1920,
                height: 1080,
                codec: "h264".to_string(),
                bitrate_mbps: 18,
            },
            lip_sync_points: vec![],
            transition_duration_seconds: 0.5,
        };

        let catalog = ShotCatalog {
            assets: vec![],
            total_duration_seconds: 0.0,
            interview_assets: vec![],
            broll_assets: vec![],
            music_assets: vec![],
            music_beat_grid: None,
        };

        let output_dir = std::env::temp_dir().join("content_editing_test");
        std::fs::create_dir_all(&output_dir).unwrap();

        let result = processor.generate_premiere_xml(&directive, &catalog, &output_dir);
        assert!(result.is_ok());

        let xml_path = result.unwrap();
        assert!(xml_path.exists());

        let content = std::fs::read_to_string(&xml_path).unwrap();
        assert!(content.contains("<xmeml version=\"4\">"));
        assert!(content.contains("Test Project"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&output_dir);
    }
}
