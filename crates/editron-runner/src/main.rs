//! Editron Event Recap Pipeline Runner
//!
//! Standalone binary that runs the full Editron Event Recap Forge pipeline:
//!   1. Scene Analysis  — FFmpeg deep analysis of every video clip
//!   2. Beat Analysis   — BPM, beat grid, energy curve, sections from music track
//!   3. Smart Assembly  — Beat-locked timeline with pacing engine
//!   4. Export           — Premiere Pro XML + FFmpeg render script + MP4

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{info, warn};

use services::services::beat_analysis::BeatAnalysisEngine;
use services::services::editron::music::MusicLibrary;
use services::services::recap_assembly::{EditTransition, RecapAssemblyEngine, RecapAssemblyResult};
use services::services::scene_analysis::{
    assign_energy_quartiles, SceneAnalysisEngine, SceneAnalysisResult,
};

/// Editron Event Recap Pipeline Runner
#[derive(Parser)]
#[command(name = "editron-runner")]
#[command(about = "Run the full Editron Event Recap Forge pipeline on a footage directory")]
struct Cli {
    /// Path to the footage directory containing video clips
    #[arg(short = 'i', long)]
    input: PathBuf,

    /// Path to the music track (MP3, WAV, etc.)
    #[arg(short = 'm', long)]
    music: PathBuf,

    /// Output directory for generated files (XML, render script, MP4)
    #[arg(short = 'o', long)]
    output: PathBuf,

    /// Project name for the recap
    #[arg(short = 'n', long, default_value = "Event Recap")]
    name: String,

    /// Target duration in seconds (default: 59s industry standard)
    #[arg(short = 'd', long)]
    duration: Option<f64>,

    /// BPM hint for the music track (auto-detects if not provided)
    #[arg(long)]
    bpm: Option<f64>,

    /// Output resolution width
    #[arg(long, default_value = "3840")]
    width: u32,

    /// Output resolution height
    #[arg(long, default_value = "2160")]
    height: u32,

    /// Scene analysis segment interval in seconds
    #[arg(long, default_value = "1.5")]
    segment_interval: f64,

    /// Maximum parallel scene analysis tasks
    #[arg(long, default_value = "8")]
    parallelism: usize,

    /// Skip scene analysis (use for re-runs with same footage)
    #[arg(long)]
    skip_scene_analysis: bool,

    /// Skip render (only generate XML and script)
    #[arg(long)]
    skip_render: bool,

    /// Event type for music recommendation (e.g. "automotive", "concert", "gala", "parade")
    /// When provided, logs recommended music criteria for the event type.
    #[arg(long)]
    event_type: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "editron_runner=info,services=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let total_start = Instant::now();

    // Validate inputs
    anyhow::ensure!(
        cli.input.exists(),
        "Input directory does not exist: {:?}",
        cli.input
    );
    anyhow::ensure!(
        cli.music.exists(),
        "Music file does not exist: {:?}",
        cli.music
    );

    // Create output directory
    tokio::fs::create_dir_all(&cli.output)
        .await
        .context("Failed to create output directory")?;

    info!("=== EDITRON EVENT RECAP FORGE ===");
    info!("Project: {}", cli.name);
    info!("Input:   {:?}", cli.input);
    info!("Music:   {:?}", cli.music);
    info!("Output:  {:?}", cli.output);
    info!(
        "Target:  {}s @ {}x{}",
        cli.duration.unwrap_or(59.0),
        cli.width,
        cli.height
    );

    // Log music recommendation if event type is specified
    if let Some(ref event_type) = cli.event_type {
        let music_lib = MusicLibrary::new(&cli.output, &PathBuf::from("ffmpeg"));
        let recommendation = music_lib.recommend_for_content(event_type, cli.duration.unwrap_or(59.0));
        info!("Event type: {}", event_type);
        info!("Music recommendation:");
        info!("  Rationale: {}", recommendation.rationale);
        info!("  Genres: {:?}", recommendation.criteria.genres);
        info!("  Moods: {:?}", recommendation.criteria.moods);
        info!(
            "  BPM range: {}-{}",
            recommendation.criteria.min_bpm.unwrap_or(0),
            recommendation.criteria.max_bpm.unwrap_or(0)
        );
        if !recommendation.search_url.is_empty() {
            info!("  Search URL: {}", recommendation.search_url);
        }
    }

