//! Premiere Pro XML Export
//!
//! Generates Final Cut Pro XML 1.0 format which Adobe Premiere Pro can import.
//! This format is widely supported and allows full timeline reconstruction.

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use uuid::Uuid;

use super::edit_assembly::{AssembledEdit, TimelineClip, AudioClip, EditMarker, MarkerType};

/// XML Generator for Premiere Pro import
pub struct PremiereXmlExporter {
    /// Timebase (frames per second as integer, e.g., 24 for 23.976)
    timebase: u32,
    /// Actual frame rate
    frame_rate: f32,
    /// NTSC flag (for drop-frame timecode)
    ntsc: bool,
}

impl PremiereXmlExporter {
    pub fn new(frame_rate: f32) -> Self {
        // Determine timebase and NTSC flag
        let (timebase, ntsc) = match frame_rate as u32 {
            23 | 24 => (24, frame_rate < 24.0), // 23.976 is NTSC
            25 => (25, false),
            29 | 30 => (30, frame_rate < 30.0), // 29.97 is NTSC
            50 => (50, false),
            59 | 60 => (60, frame_rate < 60.0), // 59.94 is NTSC
            _ => (30, false),
        };

        Self {
            timebase,
            frame_rate,
            ntsc,
        }
    }

    /// Convert seconds to frame count
    fn seconds_to_frames(&self, seconds: f64) -> i64 {
        (seconds * self.frame_rate as f64).round() as i64
    }

    /// Generate FCP XML from assembled edit
    pub fn export(&self, edit: &AssembledEdit, output_path: &Path) -> std::io::Result<PathBuf> {
        let xml = self.generate_xml(edit);
        std::fs::write(output_path, &xml)?;
        Ok(output_path.to_path_buf())
    }

