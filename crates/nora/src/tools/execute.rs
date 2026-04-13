//! Tool execution entry point with permission checking

use chrono::Utc;

use super::{types::*, ExecutiveTools};

#[allow(dead_code)]
impl ExecutiveTools {
    pub async fn execute_tool(
        &self,
        tool: NoraExecutiveTool,
        user_permissions: Vec<Permission>,
    ) -> crate::Result<ToolExecutionResult> {
        let start_time = std::time::Instant::now();
        let tool_name = self.get_tool_name(&tool);
        let execution_id = uuid::Uuid::new_v4().to_string();

        // Check permissions
        if let Some(tool_def) = self.available_tools.get(&tool_name) {
            for required_permission in &tool_def.required_permissions {
                if !user_permissions.contains(required_permission) {
                    return Ok(ToolExecutionResult {
                        tool_name,
                        execution_id,
                        status: ExecutionStatus::Failed,
                        result_data: None,
                        error_message: Some(format!(
                            "Missing required permission: {:?}",
                            required_permission
                        )),
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        timestamp: Utc::now(),
                    });
                }
            }
        }

        // Execute tool
        let result_data = self.execute_tool_implementation(tool).await?;

        Ok(ToolExecutionResult {
            tool_name,
            execution_id,
            status: ExecutionStatus::Success,
            result_data: Some(result_data),
            error_message: None,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            timestamp: Utc::now(),
        })
    }

    pub(super) fn initialize_tools(&mut self) {
        // Project Management tools
        self.add_tool_definition(ToolDefinition {
            name: "create_project".to_string(),
            description: "Create a new project with git repository".to_string(),
            category: ToolCategory::Planning,
            parameters: vec![
                ToolParameter {
                    name: "name".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Project name".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "git_repo_path".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Path to git repository".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Executive, Permission::Write],
            estimated_duration: Some("1-2 minutes".to_string()),
        });

        self.add_tool_definition(ToolDefinition {
            name: "create_board".to_string(),
            description: "Create a new kanban board for a project".to_string(),
            category: ToolCategory::Coordination,
            parameters: vec![
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Project UUID".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "name".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Board name".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Write, Permission::Execute],
            estimated_duration: Some("30 seconds".to_string()),
        });

        self.add_tool_definition(ToolDefinition {
            name: "create_task_on_board".to_string(),
            description: "Create a new task on a specific kanban board".to_string(),
            category: ToolCategory::Coordination,
            parameters: vec![
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Project UUID".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "board_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Board UUID".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "title".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Task title".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "description".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Task description".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "priority".to_string(),
                    parameter_type: ParameterType::Enum(vec![
                        "low".to_string(),
                        "medium".to_string(),
                        "high".to_string(),
                    ]),
                    description: "Task priority level".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("medium")),
                },
            ],
            required_permissions: vec![Permission::Write],
            estimated_duration: Some("10-30 seconds".to_string()),
        });