    println!();

    // ─── Stage 1: Collect video files ────────────────────────────────────────
    info!("[1/6] BATCH INTAKE — Collecting video files...");
    let all_video_files = collect_video_files(&cli.input).await?;
    info!("Found {} video files total", all_video_files.len());

    // Filter out 0-byte Dropbox placeholders
    let mut video_files = Vec::new();
    let mut placeholder_count = 0usize;
    for path in &all_video_files {
        let meta = tokio::fs::metadata(path).await?;
        if meta.len() > 0 {
            video_files.push(path.clone());
        } else {
            placeholder_count += 1;
        }
    }

    if placeholder_count > 0 {
        warn!(
            "{} files are Dropbox placeholders (0 bytes) — skipping them",
            placeholder_count
        );
    }

    info!("{} video files available for analysis", video_files.len());

    if video_files.is_empty() {
        anyhow::bail!(
            "No synced video files found in {:?}. {} files are Dropbox placeholders.",
            cli.input,
            placeholder_count
        );
    }

    // ─── Stage 2: Scene Analysis ─────────────────────────────────────────────
    let mut scene_result = if cli.skip_scene_analysis {
        info!("[2/6] SCENE ANALYSIS — Skipped (--skip-scene-analysis)");
        build_minimal_scene_result(&video_files)
    } else {
        info!(
            "[2/6] SCENE ANALYSIS — Analyzing {} clips (parallelism: {})...",
            video_files.len(),
            cli.parallelism
        );
        run_scene_analysis(&video_files, cli.segment_interval, cli.parallelism).await?
    };

    info!(
        "Scene analysis complete: {}/{} clips usable",
        scene_result.total_usable, scene_result.total_clips
    );

    // Assign energy quartiles after all clips have been analyzed
    assign_energy_quartiles(&mut scene_result.clips);
    info!("Energy quartiles assigned to {} clips", scene_result.clips.len());

    // Log content type distribution
    {
        let mut type_counts: HashMap<String, u32> = HashMap::new();
        for clip in &scene_result.clips {
            let type_name = format!("{:?}", clip.dominant_content_type);
            *type_counts.entry(type_name).or_insert(0) += 1;
        }
        info!("Content type distribution:");
        for (ct, count) in &type_counts {
            let pct = (*count as f64 / scene_result.clips.len().max(1) as f64) * 100.0;
            info!("  {}: {} ({:.0}%)", ct, count, pct);
        }
    }

    // ─── Stage 3: Beat Analysis (Sonic Engineering) ──────────────────────────
    info!("[3/6] SONIC ENGINEERING — Analyzing beat grid...");
    let beat_start = Instant::now();
    let beat_engine = BeatAnalysisEngine::new();
    let beat_grid = beat_engine
        .analyze(&cli.music, cli.bpm, 4)
        .await
        .context("Beat analysis failed")?;

    info!(
        "Beat analysis complete: {:.1} BPM, {} beats, {} sections ({:.1}s)",
        beat_grid.bpm,
        beat_grid.total_beats,
        beat_grid.sections.len(),
        beat_start.elapsed().as_secs_f64()
    );

    for section in &beat_grid.sections {
        info!(
            "  Section: {} [{:.1}s - {:.1}s] energy={:.2} direction={:?} → {:?}",
            section.name,
            section.start,
            section.end,
            section.energy_level,
            section.energy_direction,
            section.suggested_content
        );
    }

    // ─── Stage 4: Visual QC ─────────────────────────────────────────────────
    info!("[4/6] VISUAL QC — (using scene analysis in-points)");

    // ─── Stage 5: Smart Assembly ─────────────────────────────────────────────
    info!("[5/6] SMART ASSEMBLY — Building beat-locked timeline...");
    let assembly_start = Instant::now();