    /// Generate the XML string
    pub fn generate_xml(&self, edit: &AssembledEdit) -> String {
        let mut xml = String::new();

        // XML declaration
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push('\n');
        xml.push_str(r#"<!DOCTYPE xmeml>"#);
        xml.push('\n');

        // Root element
        xml.push_str(r#"<xmeml version="5">"#);
        xml.push('\n');

        // Project
        xml.push_str(&self.generate_project(edit));

        xml.push_str("</xmeml>\n");
        xml
    }

    fn generate_project(&self, edit: &AssembledEdit) -> String {
        let mut xml = String::new();
        let project_id = Uuid::new_v4();

        xml.push_str(&format!(r#"  <project id="{}">"#, project_id));
        xml.push('\n');
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&edit.name)));

        // Children (bins and sequences)
        xml.push_str("    <children>\n");

        // Media bin
        xml.push_str(&self.generate_media_bin(edit));

        // Sequence
        xml.push_str(&self.generate_sequence(edit));

        xml.push_str("    </children>\n");
        xml.push_str("  </project>\n");

        xml
    }

    fn generate_media_bin(&self, edit: &AssembledEdit) -> String {
        let mut xml = String::new();
        let bin_id = Uuid::new_v4();

        xml.push_str(&format!(r#"      <bin id="{}">"#, bin_id));
        xml.push('\n');
        xml.push_str("        <name>Media</name>\n");
        xml.push_str("        <children>\n");

        // Collect unique media files
        let mut media_files: HashMap<String, &PathBuf> = HashMap::new();

        for clip in &edit.video_clips {
            let key = clip.source.to_string_lossy().to_string();
            media_files.insert(key, &clip.source);
        }

        for clip in &edit.audio_clips {
            let key = clip.source.to_string_lossy().to_string();
            media_files.insert(key, &clip.source);
        }

        // Generate clip entries
        for (_, path) in &media_files {
            xml.push_str(&self.generate_master_clip(path, edit));
        }

        xml.push_str("        </children>\n");
        xml.push_str("      </bin>\n");

        xml
    }

    fn generate_master_clip(&self, path: &Path, edit: &AssembledEdit) -> String {
        let mut xml = String::new();
        let clip_id = Uuid::new_v4();
        let file_id = Uuid::new_v4();

        let filename = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let is_video = path.extension()
            .map(|e| {
                let ext = e.to_string_lossy().to_lowercase();
                matches!(ext.as_str(), "mp4" | "mov" | "avi" | "mxf" | "mkv" | "m4v")
            })
            .unwrap_or(false);

        let is_audio = path.extension()
            .map(|e| {
                let ext = e.to_string_lossy().to_lowercase();
                matches!(ext.as_str(), "mp3" | "wav" | "aif" | "aiff" | "m4a" | "flac")
            })
            .unwrap_or(false);

        xml.push_str(&format!(r#"          <clip id="{}">"#, clip_id));
        xml.push('\n');
        xml.push_str(&format!("            <name>{}</name>\n", escape_xml(&filename)));
        xml.push_str(&format!("            <duration>{}</duration>\n", self.seconds_to_frames(edit.duration)));

        // Rate
        xml.push_str("            <rate>\n");
        xml.push_str(&format!("              <timebase>{}</timebase>\n", self.timebase));
        xml.push_str(&format!("              <ntsc>{}</ntsc>\n", if self.ntsc { "TRUE" } else { "FALSE" }));
        xml.push_str("            </rate>\n");

        // Media
        xml.push_str("            <media>\n");

        if is_video {
            xml.push_str("              <video>\n");
            xml.push_str("                <track>\n");
            xml.push_str(&format!(r#"                  <clipitem id="{}_video">"#, clip_id));
            xml.push('\n');
            xml.push_str(&format!("                    <name>{}</name>\n", escape_xml(&filename)));
            xml.push_str(&self.generate_file_reference(&file_id, path, edit));
            xml.push_str("                  </clipitem>\n");
            xml.push_str("                </track>\n");
            xml.push_str("              </video>\n");
        }

        if is_audio || is_video {
            xml.push_str("              <audio>\n");
            xml.push_str("                <track>\n");
            xml.push_str(&format!(r#"                  <clipitem id="{}_audio">"#, clip_id));
            xml.push('\n');
            xml.push_str(&format!("                    <name>{}</name>\n", escape_xml(&filename)));
            xml.push_str(&self.generate_file_reference(&file_id, path, edit));
            xml.push_str("                  </clipitem>\n");
            xml.push_str("                </track>\n");
            xml.push_str("              </audio>\n");
        }

        xml.push_str("            </media>\n");
        xml.push_str("          </clip>\n");

        xml
    }

    fn generate_file_reference(&self, file_id: &Uuid, path: &Path, edit: &AssembledEdit) -> String {
        let mut xml = String::new();

        let filename = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        // Convert path to file:// URL
        let file_url = format!("file://localhost{}", path.to_string_lossy().replace(" ", "%20"));

        xml.push_str(&format!(r#"                    <file id="{}">"#, file_id));
        xml.push('\n');
        xml.push_str(&format!("                      <name>{}</name>\n", escape_xml(&filename)));
        xml.push_str(&format!("                      <pathurl>{}</pathurl>\n", escape_xml(&file_url)));
        xml.push_str("                      <rate>\n");
        xml.push_str(&format!("                        <timebase>{}</timebase>\n", self.timebase));
        xml.push_str(&format!("                        <ntsc>{}</ntsc>\n", if self.ntsc { "TRUE" } else { "FALSE" }));
        xml.push_str("                      </rate>\n");
        xml.push_str(&format!("                      <duration>{}</duration>\n", self.seconds_to_frames(edit.duration)));
        xml.push_str("                      <media>\n");
        xml.push_str("                        <video>\n");
        xml.push_str("                          <samplecharacteristics>\n");
        xml.push_str(&format!("                            <width>{}</width>\n", edit.width));
        xml.push_str(&format!("                            <height>{}</height>\n", edit.height));
        xml.push_str("                          </samplecharacteristics>\n");
        xml.push_str("                        </video>\n");
        xml.push_str("                        <audio>\n");
        xml.push_str("                          <channelcount>2</channelcount>\n");
        xml.push_str("                          <samplecharacteristics>\n");
        xml.push_str("                            <depth>16</depth>\n");
        xml.push_str("                            <samplerate>48000</samplerate>\n");
        xml.push_str("                          </samplecharacteristics>\n");
        xml.push_str("                        </audio>\n");
        xml.push_str("                      </media>\n");
        xml.push_str("                    </file>\n");

        xml
    }

    fn generate_sequence(&self, edit: &AssembledEdit) -> String {
        let mut xml = String::new();
        let seq_id = Uuid::new_v4();

        xml.push_str(&format!(r#"      <sequence id="{}">"#, seq_id));
        xml.push('\n');
        xml.push_str(&format!("        <name>{}</name>\n", escape_xml(&edit.name)));
        xml.push_str(&format!("        <duration>{}</duration>\n", self.seconds_to_frames(edit.duration)));

        // Rate
        xml.push_str("        <rate>\n");
        xml.push_str(&format!("          <timebase>{}</timebase>\n", self.timebase));
        xml.push_str(&format!("          <ntsc>{}</ntsc>\n", if self.ntsc { "TRUE" } else { "FALSE" }));
        xml.push_str("        </rate>\n");

        // Timecode
        xml.push_str("        <timecode>\n");
        xml.push_str("          <rate>\n");
        xml.push_str(&format!("            <timebase>{}</timebase>\n", self.timebase));
        xml.push_str(&format!("            <ntsc>{}</ntsc>\n", if self.ntsc { "TRUE" } else { "FALSE" }));
        xml.push_str("          </rate>\n");
        xml.push_str("          <string>00:00:00:00</string>\n");
        xml.push_str("          <frame>0</frame>\n");
        xml.push_str("          <displayformat>NDF</displayformat>\n");
        xml.push_str("        </timecode>\n");

        // Media (tracks)
        xml.push_str("        <media>\n");

        // Video tracks
        xml.push_str("          <video>\n");
        xml.push_str(&self.generate_video_format(edit));
        xml.push_str(&self.generate_video_tracks(edit));
        xml.push_str("          </video>\n");

        // Audio tracks
        xml.push_str("          <audio>\n");
        xml.push_str(&self.generate_audio_tracks(edit));
        xml.push_str("          </audio>\n");

        xml.push_str("        </media>\n");

        // Markers
        if !edit.markers.is_empty() {
            for marker in &edit.markers {
                xml.push_str(&self.generate_marker(marker));
            }
        }

        xml.push_str("      </sequence>\n");

        xml
    }

    fn generate_video_format(&self, edit: &AssembledEdit) -> String {
        let mut xml = String::new();

        xml.push_str("            <format>\n");
        xml.push_str("              <samplecharacteristics>\n");
        xml.push_str(&format!("                <width>{}</width>\n", edit.width));
        xml.push_str(&format!("                <height>{}</height>\n", edit.height));
        xml.push_str("                <anamorphic>FALSE</anamorphic>\n");
        xml.push_str("                <pixelaspectratio>square</pixelaspectratio>\n");
        xml.push_str("                <fielddominance>none</fielddominance>\n");
        xml.push_str("                <rate>\n");
        xml.push_str(&format!("                  <timebase>{}</timebase>\n", self.timebase));
        xml.push_str(&format!("                  <ntsc>{}</ntsc>\n", if self.ntsc { "TRUE" } else { "FALSE" }));
        xml.push_str("                </rate>\n");
        xml.push_str("                <colordepth>24</colordepth>\n");
        xml.push_str("                <codec>\n");
        xml.push_str("                  <name>Apple ProRes 422</name>\n");
        xml.push_str("                </codec>\n");
        xml.push_str("              </samplecharacteristics>\n");
        xml.push_str("            </format>\n");

        xml
    }

    fn generate_video_tracks(&self, edit: &AssembledEdit) -> String {
        let mut xml = String::new();

        // Group clips by track
        let max_track = edit.video_clips.iter()
            .map(|c| c.track)
            .max()
            .unwrap_or(1);

        for track_num in 1..=max_track {
            let track_clips: Vec<&TimelineClip> = edit.video_clips.iter()
                .filter(|c| c.track == track_num)
                .collect();

            xml.push_str("            <track>\n");

            for clip in track_clips {
                xml.push_str(&self.generate_video_clipitem(clip, edit));
            }

            xml.push_str("            </track>\n");
        }

        xml
    }

    fn generate_video_clipitem(&self, clip: &TimelineClip, edit: &AssembledEdit) -> String {
        let mut xml = String::new();
        let clipitem_id = Uuid::new_v4();
        let file_id = Uuid::new_v4();

        let filename = clip.source.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let file_url = format!("file://localhost{}", clip.source.to_string_lossy().replace(" ", "%20"));

        xml.push_str(&format!(r#"              <clipitem id="{}">"#, clipitem_id));
        xml.push('\n');
        xml.push_str(&format!("                <name>{}</name>\n", escape_xml(&filename)));
        xml.push_str("                <enabled>TRUE</enabled>\n");
        xml.push_str(&format!("                <start>{}</start>\n", self.seconds_to_frames(clip.timeline_in)));
        xml.push_str(&format!("                <end>{}</end>\n", self.seconds_to_frames(clip.timeline_out)));
        xml.push_str(&format!("                <in>{}</in>\n", self.seconds_to_frames(clip.source_in)));
        xml.push_str(&format!("                <out>{}</out>\n", self.seconds_to_frames(clip.source_out)));

        // Master clip reference
        xml.push_str(&format!("                <masterclipid>{}</masterclipid>\n", clipitem_id));

        // File reference
        xml.push_str(&format!(r#"                <file id="{}">"#, file_id));
        xml.push('\n');
        xml.push_str(&format!("                  <name>{}</name>\n", escape_xml(&filename)));
        xml.push_str(&format!("                  <pathurl>{}</pathurl>\n", escape_xml(&file_url)));
        xml.push_str("                  <rate>\n");
        xml.push_str(&format!("                    <timebase>{}</timebase>\n", self.timebase));
        xml.push_str(&format!("                    <ntsc>{}</ntsc>\n", if self.ntsc { "TRUE" } else { "FALSE" }));
        xml.push_str("                  </rate>\n");
        xml.push_str("                  <media>\n");
        xml.push_str("                    <video>\n");
        xml.push_str("                      <samplecharacteristics>\n");
        xml.push_str(&format!("                        <width>{}</width>\n", edit.width));
        xml.push_str(&format!("                        <height>{}</height>\n", edit.height));
        xml.push_str("                      </samplecharacteristics>\n");
        xml.push_str("                    </video>\n");
        xml.push_str("                  </media>\n");
        xml.push_str("                </file>\n");

        // Transitions
        if let Some(ref transition) = clip.transition_in {
            xml.push_str(&self.generate_transition(transition, true));
        }
        if let Some(ref transition) = clip.transition_out {
            xml.push_str(&self.generate_transition(transition, false));
        }

        // Speed/time remapping
        if clip.speed != 1.0 {
            xml.push_str("                <filter>\n");
            xml.push_str("                  <effect>\n");
            xml.push_str("                    <name>Time Remap</name>\n");
            xml.push_str("                    <effectid>timeremap</effectid>\n");
            xml.push_str("                    <effecttype>motion</effecttype>\n");
            xml.push_str("                    <parameter>\n");
            xml.push_str("                      <parameterid>speed</parameterid>\n");
            xml.push_str(&format!("                      <value>{}</value>\n", clip.speed * 100.0));
            xml.push_str("                    </parameter>\n");
            xml.push_str("                  </effect>\n");
            xml.push_str("                </filter>\n");
        }

        // Opacity
        if clip.opacity != 1.0 {
            xml.push_str("                <filter>\n");
            xml.push_str("                  <effect>\n");
            xml.push_str("                    <name>Basic Motion</name>\n");
            xml.push_str("                    <effectid>basic</effectid>\n");
            xml.push_str("                    <effecttype>motion</effecttype>\n");
            xml.push_str("                    <parameter>\n");
            xml.push_str("                      <parameterid>opacity</parameterid>\n");
            xml.push_str(&format!("                      <value>{}</value>\n", clip.opacity * 100.0));
            xml.push_str("                    </parameter>\n");
            xml.push_str("                  </effect>\n");
            xml.push_str("                </filter>\n");
        }

        xml.push_str("              </clipitem>\n");

        xml
    }

    fn generate_transition(&self, transition: &super::edit_assembly::TransitionSpec, is_start: bool) -> String {
        let mut xml = String::new();

        let alignment = if is_start { "start" } else { "end" };

        xml.push_str("                <transitionitem>\n");
        xml.push_str(&format!("                  <start>{}</start>\n", 0));
        xml.push_str(&format!("                  <end>{}</end>\n", self.seconds_to_frames(transition.duration)));
        xml.push_str(&format!("                  <alignment>{}</alignment>\n", alignment));

        xml.push_str("                  <effect>\n");

        match transition.transition_type.as_str() {
            "dissolve" => {
                xml.push_str("                    <name>Cross Dissolve</name>\n");
                xml.push_str("                    <effectid>CrossDissolve</effectid>\n");
                xml.push_str("                    <effectcategory>Dissolve</effectcategory>\n");
                xml.push_str("                    <effecttype>transition</effecttype>\n");
            }
            "dip_to_black" => {
                xml.push_str("                    <name>Dip to Black</name>\n");
                xml.push_str("                    <effectid>DipToBlack</effectid>\n");
                xml.push_str("                    <effectcategory>Dissolve</effectcategory>\n");
                xml.push_str("                    <effecttype>transition</effecttype>\n");
            }
            "wipe" => {
                xml.push_str("                    <name>Wipe</name>\n");
                xml.push_str("                    <effectid>Wipe</effectid>\n");
                xml.push_str("                    <effectcategory>Wipe</effectcategory>\n");
                xml.push_str("                    <effecttype>transition</effecttype>\n");
            }
            _ => {
                xml.push_str("                    <name>Cross Dissolve</name>\n");
                xml.push_str("                    <effectid>CrossDissolve</effectid>\n");
                xml.push_str("                    <effectcategory>Dissolve</effectcategory>\n");
                xml.push_str("                    <effecttype>transition</effecttype>\n");
            }
        }

        xml.push_str("                  </effect>\n");
        xml.push_str("                </transitionitem>\n");

        xml
    }

    fn generate_audio_tracks(&self, edit: &AssembledEdit) -> String {
        let mut xml = String::new();

        // Audio format
        xml.push_str("            <format>\n");
        xml.push_str("              <samplecharacteristics>\n");
        xml.push_str("                <depth>16</depth>\n");
        xml.push_str("                <samplerate>48000</samplerate>\n");
        xml.push_str("              </samplecharacteristics>\n");
        xml.push_str("            </format>\n");
        xml.push_str("            <outputs>\n");
        xml.push_str("              <group>\n");
        xml.push_str("                <index>1</index>\n");
        xml.push_str("                <numchannels>2</numchannels>\n");
        xml.push_str("                <downmix>0</downmix>\n");
        xml.push_str("                <channel>\n");
        xml.push_str("                  <index>1</index>\n");
        xml.push_str("                </channel>\n");
        xml.push_str("                <channel>\n");
        xml.push_str("                  <index>2</index>\n");
        xml.push_str("                </channel>\n");
        xml.push_str("              </group>\n");
        xml.push_str("            </outputs>\n");

        // Group clips by track
        let max_track = edit.audio_clips.iter()
            .map(|c| c.track)
            .max()
            .unwrap_or(1);

        for track_num in 1..=max_track {
            let track_clips: Vec<&AudioClip> = edit.audio_clips.iter()
                .filter(|c| c.track == track_num)
                .collect();

            // Stereo track (2 channels)
            for channel in 1..=2 {
                xml.push_str("            <track>\n");

                for clip in &track_clips {
                    xml.push_str(&self.generate_audio_clipitem(clip, channel, edit));
                }

                xml.push_str("              <outputchannelindex>1</outputchannelindex>\n");
                xml.push_str("            </track>\n");
            }
        }

        xml
    }

    fn generate_audio_clipitem(&self, clip: &AudioClip, channel: u32, _edit: &AssembledEdit) -> String {
        let mut xml = String::new();
        let clipitem_id = Uuid::new_v4();
        let file_id = Uuid::new_v4();

        let filename = clip.source.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let file_url = format!("file://localhost{}", clip.source.to_string_lossy().replace(" ", "%20"));

        xml.push_str(&format!(r#"              <clipitem id="{}_{}">"#, clipitem_id, channel));
        xml.push('\n');
        xml.push_str(&format!("                <name>{}</name>\n", escape_xml(&filename)));
        xml.push_str("                <enabled>TRUE</enabled>\n");
        xml.push_str(&format!("                <start>{}</start>\n", self.seconds_to_frames(clip.timeline_in)));
        xml.push_str(&format!("                <end>{}</end>\n", self.seconds_to_frames(clip.timeline_out)));
        xml.push_str(&format!("                <in>{}</in>\n", self.seconds_to_frames(clip.source_in)));
        xml.push_str(&format!("                <out>{}</out>\n", self.seconds_to_frames(clip.source_out)));

        // File reference
        xml.push_str(&format!(r#"                <file id="{}">"#, file_id));
        xml.push('\n');
        xml.push_str(&format!("                  <name>{}</name>\n", escape_xml(&filename)));
        xml.push_str(&format!("                  <pathurl>{}</pathurl>\n", escape_xml(&file_url)));
        xml.push_str("                  <rate>\n");
        xml.push_str(&format!("                    <timebase>{}</timebase>\n", self.timebase));
        xml.push_str(&format!("                    <ntsc>{}</ntsc>\n", if self.ntsc { "TRUE" } else { "FALSE" }));
        xml.push_str("                  </rate>\n");
        xml.push_str("                  <media>\n");
        xml.push_str("                    <audio>\n");
        xml.push_str("                      <channelcount>2</channelcount>\n");
        xml.push_str("                      <samplecharacteristics>\n");
        xml.push_str("                        <depth>16</depth>\n");
        xml.push_str("                        <samplerate>48000</samplerate>\n");
        xml.push_str("                      </samplecharacteristics>\n");
        xml.push_str("                    </audio>\n");
        xml.push_str("                  </media>\n");
        xml.push_str("                </file>\n");

        // Source channel
        xml.push_str(&format!("                <sourcetrack>\n"));
        xml.push_str(&format!("                  <mediatype>audio</mediatype>\n"));
        xml.push_str(&format!("                  <trackindex>{}</trackindex>\n", channel));
        xml.push_str(&format!("                </sourcetrack>\n"));

        // Volume/Level
        if clip.volume != 1.0 {
            xml.push_str("                <filter>\n");
            xml.push_str("                  <effect>\n");
            xml.push_str("                    <name>Audio Levels</name>\n");
            xml.push_str("                    <effectid>audiolevels</effectid>\n");
            xml.push_str("                    <effectcategory>audio</effectcategory>\n");
            xml.push_str("                    <effecttype>filter</effecttype>\n");
            xml.push_str("                    <parameter>\n");
            xml.push_str("                      <parameterid>level</parameterid>\n");
            // Convert 0-2 scale to dB (1.0 = 0dB)
            let db = 20.0 * (clip.volume as f64).log10();
            xml.push_str(&format!("                      <value>{:.2}</value>\n", db));
            xml.push_str("                    </parameter>\n");
            xml.push_str("                  </effect>\n");
            xml.push_str("                </filter>\n");
        }

        // Fade in
        if let Some(fade_duration) = clip.fade_in {
            xml.push_str("                <filter>\n");
            xml.push_str("                  <effect>\n");
            xml.push_str("                    <name>Audio Fade In</name>\n");
            xml.push_str("                    <effectid>audiofadein</effectid>\n");
            xml.push_str("                    <effecttype>filter</effecttype>\n");
            xml.push_str("                    <parameter>\n");
            xml.push_str("                      <parameterid>duration</parameterid>\n");
            xml.push_str(&format!("                      <value>{}</value>\n", self.seconds_to_frames(fade_duration)));
            xml.push_str("                    </parameter>\n");
            xml.push_str("                  </effect>\n");
            xml.push_str("                </filter>\n");
        }

        // Fade out
        if let Some(fade_duration) = clip.fade_out {
            xml.push_str("                <filter>\n");
            xml.push_str("                  <effect>\n");
            xml.push_str("                    <name>Audio Fade Out</name>\n");
            xml.push_str("                    <effectid>audiofadeout</effectid>\n");
            xml.push_str("                    <effecttype>filter</effecttype>\n");
            xml.push_str("                    <parameter>\n");
            xml.push_str("                      <parameterid>duration</parameterid>\n");
            xml.push_str(&format!("                      <value>{}</value>\n", self.seconds_to_frames(fade_duration)));
            xml.push_str("                    </parameter>\n");
            xml.push_str("                  </effect>\n");
            xml.push_str("                </filter>\n");
        }

        xml.push_str("              </clipitem>\n");

        xml
    }

    fn generate_marker(&self, marker: &EditMarker) -> String {
        let mut xml = String::new();

        xml.push_str("        <marker>\n");
        xml.push_str(&format!("          <name>{}</name>\n", escape_xml(&marker.name)));
        xml.push_str(&format!("          <in>{}</in>\n", self.seconds_to_frames(marker.time)));
        xml.push_str(&format!("          <out>{}</out>\n", self.seconds_to_frames(marker.time)));

        // Marker color (Premiere uses color index 0-7)
        let color_index = match marker.marker_type {
            MarkerType::Chapter => 0,   // Green
            MarkerType::Beat => 1,      // Red
            MarkerType::Section => 2,   // Purple
            MarkerType::Note => 3,      // Orange
        };
        xml.push_str(&format!("          <comment>{}</comment>\n", escape_xml(&marker.color)));

        xml.push_str("        </marker>\n");

        xml
    }
}

/// Escape XML special characters
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

    #[test]
    fn test_seconds_to_frames() {
        let exporter = PremiereXmlExporter::new(24.0);
        assert_eq!(exporter.seconds_to_frames(1.0), 24);
        assert_eq!(exporter.seconds_to_frames(2.5), 60);
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(escape_xml("Test & <value>"), "Test &amp; &lt;value&gt;");
    }
}
