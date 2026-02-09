//! Native Premiere Pro .prproj Export
//!
//! Modifies native .prproj files (gzipped XML) to rearrange clips on the timeline.
//! This is the most reliable way to create editable Premiere Pro projects — FCP XML
//! import is unreliable in Premiere Pro 2026+, and ExtendScript automation requires
//! manual CEP panel installation.
//!
//! ## How .prproj files work
//!
//! A .prproj file is gzipped XML containing the full project state. Key structures:
//!
//! ```text
//! VideoClipTrackItem (timeline position: Start/End in ticks)
//!   └─ SubClip (links to source)
//!       ├─ VideoClip (source in/out: InPoint/OutPoint in ticks)
//!       │   └─ Source → VideoMediaSource → Media (file path)
//!       └─ MasterClip ObjectURef (UUID linking to bin item)
//! ```
//!
//! ## Time units
//!
//! Premiere Pro uses "ticks" internally:
//! - **254,016,000,000 ticks per second** (constant across all frame rates)
//! - At 29.97fps: 1 frame = 8,475,667,200 ticks
//! - At 23.976fps: 1 frame = 10,594,584,000 ticks
//!
//! ## Strategy
//!
//! Instead of injecting new XML objects (which crashes Premiere due to missing
//! internal references), we **reuse existing objects** from the original project:
//!
//! 1. Decompress the .prproj gzip
//! 2. Map each clip filename → VideoClipTrackItem ObjectID
//! 3. Update Start/End (timeline position) and InPoint/OutPoint (source range)
//! 4. Update the TrackItems list to reference only the clips in the new EDL
//! 5. Recompress and write
//!
//! This preserves all internal object references, component chains, and media links.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use regex::Regex;

use super::{EditronError, EditronResult};

/// Premiere Pro internal tick rate: 254,016,000,000 ticks per second
const TICKS_PER_SECOND: u64 = 254_016_000_000;

/// A single clip placement in the edit decision list
#[derive(Debug, Clone)]
pub struct PrprojClipEntry {
    /// Source filename (e.g., "MVI_6359.MP4")
    pub filename: String,
    /// Source in-point in seconds
    pub source_in: f64,
    /// Clip duration in seconds
    pub duration: f64,
    /// Optional description/label
    pub label: Option<String>,
}

/// Result of a .prproj recut operation
#[derive(Debug)]
pub struct PrprojRecutResult {
    /// Path to the output .prproj file
    pub output_path: PathBuf,
    /// Number of clips placed on timeline
    pub clips_placed: usize,
    /// Total timeline duration in seconds
    pub total_duration: f64,
    /// Any clips from the EDL that weren't found in the project
    pub missing_clips: Vec<String>,
    /// File size in bytes
    pub file_size: u64,
}

/// Engine for modifying native Premiere Pro .prproj files
pub struct PrprojRecutEngine {
    /// Decompressed XML content
    xml: String,
    /// Map of clip filename → MasterClip UUID
    clip_uuid_map: HashMap<String, String>,
    /// Map of VideoClipTrackItem ObjectID → clip filename
    vcti_to_name: HashMap<String, String>,
    /// Map of VideoClipTrackItem ObjectID → VideoClip ObjectID
    vcti_to_clip_id: HashMap<String, String>,
}