    let (placements, music_window) = RecapAssemblyEngine::assemble(
        &scene_result,
        &beat_grid,
        &cli.music,
        cli.width,
        cli.height,
        cli.duration,
    );

    let beat_locked_cuts = placements.iter().filter(|p| p.beat_locked).count() as u32;

    info!(
        "Assembly complete: {} clips placed, {} beat-locked cuts, {:.1}s duration ({:.1}s)",
        placements.len(),
        beat_locked_cuts,
        music_window.duration,
        assembly_start.elapsed().as_secs_f64()
    );

    for (i, p) in placements.iter().enumerate() {
        info!(
            "  [{:2}] {:<40} [{:.1}s-{:.1}s] → [{:.1}s-{:.1}s] section={} energy={:.2} {}",
            i + 1,
            p.clip_filename,
            p.source_in,
            p.source_out,
            p.timeline_in,
            p.timeline_out,
            p.section_name,
            p.energy_match_score,
            if p.beat_locked { "BEAT-LOCKED" } else { "" }
        );
    }

    // ─── Diagnostic logging ──────────────────────────────────────────────────
    {
        // Folder coverage
        let unique_folders_used: std::collections::HashSet<&str> = placements
            .iter()
            .filter_map(|p| {
                scene_result
                    .clips
                    .iter()
                    .find(|c| c.filename == p.clip_filename)
                    .map(|c| c.source_folder.as_str())
            })
            .collect();
        let total_folders: std::collections::HashSet<&str> = scene_result
            .clips
            .iter()
            .map(|c| c.source_folder.as_str())
            .collect();
        info!(
            "Folder coverage: {}/{} folders represented ({:.0}%)",
            unique_folders_used.len(),
            total_folders.len(),
            if total_folders.is_empty() {
                0.0
            } else {
                unique_folders_used.len() as f64 / total_folders.len() as f64 * 100.0
            }
        );

        // Shot duration stats
        let shot_durs: Vec<f64> = placements
            .iter()
            .map(|p| p.timeline_out - p.timeline_in)
            .collect();
        if !shot_durs.is_empty() {
            let min_dur = shot_durs
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);
            let max_dur = shot_durs
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            let avg_dur = shot_durs.iter().sum::<f64>() / shot_durs.len() as f64;
            let variance = shot_durs
                .iter()
                .map(|d| (d - avg_dur).powi(2))
                .sum::<f64>()
                / shot_durs.len() as f64;
            let stddev = variance.sqrt();
            info!(
                "Shot durations: min={:.2}s max={:.2}s avg={:.2}s stddev={:.2}s",
                min_dur, max_dur, avg_dur, stddev
            );
        }

        // Max clip reuse count
        let mut clip_use_counts: HashMap<&str, u32> = HashMap::new();
        for p in &placements {
            *clip_use_counts.entry(&p.clip_filename).or_insert(0) += 1;
        }
        let max_reuse = clip_use_counts.values().max().copied().unwrap_or(0);
        info!("Max clip reuse: {}x", max_reuse);