        // Coordination tools
        self.add_tool_definition(ToolDefinition {
            name: "coordinate_team_meeting".to_string(),
            description: "Schedule and coordinate team meetings with agenda".to_string(),
            category: ToolCategory::Coordination,
            parameters: vec![
                ToolParameter {
                    name: "participants".to_string(),
                    parameter_type: ParameterType::Array,
                    description: "List of meeting participants".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "agenda".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Meeting agenda".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Write, Permission::Execute],
            estimated_duration: Some("2-5 minutes".to_string()),
        });

        // Task delegation tool - critical for orchestrating other agents
        self.add_tool_definition(ToolDefinition {
            name: "delegate_task".to_string(),
            description: "Delegate a task to another agent (like AURI for coding) and trigger execution. Use this to assign work to specialized agents.".to_string(),
            category: ToolCategory::Coordination,
            parameters: vec![
                ToolParameter {
                    name: "task_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "The UUID of the task to delegate".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "assignee".to_string(),
                    parameter_type: ParameterType::String,
                    description: "The agent name to assign the task to (e.g., 'Auri' for coding, 'Editron' for video editing)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "priority".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Task priority: low, medium, high, or critical".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("medium")),
                },
            ],
            required_permissions: vec![Permission::Write, Permission::Execute],
            estimated_duration: Some("1-5 seconds to start, execution time varies".to_string()),
        });

        // Analysis tools
        self.add_tool_definition(ToolDefinition {
            name: "generate_kpi_dashboard".to_string(),
            description: "Generate KPI dashboard with specified metrics".to_string(),
            category: ToolCategory::Analysis,
            parameters: vec![ToolParameter {
                name: "metrics".to_string(),
                parameter_type: ParameterType::Array,
                description: "List of metrics to include".to_string(),
                required: true,
                default_value: None,
            }],
            required_permissions: vec![Permission::ReadOnly],
            estimated_duration: Some("30 seconds - 2 minutes".to_string()),
        });

        // Media production tools
        self.add_tool_definition(ToolDefinition {
            name: "ingest_media_batch".to_string(),
            description: "Ingest a batch of raw media from Dropbox, other capture sources, or a local directory path"
                .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "source_url".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Public or signed URL to the capture folder, or local directory path (e.g. /path/to/footage)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "reference_name".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Human readable reference for the batch".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "storage_tier".to_string(),
                    parameter_type: ParameterType::Enum(vec![
                        "hot".to_string(),
                        "warm".to_string(),
                        "cold".to_string(),
                    ]),
                    description: "Target storage tier".to_string(),
                    required: true,
                    default_value: Some(serde_json::json!("hot")),
                },
                ToolParameter {
                    name: "checksum_required".to_string(),
                    parameter_type: ParameterType::Boolean,
                    description: "Whether to verify checksums before import".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(true)),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID to log pipeline tasks under".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "task_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional workflow task ID to attach artifacts and activity to".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Write],
            estimated_duration: Some("2-6 minutes depending on payload".to_string()),
        });