impl PrprojRecutEngine {
    /// Load and parse a .prproj file
    pub fn load<P: AsRef<Path>>(path: P) -> EditronResult<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(EditronError::FileNotFound(path.to_path_buf()));
        }

        // Decompress gzip
        let file = std::fs::File::open(path)?;
        let mut decoder = GzDecoder::new(file);
        let mut xml = String::new();
        decoder.read_to_string(&mut xml)
            .map_err(|e| EditronError::InvalidFormat(format!("Failed to decompress .prproj: {}", e)))?;

        let mut engine = Self {
            xml,
            clip_uuid_map: HashMap::new(),
            vcti_to_name: HashMap::new(),
            vcti_to_clip_id: HashMap::new(),
        };

        engine.parse_structure()?;
        Ok(engine)
    }

    /// Parse the XML structure to build internal maps
    fn parse_structure(&mut self) -> EditronResult<()> {
        // Build SubClip name → MasterClip UUID map
        let subclip_re = Regex::new(r"<SubClip[^>]*>.*?</SubClip>")
            .map_err(|e| EditronError::InvalidFormat(e.to_string()))?;
        let uuid_re = Regex::new(r#"<MasterClip ObjectURef="([^"]+)""#)
            .map_err(|e| EditronError::InvalidFormat(e.to_string()))?;
        let name_re = Regex::new(r"<Name>([^<]+)</Name>")
            .map_err(|e| EditronError::InvalidFormat(e.to_string()))?;

        // Use a simple approach to avoid regex DOTALL issues
        // Find SubClip blocks by searching for open/close tags
        let mut pos = 0;
        while let Some(start) = self.xml[pos..].find("<SubClip ") {
            let abs_start = pos + start;
            if let Some(end) = self.xml[abs_start..].find("</SubClip>") {
                let abs_end = abs_start + end + "</SubClip>".len();
                let block = &self.xml[abs_start..abs_end];

                if let (Some(uuid_cap), Some(name_cap)) = (
                    uuid_re.captures(block),
                    name_re.captures(block),
                ) {
                    let uuid = uuid_cap.get(1).unwrap().as_str().to_string();
                    let name = name_cap.get(1).unwrap().as_str().to_string();
                    self.clip_uuid_map.insert(name, uuid);
                }

                pos = abs_end;
            } else {
                break;
            }
        }

        // Map VideoClipTrackItem ObjectIDs to clip names and VideoClip IDs
        let vcti_re = Regex::new(r#"<VideoClipTrackItem ObjectID="(\d+)""#)
            .map_err(|e| EditronError::InvalidFormat(e.to_string()))?;
        let subclip_ref_re = Regex::new(r#"<SubClip ObjectRef="(\d+)""#)
            .map_err(|e| EditronError::InvalidFormat(e.to_string()))?;
        let clip_ref_re = Regex::new(r#"<Clip ObjectRef="(\d+)""#)
            .map_err(|e| EditronError::InvalidFormat(e.to_string()))?;

        pos = 0;
        while let Some(start) = self.xml[pos..].find("<VideoClipTrackItem ") {
            let abs_start = pos + start;
            if let Some(end) = self.xml[abs_start..].find("</VideoClipTrackItem>") {
                let abs_end = abs_start + end + "</VideoClipTrackItem>".len();
                let block = &self.xml[abs_start..abs_end];

                if let Some(vcti_cap) = vcti_re.captures(block) {
                    let vcti_id = vcti_cap.get(1).unwrap().as_str().to_string();

                    // Find SubClip reference
                    if let Some(sc_cap) = subclip_ref_re.captures(block) {
                        let subclip_id = sc_cap.get(1).unwrap().as_str();

                        // Find the SubClip definition to get name and clip ref
                        let sc_pattern = format!("ObjectID=\"{}\"", subclip_id);
                        if let Some(sc_pos) = self.xml.find(&sc_pattern) {
                            let sc_block_end = self.xml[sc_pos..].find("</SubClip>")
                                .map(|e| sc_pos + e + "</SubClip>".len())
                                .unwrap_or(sc_pos + 500);
                            let sc_block = &self.xml[sc_pos..sc_block_end.min(self.xml.len())];

                            if let Some(name_cap) = name_re.captures(sc_block) {
                                self.vcti_to_name.insert(
                                    vcti_id.clone(),
                                    name_cap.get(1).unwrap().as_str().to_string(),
                                );
                            }
                            if let Some(clip_cap) = clip_ref_re.captures(sc_block) {
                                self.vcti_to_clip_id.insert(
                                    vcti_id.clone(),
                                    clip_cap.get(1).unwrap().as_str().to_string(),
                                );
                            }
                        }
                    }
                }

                pos = abs_end;
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Get the number of clips found in the project
    pub fn clip_count(&self) -> usize {
        self.clip_uuid_map.len()
    }

    /// Get all clip filenames in the project
    pub fn clip_names(&self) -> Vec<&str> {
        self.clip_uuid_map.keys().map(|s| s.as_str()).collect()
    }

    /// Apply an EDL to recut the project timeline
    ///
    /// This modifies the internal XML to rearrange clips on V1 according to the
    /// provided edit decision list. All clips must already exist in the project bin.
    pub fn apply_edl(&mut self, edl: &[PrprojClipEntry]) -> EditronResult<PrprojRecutResult> {
        let mut missing_clips = Vec::new();
        let mut edl_vcti_map = Vec::new(); // (edl_index, vcti_id, clip_id)

        // Build name → VCTI ID map
        let mut name_to_vctis: HashMap<String, Vec<String>> = HashMap::new();
        for (vcti_id, name) in &self.vcti_to_name {
            name_to_vctis.entry(name.clone()).or_default().push(vcti_id.clone());
        }

        // Assign VCTIs to EDL entries
        let mut used_vctis = std::collections::HashSet::new();
        for (i, entry) in edl.iter().enumerate() {
            let available: Vec<_> = name_to_vctis
                .get(&entry.filename)
                .map(|v| v.iter().filter(|id| !used_vctis.contains(*id)).cloned().collect())
                .unwrap_or_default();

            if let Some(vcti_id) = available.first() {
                used_vctis.insert(vcti_id.clone());
                let clip_id = self.vcti_to_clip_id.get(vcti_id).cloned();
                edl_vcti_map.push((i, Some(vcti_id.clone()), clip_id));
            } else {
                missing_clips.push(entry.filename.clone());
                edl_vcti_map.push((i, None, None));
            }
        }

        // Update TrackItems list in the VideoClipTrack
        let track_items_re = Regex::new(
            r"(<ClipItems Version=""3"">\s*<TrackItems Version=""1"">)([\s\S]*?)(</TrackItems>)"
        ).map_err(|e| EditronError::InvalidFormat(e.to_string()))?;

        // Build replacement manually since regex with special chars is tricky
        if let Some(ti_start) = self.xml.find("<ClipItems Version=\"3\">\n") {
            if let Some(ti_items_start) = self.xml[ti_start..].find("<TrackItems Version=\"1\">") {
                let abs_ti_start = ti_start + ti_items_start + "<TrackItems Version=\"1\">".len();
                if let Some(ti_end) = self.xml[abs_ti_start..].find("</TrackItems>") {
                    let abs_ti_end = abs_ti_start + ti_end;

                    // Build new TrackItems content
                    let mut new_items = String::from("\n");
                    for (idx, (_, vcti_id_opt, _)) in edl_vcti_map.iter().enumerate() {
                        if let Some(vcti_id) = vcti_id_opt {
                            new_items.push_str(&format!(
                                "\t\t\t\t\t<TrackItem Index=\"{}\" ObjectRef=\"{}\"/>\n",
                                idx, vcti_id
                            ));
                        }
                    }
                    new_items.push_str("\t\t\t\t");

                    self.xml = format!(
                        "{}{}{}",
                        &self.xml[..abs_ti_start],
                        new_items,
                        &self.xml[abs_ti_end..],
                    );
                }
            }
        }

        // Update each clip's timeline position and source in/out
        let mut timeline_pos = 0.0;
        let mut clips_placed = 0;

        for (edl_i, vcti_id_opt, clip_id_opt) in &edl_vcti_map {
            let entry = &edl[*edl_i];

            if let Some(vcti_id) = vcti_id_opt {
                let start_ticks = seconds_to_ticks(timeline_pos);
                let end_ticks = seconds_to_ticks(timeline_pos + entry.duration);
                let in_ticks = seconds_to_ticks(entry.source_in);
                let out_ticks = seconds_to_ticks(entry.source_in + entry.duration);

                // Update VideoClipTrackItem Start/End
                self.update_track_item_times(vcti_id, start_ticks, end_ticks);

                // Update VideoClip InPoint/OutPoint
                if let Some(clip_id) = clip_id_opt {
                    self.update_clip_in_out(clip_id, in_ticks, out_ticks);
                }

                clips_placed += 1;
            }

            timeline_pos += entry.duration;
        }

        Ok(PrprojRecutResult {
            output_path: PathBuf::new(), // Set by caller
            clips_placed,
            total_duration: timeline_pos,
            missing_clips,
            file_size: 0, // Set after write
        })
    }

    /// Update a VideoClipTrackItem's Start/End times
    fn update_track_item_times(&mut self, vcti_id: &str, start_ticks: u64, end_ticks: u64) {
        let pattern = format!("ObjectID=\"{}\"", vcti_id);
        if let Some(vcti_pos) = self.xml.find(&pattern) {
            // Find the <TrackItem Version="4"> within this VCTI
            let search_region = &self.xml[vcti_pos..];
            if let Some(ti_offset) = search_region.find("<TrackItem Version=\"4\">") {
                let ti_start = vcti_pos + ti_offset;
                if let Some(ti_end_offset) = self.xml[ti_start..].find("</TrackItem>") {
                    let ti_end = ti_start + ti_end_offset;
                    let ti_content_start = ti_start + "<TrackItem Version=\"4\">".len();

                    // Build new content
                    let new_content = if start_ticks == 0 {
                        format!("\n\t\t\t\t<End>{}</End>\n\t\t\t", end_ticks)
                    } else {
                        format!(
                            "\n\t\t\t\t<Start>{}</Start>\n\t\t\t\t<End>{}</End>\n\t\t\t",
                            start_ticks, end_ticks
                        )
                    };

                    self.xml = format!(
                        "{}{}{}",
                        &self.xml[..ti_content_start],
                        new_content,
                        &self.xml[ti_end..],
                    );
                }
            }
        }
    }

    /// Update a VideoClip's InPoint/OutPoint
    fn update_clip_in_out(&mut self, clip_id: &str, in_ticks: u64, out_ticks: u64) {
        let pattern = format!("ObjectID=\"{}\"", clip_id);
        if let Some(clip_pos) = self.xml.find(&pattern) {
            let search_region = &self.xml[clip_pos..clip_pos + 2000.min(self.xml.len() - clip_pos)];

            // Replace InPoint
            if let Some(in_start) = search_region.find("<InPoint>") {
                if let Some(in_end) = search_region[in_start..].find("</InPoint>") {
                    let abs_start = clip_pos + in_start;
                    let abs_end = clip_pos + in_start + in_end + "</InPoint>".len();
                    let replacement = format!("<InPoint>{}</InPoint>", in_ticks);
                    self.xml = format!(
                        "{}{}{}",
                        &self.xml[..abs_start],
                        replacement,
                        &self.xml[abs_end..],
                    );
                }
            }

            // Re-find position since xml changed
            if let Some(clip_pos) = self.xml.find(&pattern) {
                let search_region = &self.xml[clip_pos..clip_pos + 2000.min(self.xml.len() - clip_pos)];
                if let Some(out_start) = search_region.find("<OutPoint>") {
                    if let Some(out_end) = search_region[out_start..].find("</OutPoint>") {
                        let abs_start = clip_pos + out_start;
                        let abs_end = clip_pos + out_start + out_end + "</OutPoint>".len();
                        let replacement = format!("<OutPoint>{}</OutPoint>", out_ticks);
                        self.xml = format!(
                            "{}{}{}",
                            &self.xml[..abs_start],
                            replacement,
                            &self.xml[abs_end..],
                        );
                    }
                }
            }
        }
    }

    /// Write the modified project to a .prproj file
    pub fn write<P: AsRef<Path>>(&self, path: P) -> EditronResult<u64> {
        let path = path.as_ref();
        let file = std::fs::File::create(path)?;
        let mut encoder = GzEncoder::new(file, Compression::new(9));
        encoder.write_all(self.xml.as_bytes())?;
        encoder.finish()?;

        let size = std::fs::metadata(path)?.len();
        Ok(size)
    }
}

/// Convert seconds to Premiere Pro ticks
fn seconds_to_ticks(seconds: f64) -> u64 {
    (seconds * TICKS_PER_SECOND as f64).round() as u64
}

/// Convert Premiere Pro ticks to seconds
#[allow(dead_code)]
fn ticks_to_seconds(ticks: u64) -> f64 {
    ticks as f64 / TICKS_PER_SECOND as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seconds_to_ticks() {
        // 1 second = 254,016,000,000 ticks
        assert_eq!(seconds_to_ticks(1.0), 254_016_000_000);
        assert_eq!(seconds_to_ticks(0.0), 0);
        assert_eq!(seconds_to_ticks(60.0), 15_240_960_000_000);
    }

    #[test]
    fn test_ticks_to_seconds() {
        assert!((ticks_to_seconds(254_016_000_000) - 1.0).abs() < 0.001);
        assert!((ticks_to_seconds(15_240_960_000_000) - 60.0).abs() < 0.001);
    }
}