        // Transition type distribution
        let mut trans_counts: HashMap<&str, u32> = HashMap::new();
        for p in &placements {
            let name = match &p.transition_in {
                EditTransition::HardCut => "HardCut",
                EditTransition::Dissolve { .. } => "Dissolve",
                EditTransition::DipToBlack { .. } => "DipToBlack",
                EditTransition::WhipDissolve { .. } => "WhipDissolve",
                EditTransition::AdditiveMix { .. } => "AdditiveMix",
                EditTransition::WhipRight { .. } => "WhipRight",
                EditTransition::SlideLeft { .. } => "SlideLeft",
                EditTransition::SlideRight { .. } => "SlideRight",
                EditTransition::CircleOpen { .. } => "CircleOpen",
                EditTransition::CircleClose { .. } => "CircleClose",
            };
            *trans_counts.entry(name).or_insert(0) += 1;
        }
        info!("Transition distribution:");
        for (name, count) in &trans_counts {
            info!("  {}: {}", name, count);
        }
    }

    // ─── Stage 6: Export ─────────────────────────────────────────────────────
    info!("[6/6] EXPORT — Generating Premiere Pro XML and render script...");

    let safe_name = cli
        .name
        .replace(' ', "_")
        .replace('/', "-")
        .replace('\\', "-");
    let xml_path = cli.output.join(format!("{safe_name}.xml"));
    let script_path = cli.output.join(format!("{safe_name}_render.sh"));
    let mp4_path = cli.output.join(format!("{safe_name}.mp4"));

    let assembly_result = RecapAssemblyResult {
        id: uuid::Uuid::new_v4().to_string(),
        name: cli.name.clone(),
        duration: music_window.duration,
        width: cli.width,
        height: cli.height,
        fps: 29.97,
        bpm: beat_grid.bpm,
        placements: placements.clone(),
        music_path: cli.music.to_string_lossy().to_string(),
        music_window: Some(music_window.clone()),
        xml_path: xml_path.to_string_lossy().to_string(),
        clips_used: placements.len() as u32,
        clips_available: scene_result.total_usable,
        beat_locked_cuts,
        processing_time_ms: total_start.elapsed().as_millis() as u64,
    };

    // Generate Premiere Pro XML
    let xml =
        RecapAssemblyEngine::generate_premiere_xml(&assembly_result, &placements, &beat_grid);
    tokio::fs::write(&xml_path, &xml)
        .await
        .context("Failed to write Premiere XML")?;
    info!("Premiere XML: {:?}", xml_path);

    // Generate render script
    let render_script = RecapAssemblyEngine::generate_render_script(
        &placements,
        &cli.music.to_string_lossy(),
        &music_window,
        &mp4_path.to_string_lossy(),
    );
    tokio::fs::write(&script_path, &render_script)
        .await
        .context("Failed to write render script")?;

    // Make render script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&script_path, perms)?;
    }
    info!("Render script: {:?}", script_path);

    // Write assembly result JSON for debugging/inspection
    let json_path = cli.output.join(format!("{safe_name}_assembly.json"));
    let json = serde_json::to_string_pretty(&assembly_result)?;
    tokio::fs::write(&json_path, &json).await?;
    info!("Assembly JSON: {:?}", json_path);

    // ─── Optional: Execute render ────────────────────────────────────────────
    if !cli.skip_render {
        info!("Rendering MP4 with FFmpeg...");
        let render_start = Instant::now();

        let status = tokio::process::Command::new("bash")
            .arg(&script_path)
            .status()
            .await
            .context("Failed to execute render script")?;

        if status.success() {
            info!(
                "Render complete: {:?} ({:.1}s)",
                mp4_path,
                render_start.elapsed().as_secs_f64()
            );
        } else {
            warn!("Render script exited with status: {}", status);
            warn!("You can re-run manually: bash {:?}", script_path);
        }
    } else {
        info!("Render skipped (--skip-render). Run manually:");
        info!("  bash {:?}", script_path);
    }

    // ─── Summary ─────────────────────────────────────────────────────────────
    let total_elapsed = total_start.elapsed();
    println!();
    println!("╔══════════════════════════════════════════════════╗");
    println!("║         EDITRON RECAP FORGE — COMPLETE          ║");
    println!("╠══════════════════════════════════════════════════╣");
    println!("║ Project:      {:<35}║", cli.name);
    println!(
        "║ Duration:     {:<35}║",
        format!("{:.1}s", music_window.duration)
    );
    println!(
        "║ Resolution:   {:<35}║",
        format!("{}x{}", cli.width, cli.height)
    );
    println!(
        "║ BPM:          {:<35}║",
        format!("{:.1}", beat_grid.bpm)
    );
    println!(
        "║ Clips used:   {:<35}║",
        format!("{}/{}", placements.len(), scene_result.total_usable)
    );
    println!(
        "║ Beat-locked:  {:<35}║",
        format!("{} cuts", beat_locked_cuts)
    );
    println!(
        "║ Total time:   {:<35}║",
        format!("{:.1}s", total_elapsed.as_secs_f64())
    );
    println!("╠══════════════════════════════════════════════════╣");
    println!("║ Outputs:                                        ║");
    println!("║  XML:    {:<40}║", xml_path.display());
    println!("║  Script: {:<40}║", script_path.display());
    println!("║  JSON:   {:<40}║", json_path.display());
    if !cli.skip_render {
        println!("║  MP4:    {:<40}║", mp4_path.display());
    }
    println!("╚══════════════════════════════════════════════════╝");

    Ok(())
}