        self.add_tool_definition(ToolDefinition {
            name: "analyze_media_batch".to_string(),
            description:
                "Run iterative analysis on an ingested batch to extract highlights and notes"
                    .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "batch_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Ingest batch identifier".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "brief".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Creative brief or prompt".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "passes".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Number of iterative passes".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(2)),
                },
                ToolParameter {
                    name: "deliverable_targets".to_string(),
                    parameter_type: ParameterType::Array,
                    description: "List of deliverables to scout for".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(["recap", "highlights"])),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID to log analysis tasks".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "task_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional workflow task ID to attach artifacts and activity to"
                        .to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Write, Permission::Execute],
            estimated_duration: Some("5-15 minutes".to_string()),
        });

        self.add_tool_definition(ToolDefinition {
            name: "generate_video_edits".to_string(),
            description: "Create edit timelines and drafts for specified deliverables".to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "batch_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Source batch identifier".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "deliverable_type".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Type of edit (recap, highlight, sizzle)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "aspect_ratios".to_string(),
                    parameter_type: ParameterType::Array,
                    description: "Target aspect ratios".to_string(),
                    required: true,
                    default_value: Some(serde_json::json!(["16:9", "9:16"])),
                },
                ToolParameter {
                    name: "reference_style".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional reference edit or look".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "include_captions".to_string(),
                    parameter_type: ParameterType::Boolean,
                    description: "Auto-generate caption tracks".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(true)),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID to log edit tasks".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "task_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional workflow task ID to attach artifacts and activity to"
                        .to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute, Permission::Executive],
            estimated_duration: Some("10-20 minutes".to_string()),
        });

        self.add_tool_definition(ToolDefinition {
            name: "render_video_deliverables".to_string(),
            description: "Render and distribute finished edits to downstream destinations"
                .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "edit_session_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Edit session identifier".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "destinations".to_string(),
                    parameter_type: ParameterType::Array,
                    description: "List of delivery endpoints (Dropbox, Frame.io, S3 folder, etc.)"
                        .to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "formats".to_string(),
                    parameter_type: ParameterType::Array,
                    description: "Export formats or presets".to_string(),
                    required: true,
                    default_value: Some(serde_json::json!(["ProRes422", "H.264"])),
                },
                ToolParameter {
                    name: "priority".to_string(),
                    parameter_type: ParameterType::Enum(vec![
                        "low".to_string(),
                        "standard".to_string(),
                        "rush".to_string(),
                    ]),
                    description: "Render queue priority".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("standard")),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID to log render tasks".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "task_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional workflow task ID to attach artifacts and activity to"
                        .to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("5-30 minutes depending on outputs".to_string()),
        });

        self.add_tool_definition(ToolDefinition {
            name: "run_visual_qc".to_string(),
            description:
                "Run Spectra Visual QC pass — vision-guided frame analysis to score composition and select optimal in-points"
                    .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "batch_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Media batch identifier (must be Ready)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "candidates_per_clip".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Number of candidate frames per clip".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(5)),
                },
                ToolParameter {
                    name: "min_composition_score".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Minimum composition score to pass QC (0.0-1.0)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(0.6)),
                },
                ToolParameter {
                    name: "target_aspect_ratio".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Target delivery aspect ratio (e.g. '16:9')".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID to associate this QC pass".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("2-15 minutes depending on clip count".to_string()),
        });

        // Scene Analysis tool
        self.add_tool_definition(ToolDefinition {
            name: "analyze_scenes".to_string(),
            description:
                "Deep scene analysis using FFmpeg — measures brightness, motion intensity, complexity, and classifies content type (high_energy, establishing, intimate, transition, ambient) per segment. Run BEFORE hero selection to understand footage."
                    .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "batch_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Media batch identifier (must be Ready)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "segment_interval".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Seconds between analysis samples (default 3.0)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(3.0)),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("1-5 minutes depending on clip count".to_string()),
        });

        // Beat Grid Analysis tool
        self.add_tool_definition(ToolDefinition {
            name: "analyze_beat_grid".to_string(),
            description:
                "Analyze audio track for BPM, beat grid, energy curve, music sections (intro/verse/chorus/bridge/outro), and transition markers. Sonix Engineering step — marks beats for beat-locked cuts."
                    .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "audio_path".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Path to the audio track file".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "bpm_hint".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Optional BPM hint to guide detection".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "beats_per_bar".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Beats per bar (default 4)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(4)),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("30 seconds - 2 minutes".to_string()),
        });

        // Assemble Recap Edit tool
        self.add_tool_definition(ToolDefinition {
            name: "assemble_recap_edit".to_string(),
            description:
                "Full recap assembly: runs scene analysis + beat grid analysis, matches clips to music sections by energy/content, beat-locks all cuts, mutes NAT audio (music only), and exports Premiere Pro XML. The complete Editron pipeline."
                    .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "batch_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Media batch identifier (must be Ready)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "audio_path".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Path to the music track".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "bpm_hint".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Optional BPM hint for beat detection".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "target_aspect_ratio".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Target aspect ratio (e.g. '16:9', '9:16', '1:1')".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("16:9")),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "project_name".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Project name used for output filenames and XML metadata (e.g. 'MOPAR Car Show')".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("Recap")),
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("3-10 minutes".to_string()),
        });

        // Execute Render Script tool
        self.add_tool_definition(ToolDefinition {
            name: "execute_render_script".to_string(),
            description:
                "Execute an FFmpeg render script produced by AssembleRecapEdit. Runs the bash script and produces the final MP4 deliverable."
                    .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "render_script".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Path to the render.sh script".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "render_output".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Expected output MP4 path".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("output.mp4")),
                },
                ToolParameter {
                    name: "xml_path".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Path to the Premiere XML (passed through for reference)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("")),
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("1-5 minutes".to_string()),
        });

        // Image Generation tool (fal.ai FLUX Pro Ultra)
        self.add_tool_definition(ToolDefinition {
            name: "generate_image".to_string(),
            description: "Generate a high-quality AI image using fal.ai FLUX Pro Ultra. Supports text-to-image and image-to-image (reference_image_url). Use raw_mode=true for photorealistic portraits. Costs 50 VIBE per image.".to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "prompt".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Text prompt describing the desired image".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "reference_image_url".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional URL to a reference image for img2img mode".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "aspect_ratio".to_string(),
                    parameter_type: ParameterType::Enum(vec![
                        "1:1".to_string(),
                        "16:9".to_string(),
                        "9:16".to_string(),
                        "4:3".to_string(),
                        "3:4".to_string(),
                        "21:9".to_string(),
                    ]),
                    description: "Output aspect ratio (default 3:4 for portraits)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("3:4")),
                },
                ToolParameter {
                    name: "raw_mode".to_string(),
                    parameter_type: ParameterType::Boolean,
                    description: "Enable raw photographic mode for ultra-realistic output (recommended for portraits)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(true)),
                },
                ToolParameter {
                    name: "seed".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Optional seed for reproducibility".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "model".to_string(),
                    parameter_type: ParameterType::String,
                    description: "fal.ai model ID (default: fal-ai/flux-pro/v1.1-ultra)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("fal-ai/flux-pro/v1.1-ultra")),
                },
                ToolParameter {
                    name: "output_filename".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Output filename (without extension). Saved to dev_assets/video_gen/portraits/".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "task_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional task UUID to log this generation against".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID for VIBE billing context".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("20-60 seconds".to_string()),
        });

        // Apply Video Effect tool (FFmpeg presets)
        self.add_tool_definition(ToolDefinition {
            name: "apply_video_effect".to_string(),
            description: "Apply a cinematic post-processing effect to a video file using FFmpeg. Presets: hologram_glitch (cyberpunk RGB split + scanlines + blue tint + fade), color_grade (cinematic LUT), vignette (soft edge darkening). Costs 100 VIBE per render.".to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "input_path".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Path to the input video file".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "effect".to_string(),
                    parameter_type: ParameterType::Enum(vec![
                        "hologram_glitch".to_string(),
                        "color_grade".to_string(),
                        "vignette".to_string(),
                    ]),
                    description: "Effect preset to apply".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "output_path".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Output video path (defaults to input_path with _fx suffix)".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "task_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional task UUID to log this render against".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID for VIBE billing context".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("1-5 minutes depending on video length".to_string()),
        });

        // Post-Process Video Job tool (chained effects on HeyGen output)
        self.add_tool_definition(ToolDefinition {
            name: "post_process_video_job".to_string(),
            description: "Apply a chain of post-processing effects to a completed HeyGen video job. Looks up the video_job by ID, downloads the output, applies effects sequentially, and saves the final deliverable. Costs 100 VIBE per effect applied.".to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "video_job_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "UUID of the completed VideoJob record".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "effects".to_string(),
                    parameter_type: ParameterType::Array,
                    description: "Ordered list of effects to apply (e.g. [\"hologram_glitch\", \"vignette\"])".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "task_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional task UUID to log this pipeline against".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID for VIBE billing context".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute, Permission::Executive],
            estimated_duration: Some("2-10 minutes depending on video length and effect count".to_string()),
        });
    }

    fn add_tool_definition(&mut self, tool_def: ToolDefinition) {
        self.available_tools.insert(tool_def.name.clone(), tool_def);
    }

    pub(crate) fn get_tool_name(&self, tool: &NoraExecutiveTool) -> String {
        match tool {
            // Project Management
            NoraExecutiveTool::CreateProject { .. } => "create_project".to_string(),
            NoraExecutiveTool::CreateBoard { .. } => "create_board".to_string(),
            NoraExecutiveTool::CreateTaskInProject { .. } => "create_task".to_string(),
            NoraExecutiveTool::GetProjectTasks { .. } => "get_project_tasks".to_string(),
            NoraExecutiveTool::GetProjectDetails { .. } => "get_project_details".to_string(),
            NoraExecutiveTool::DeleteProject { .. } => "delete_project".to_string(),
            NoraExecutiveTool::UpdateProject { .. } => "update_project".to_string(),
            NoraExecutiveTool::CreateTaskOnBoard { .. } => "create_task_on_board".to_string(),
            NoraExecutiveTool::AddTaskToBoard { .. } => "add_task_to_board".to_string(),
            NoraExecutiveTool::ExecuteWorkflow { .. } => "execute_workflow".to_string(),
            NoraExecutiveTool::DispatchAgentsParallel { .. } => {
                "dispatch_agents_parallel".to_string()
            }
            NoraExecutiveTool::CancelWorkflow { .. } => "cancel_workflow".to_string(),
            NoraExecutiveTool::ListActiveWorkflows => "list_active_workflows".to_string(),
            NoraExecutiveTool::ListAvailableWorkflows { .. } => {
                "list_available_workflows".to_string()
            }

            // Coordination
            NoraExecutiveTool::CoordinateTeamMeeting { .. } => {
                "coordinate_team_meeting".to_string()
            }
            NoraExecutiveTool::DelegateTask { .. } => "delegate_task".to_string(),
            NoraExecutiveTool::EscalateIssue { .. } => "escalate_issue".to_string(),

            // Planning
            NoraExecutiveTool::GenerateProjectRoadmap { .. } => {
                "generate_project_roadmap".to_string()
            }
            NoraExecutiveTool::GenerateKPIDashboard { .. } => "generate_kpi_dashboard".to_string(),
            NoraExecutiveTool::IngestMediaBatch { .. } => "ingest_media_batch".to_string(),
            NoraExecutiveTool::AnalyzeMediaBatch { .. } => "analyze_media_batch".to_string(),
            NoraExecutiveTool::GenerateVideoEdits { .. } => "generate_video_edits".to_string(),
            NoraExecutiveTool::RenderVideoDeliverables { .. } => {
                "render_video_deliverables".to_string()
            }
            NoraExecutiveTool::RunVisualQc { .. } => "run_visual_qc".to_string(),
            NoraExecutiveTool::AnalyzeScenes { .. } => "analyze_scenes".to_string(),
            NoraExecutiveTool::AnalyzeBeatGrid { .. } => "analyze_beat_grid".to_string(),
            NoraExecutiveTool::AssembleRecapEdit { .. } => "assemble_recap_edit".to_string(),
            NoraExecutiveTool::ExecuteRenderScript { .. } => "execute_render_script".to_string(),
            NoraExecutiveTool::SearchMusic { .. } => "search_music".to_string(),
            NoraExecutiveTool::DownloadMusicTrack { .. } => "download_music_track".to_string(),
            NoraExecutiveTool::RecommendMusicForVideo { .. } => {
                "recommend_music_for_video".to_string()
            }
            NoraExecutiveTool::PreviewMusicTrack { .. } => "preview_music_track".to_string(),
            NoraExecutiveTool::GetMusicTrackDetails { .. } => "get_music_track_details".to_string(),
            NoraExecutiveTool::AnalyzeMusicTrack { .. } => "analyze_music_track".to_string(),

            NoraExecutiveTool::GenerateImage { .. } => "generate_image".to_string(),
            NoraExecutiveTool::ApplyVideoEffect { .. } => "apply_video_effect".to_string(),
            NoraExecutiveTool::PostProcessVideoJob { .. } => "post_process_video_job".to_string(),

            // Add more mappings...
            _ => "unknown_tool".to_string(),
        }
    }
}