/// Recursively collect all video files from a directory
async fn collect_video_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let dir = dir.to_path_buf();
    let files = tokio::task::spawn_blocking(move || {
        let mut files = Vec::new();
        let mut stack = vec![dir];
        while let Some(current) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(&current) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if !path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .starts_with('.')
                        {
                            stack.push(path);
                        }
                    } else if is_video_file(&path) {
                        files.push(path);
                    }
                }
            }
        }
        files.sort();
        files
    })
    .await?;
    Ok(files)
}

fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            matches!(
                e.to_lowercase().as_str(),
                "mp4" | "mov" | "avi" | "mkv" | "mxf" | "m4v" | "mts" | "m2ts"
            )
        })
        .unwrap_or(false)
}

/// Run parallel scene analysis on all video clips
async fn run_scene_analysis(
    files: &[PathBuf],
    segment_interval: f64,
    parallelism: usize,
) -> Result<SceneAnalysisResult> {
    let scene_engine = SceneAnalysisEngine::new();
    let start = Instant::now();

    // Process in batches to limit parallelism
    let mut clip_analyses = Vec::new();
    for chunk in files.chunks(parallelism) {
        let mut handles = Vec::new();

        for path in chunk {
            let engine = scene_engine.clone();
            let path = path.clone();
            handles.push(tokio::spawn(async move {
                let result = engine.analyze_clip(&path, segment_interval).await;
                (path, result)
            }));
        }

        for handle in handles {
            match handle.await {
                Ok((_path, Ok(analysis))) => {
                    info!(
                        "  Analyzed: {} — energy={:.2} type={:?} quality={:.2} usable={}",
                        analysis.filename,
                        analysis.overall_energy,
                        analysis.dominant_content_type,
                        analysis.quality_score,
                        analysis.usable
                    );
                    clip_analyses.push(analysis);
                }
                Ok((path, Err(e))) => {
                    warn!(
                        "  FAILED: {:?} — {}",
                        path.file_name().unwrap_or_default(),
                        e
                    );
                }
                Err(e) => {
                    warn!("  Task panicked: {}", e);
                }
            }
        }
    }

    let total_usable = clip_analyses.iter().filter(|c| c.usable).count() as u32;

    info!(
        "Scene analysis took {:.1}s for {} clips",
        start.elapsed().as_secs_f64(),
        clip_analyses.len()
    );

    Ok(SceneAnalysisResult {
        batch_id: format!(
            "editron-runner-{}",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        ),
        total_clips: clip_analyses.len() as u32,
        total_usable,
        clips: clip_analyses,
        processing_time_ms: start.elapsed().as_millis() as u64,
    })
}

/// Build a minimal SceneAnalysisResult without actual analysis
fn build_minimal_scene_result(files: &[PathBuf]) -> SceneAnalysisResult {
    use services::services::scene_analysis::{ClipAnalysis, ContentType};

    let clips: Vec<ClipAnalysis> = files
        .iter()
        .map(|path| {
            let source_folder = path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            ClipAnalysis {
                filename: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                path: path.clone(),
                duration: 10.0, // placeholder
                width: 3840,
                height: 2160,
                fps: 29.97,
                segments: vec![],
                overall_energy: 0.5,
                peak_energy_timestamp: 0.0,
                dominant_content_type: ContentType::Ambient,
                usable: true,
                source_folder,
                quality_score: 0.5,
                energy_quartile: 0,
                hard_cuts: vec![],
            }
        })
        .collect();

    SceneAnalysisResult {
        batch_id: "minimal-no-analysis".to_string(),
        total_clips: clips.len() as u32,
        total_usable: clips.len() as u32,
        clips,
        processing_time_ms: 0,
    }
}
