//! OpenAI-compatible tool schema generation

use super::ExecutiveTools;

#[allow(dead_code)]
impl ExecutiveTools {
    /// Generate OpenAI-compatible function schemas for available tools
    /// These are used for function calling / tool use
    pub fn get_openai_tool_schemas() -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "create_project",
                    "description": "Create a new project in the system. Use this when the user wants to create, start, or set up a new project.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "The name of the project to create"
                            },
                            "git_repo_path": {
                                "type": "string",
                                "description": "Optional path to a git repository for this project"
                            }
                        },
                        "required": ["name"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "create_board",
                    "description": "Create a new board (kanban, scrum, etc.) within a project. Use this when the user wants to add a board to organize tasks.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "project_id": {
                                "type": "string",
                                "description": "The UUID of the project to add the board to"
                            },
                            "name": {
                                "type": "string",
                                "description": "The name of the board"
                            },
                            "description": {
                                "type": "string",
                                "description": "Optional description of the board's purpose"
                            },
                            "board_type": {
                                "type": "string",
                                "enum": ["kanban", "scrum", "custom"],
                                "description": "The type of board to create"
                            }
                        },
                        "required": ["project_id", "name"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "create_task",
                    "description": "Create a new task in a project. Use this when the user wants to add a task, todo, or work item to an EXISTING project. The task will be added to the project's default board.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "project_name": {
                                "type": "string",
                                "description": "The name of the existing project to add the task to (e.g., 'Test Project 3')"
                            },
                            "title": {
                                "type": "string",
                                "description": "The title of the task to create"
                            },
                            "description": {
                                "type": "string",
                                "description": "Optional detailed description of the task"
                            },
                            "priority": {
                                "type": "string",
                                "enum": ["low", "medium", "high", "critical"],
                                "description": "Priority level of the task (default: medium)"
                            }
                        },
                        "required": ["project_name", "title"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "get_project_tasks",
                    "description": "Get all tasks for a project. Use this FIRST when the user asks to review, analyze, check, or look at tasks in a project. This returns the actual task data for you to analyze and summarize.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "project_name": {
                                "type": "string",
                                "description": "The name of the project to get tasks from (e.g., 'PRIME', 'PCG')"
                            },
                            "status_filter": {
                                "type": "string",
                                "enum": ["todo", "in_progress", "done", "blocked"],
                                "description": "Optional filter to only get tasks with this status"
                            }
                        },
                        "required": ["project_name"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "get_project_details",
                    "description": "Get detailed information about a project including its tasks, boards, and pods. Use this when the user asks for a project overview or summary.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "project_name": {
                                "type": "string",
                                "description": "The name of the project to get details for"
                            }
                        },
                        "required": ["project_name"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "delete_project",
                    "description": "Permanently delete a project and all its tasks. Use when the user explicitly asks to delete or remove a project.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "project_name": {
                                "type": "string",
                                "description": "Exact name of the project to delete"
                            }
                        },
                        "required": ["project_name"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "update_project",
                    "description": "Update a project's name or description.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "project_name": {
                                "type": "string",
                                "description": "Current name of the project to update"
                            },
                            "new_name": {
                                "type": "string",
                                "description": "New name for the project (optional)"
                            },
                            "new_description": {
                                "type": "string",
                                "description": "New description for the project (optional)"
                            }
                        },
                        "required": ["project_name"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "execute_workflow",
                    "description": "Execute a multi-stage agent workflow. Use this when the user requests a complex operation that involves multiple coordinated steps. IMPORTANT: For research agents (scout-research, oracle-strategy), you MUST include the user's original request/topic in the inputs.request field so the agent knows what to research.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "agent_id": {
                                "type": "string",
                                "description": "The agent identifier. Creative: 'editron-post' (video editing), 'master-cinematographer' (AI video). Strategy: 'astra-strategy' (roadmaps). Social: 'scout-research' (research), 'oracle-strategy' (content strategy), 'muse-creative' (copywriting), 'herald-distribution' (publishing), 'echo-engagement' (community)",
                                "enum": ["editron-post", "master-cinematographer", "astra-strategy", "scout-research", "oracle-strategy", "muse-creative", "herald-distribution", "echo-engagement"]
                            },
                            "workflow_id": {
                                "type": "string",
                                "description": "The workflow identifier (e.g., 'competitor-deep-dive' for Scout research, 'content-calendar-30day' for Oracle strategy)"
                            },
                            "project_id": {
                                "type": "string",
                                "description": "Optional project ID to associate the workflow with"
                            },
                            "inputs": {
                                "type": "object",
                                "description": "Input parameters for the workflow. For research workflows, MUST include 'request' with the user's full research topic/question (e.g., {'request': 'research viral content trends for hospitality industry', 'target': 'Prime Hospitality competitors'})",
                                "properties": {
                                    "request": {
                                        "type": "string",
                                        "description": "The user's original request or research topic - REQUIRED for research agents"
                                    },
                                    "target": {
                                        "type": "string",
                                        "description": "Specific target/subject to research"
                                    }
                                },
                                "additionalProperties": true
                            }
                        },
                        "required": ["agent_id", "workflow_id", "inputs"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "cancel_workflow",
                    "description": "Cancel a running workflow execution. Use this when a workflow is stuck, failing repeatedly, or needs to be stopped.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "workflow_instance_id": {
                                "type": "string",
                                "description": "The UUID of the workflow instance to cancel"
                            }
                        },
                        "required": ["workflow_instance_id"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "list_active_workflows",
                    "description": "List all currently running workflow executions with their status, agent, and instance IDs.",
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "list_available_workflows",
                    "description": "List all available workflow templates that can be executed. Use this to discover which workflows exist for each agent before calling execute_workflow. Optionally filter by agent_id to see workflows for a specific agent.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "agent_id": {
                                "type": "string",
                                "description": "Optional agent ID to filter workflows (e.g., 'master-cinematographer', 'editron-post')"
                            }
                        }
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "send_email",
                    "description": "Send an email to one or more recipients. Use this when the user wants to send, compose, or draft an email.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "to": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "List of email addresses to send to"
                            },
                            "subject": {
                                "type": "string",
                                "description": "Email subject line"
                            },
                            "body": {
                                "type": "string",
                                "description": "Email body content"
                            }
                        },
                        "required": ["to", "subject", "body"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "read_inbox",
                    "description": "Read email messages from Nora's inbox (nora@powerclubglobal.com) or, if specified, an organisation/project inbox. Use this when asked to check, read, or summarise emails.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "limit": {
                                "type": "integer",
                                "description": "Number of messages to retrieve (default 10, max 50)",
                                "default": 10
                            },
                            "owner_type": {
                                "type": "string",
                                "enum": ["agent", "organization", "project", "user"],
                                "description": "Whose inbox to read. Omit to default to Nora's own inbox."
                            },
                            "owner_id": {
                                "type": "string",
                                "description": "UUID of the owner (required when owner_type is set)"
                            }
                        },
                        "required": []
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "send_sms",
                    "description": "Send an SMS text message from Nora's phone number (+14053008311). Use this when asked to text, SMS, or send a message to a phone number.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "to": {
                                "type": "string",
                                "description": "Recipient phone number in E.164 format (e.g. +14155551234)"
                            },
                            "message": {
                                "type": "string",
                                "description": "The SMS message text (keep under 1600 characters)"
                            }
                        },
                        "required": ["to", "message"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "send_discord_message",
                    "description": "Send a message to Discord via webhook. Use this when the user wants to post or send something to Discord.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "message": {
                                "type": "string",
                                "description": "The message content to send"
                            },
                            "mentions": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Optional user IDs to mention"
                            }
                        },
                        "required": ["message"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "create_calendar_event",
                    "description": "Create a calendar event or meeting. Use this when the user wants to schedule, book, or create a meeting or event.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "title": {
                                "type": "string",
                                "description": "Title of the event"
                            },
                            "start_time": {
                                "type": "string",
                                "description": "Start time in ISO 8601 format (e.g., 2024-12-05T14:00:00Z)"
                            },
                            "end_time": {
                                "type": "string",
                                "description": "End time in ISO 8601 format"
                            },
                            "description": {
                                "type": "string",
                                "description": "Optional event description"
                            },
                            "attendees": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Optional list of attendee email addresses"
                            }
                        },
                        "required": ["title", "start_time", "end_time"]
                    }
                }
            }),
            // Editron / Media Pipeline Tools
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "ingest_media_batch",
                    "description": "Ingest a batch of media files from a URL (e.g., Dropbox) or a local directory path. Use this when the user provides video content or asks to analyze/edit videos from a link or local folder.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "source_url": {
                                "type": "string",
                                "description": "URL to the media source (Dropbox link, etc.) or local directory path (e.g. /path/to/footage)"
                            },
                            "reference_name": {
                                "type": "string",
                                "description": "Optional reference name for this batch (e.g., 'Jan 25 Event Footage')"
                            },
                            "storage_tier": {
                                "type": "string",
                                "enum": ["hot", "warm", "cold"],
                                "description": "Storage tier for the media (hot=active, warm=archive, cold=deep archive)"
                            },
                            "checksum_required": {
                                "type": "boolean",
                                "description": "Whether to verify file checksums during download"
                            },
                            "project_id": {
                                "type": "string",
                                "description": "Optional project ID to associate this batch with and create a task"
                            },
                            "task_id": {
                                "type": "string",
                                "description": "Optional workflow task ID to attach artifacts and activity to"
                            }
                        },
                        "required": ["source_url", "storage_tier", "checksum_required"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "analyze_media_batch",
                    "description": "Analyze a media batch for video editing. Identifies hero moments, suggests cuts, and prepares for editing.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "batch_id": {
                                "type": "string",
                                "description": "UUID of the media batch to analyze"
                            },
                            "brief": {
                                "type": "string",
                                "description": "Creative brief or editing instructions (e.g., 'Create a highlight reel showcasing the best moments')"
                            },
                            "deliverable_targets": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Target deliverables (e.g., ['recap', 'highlight', 'social'])"
                            },
                            "passes": {
                                "type": "integer",
                                "description": "Number of analysis passes to perform (1-3)"
                            },
                            "project_id": {
                                "type": "string",
                                "description": "Optional project ID"
                            },
                            "task_id": {
                                "type": "string",
                                "description": "Optional workflow task ID to attach artifacts and activity to"
                            }
                        },
                        "required": ["batch_id", "brief", "deliverable_targets", "passes"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "generate_video_edits",
                    "description": "Generate video edits from analyzed media. Creates edit sessions with timeline structures.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "batch_id": {
                                "type": "string",
                                "description": "UUID of the analyzed media batch"
                            },
                            "deliverable_type": {
                                "type": "string",
                                "description": "Type of deliverable (e.g., 'recap', 'highlight', 'social')"
                            },
                            "aspect_ratios": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Target aspect ratios (e.g., ['16:9', '9:16', '1:1'])"
                            },
                            "reference_style": {
                                "type": "string",
                                "description": "Optional reference style or template to follow"
                            },
                            "include_captions": {
                                "type": "boolean",
                                "description": "Whether to include automatic captions"
                            },
                            "project_id": {
                                "type": "string",
                                "description": "Optional project ID"
                            },
                            "task_id": {
                                "type": "string",
                                "description": "Optional workflow task ID to attach artifacts and activity to"
                            }
                        },
                        "required": ["batch_id", "deliverable_type", "aspect_ratios", "include_captions"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "render_video_deliverables",
                    "description": "Render final video deliverables from an edit session. Creates render jobs for export.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "edit_session_id": {
                                "type": "string",
                                "description": "UUID of the edit session to render"
                            },
                            "destinations": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Export destinations (e.g., ['local', 'youtube', 'instagram'])"
                            },
                            "formats": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Output formats (e.g., ['mp4', 'mov'])"
                            },
                            "priority": {
                                "type": "string",
                                "enum": ["low", "standard", "rush"],
                                "description": "Render priority"
                            },
                            "project_id": {
                                "type": "string",
                                "description": "Optional project ID"
                            },
                            "task_id": {
                                "type": "string",
                                "description": "Optional workflow task ID to attach artifacts and activity to"
                            }
                        },
                        "required": ["edit_session_id", "destinations", "formats", "priority"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "run_visual_qc",
                    "description": "Run Spectra Visual QC pass on a media batch. Extracts keyframes from clips, sends them to a Vision API (provider-agnostic), scores composition quality, and selects optimal in-points. Must be run AFTER analyze_media_batch and BEFORE generate_video_edits for vision-guided framing.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "batch_id": {
                                "type": "string",
                                "description": "UUID of the media batch to run Visual QC on (must be in Ready status)"
                            },
                            "candidates_per_clip": {
                                "type": "integer",
                                "description": "Number of candidate frames to extract per clip (default: 5)"
                            },
                            "min_composition_score": {
                                "type": "number",
                                "description": "Minimum composition score (0.0-1.0) to pass QC (default: 0.6)"
                            },
                            "target_aspect_ratio": {
                                "type": "string",
                                "description": "Target delivery aspect ratio for crop suggestions (e.g., '16:9', '9:16')"
                            },
                            "project_id": {
                                "type": "string",
                                "description": "Optional project ID to associate this QC pass with"
                            }
                        },
                        "required": ["batch_id"]
                    }
                }
            }),
            // === Music Discovery & Download Tools ===
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "search_music",
                    "description": "Search for music tracks across all configured platforms (Artlist, Epidemic Sound, Soundstripe). Returns tracks matching the given criteria with platform availability status. Use moods, genres, BPM ranges, and duration to find the perfect track for a video project.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Free-text search query (e.g., 'upbeat corporate', 'cinematic piano')"
                            },
                            "moods": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Mood filters (e.g., ['uplifting', 'energetic', 'cinematic'])"
                            },
                            "genres": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Genre filters (e.g., ['electronic', 'pop', 'orchestral'])"
                            },
                            "min_bpm": {
                                "type": "integer",
                                "description": "Minimum BPM (beats per minute)"
                            },
                            "max_bpm": {
                                "type": "integer",
                                "description": "Maximum BPM"
                            },
                            "min_duration": {
                                "type": "number",
                                "description": "Minimum track duration in seconds"
                            },
                            "max_duration": {
                                "type": "number",
                                "description": "Maximum track duration in seconds"
                            },
                            "instrumental": {
                                "type": "boolean",
                                "description": "Filter for instrumental-only tracks (no vocals)"
                            },
                            "platforms": {
                                "type": "array",
                                "items": { "type": "string", "enum": ["artlist", "epidemic", "soundstripe"] },
                                "description": "Limit search to specific platforms (default: all configured)"
                            },
                            "page": {
                                "type": "integer",
                                "description": "Page number (default: 1)"
                            },
                            "per_page": {
                                "type": "integer",
                                "description": "Results per page (default: 20)"
                            }
                        },
                        "required": []
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "download_music_track",
                    "description": "Download a music track to local storage by its platform-prefixed ID (e.g., 'artlist:12345', 'epidemic:67890', 'soundstripe:abc'). Returns the local file path.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "track_id": {
                                "type": "string",
                                "description": "Platform-prefixed track ID (e.g., 'artlist:12345', 'epidemic:67890', 'soundstripe:abc')"
                            },
                            "filename": {
                                "type": "string",
                                "description": "Optional output filename (default: 'Artist - Title.mp3')"
                            },
                            "output_dir": {
                                "type": "string",
                                "description": "Optional output directory path (default: editron work_dir/music/<platform>)"
                            }
                        },
                        "required": ["track_id"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "recommend_music_for_video",
                    "description": "Get music recommendations based on video content type or analysis. Returns search criteria and optionally auto-searches configured platforms.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "video_path": {
                                "type": "string",
                                "description": "Path to video file for content analysis"
                            },
                            "content_type": {
                                "type": "string",
                                "description": "Content category: 'lifestyle', 'corporate', 'cinematic', 'chill', 'fitness', 'fashion'"
                            },
                            "target_duration": {
                                "type": "number",
                                "description": "Target video duration in seconds (finds tracks at least this long)"
                            },
                            "auto_search": {
                                "type": "boolean",
                                "description": "If true, automatically search configured platforms with the recommended criteria (default: false)"
                            }
                        },
                        "required": []
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "preview_music_track",
                    "description": "Get preview/stream URL and basic info for a music track. For Epidemic Sound tracks, also returns ML-detected highlight timestamps.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "track_id": {
                                "type": "string",
                                "description": "Platform-prefixed track ID (e.g., 'artlist:12345', 'epidemic:67890')"
                            }
                        },
                        "required": ["track_id"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "get_music_track_details",
                    "description": "Get full metadata for a specific music track. For Epidemic Sound, enriches with beat timestamps and similar track recommendations.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "track_id": {
                                "type": "string",
                                "description": "Platform-prefixed track ID (e.g., 'artlist:12345', 'epidemic:67890', 'soundstripe:abc')"
                            }
                        },
                        "required": ["track_id"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "analyze_music_track",
                    "description": "Analyze a local audio file for BPM, energy profile, sections, and beat grid. Uses the same engine as AnalyzeBeatGrid but returns a music-focused summary.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "audio_path": {
                                "type": "string",
                                "description": "Path to the local audio file to analyze"
                            },
                            "bpm_hint": {
                                "type": "number",
                                "description": "Optional BPM hint if known (improves accuracy)"
                            }
                        },
                        "required": ["audio_path"]
                    }
                }
            }),
        ]
    }

    /// Generate tool schemas for user-scoped Orcha agents (subset of admin tools)
    pub fn get_user_tool_schemas() -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "create_project",
                    "description": "Create a new project. Use this when the user wants to start a new project.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "The name of the project to create"
                            }
                        },
                        "required": ["name"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "list_my_projects",
                    "description": "List all projects the user is a member of.",
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "create_task",
                    "description": "Create a new task in a project. The task will be added to the project's default board.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "project_name": {
                                "type": "string",
                                "description": "The name of the project to add the task to"
                            },
                            "title": {
                                "type": "string",
                                "description": "The title of the task"
                            },
                            "description": {
                                "type": "string",
                                "description": "Optional description of the task"
                            },
                            "priority": {
                                "type": "string",
                                "enum": ["low", "medium", "high", "critical"],
                                "description": "Priority level (default: medium)"
                            }
                        },
                        "required": ["project_name", "title"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "get_project_tasks",
                    "description": "Get all tasks for a project.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "project_name": {
                                "type": "string",
                                "description": "The name of the project"
                            },
                            "status_filter": {
                                "type": "string",
                                "enum": ["todo", "in_progress", "done", "blocked"],
                                "description": "Optional status filter"
                            }
                        },
                        "required": ["project_name"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "update_task_status",
                    "description": "Update the status of a task.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "task_id": {
                                "type": "string",
                                "description": "The UUID of the task to update"
                            },
                            "status": {
                                "type": "string",
                                "enum": ["todo", "in_progress", "done", "blocked"],
                                "description": "The new status"
                            }
                        },
                        "required": ["task_id", "status"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "search_web",
                    "description": "Search the web for information. Use this when the user asks you to look up or research something.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The search query"
                            }
                        },
                        "required": ["query"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "fetch_web_page",
                    "description": "Fetch and read the content of a web page.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "The URL to fetch"
                            }
                        },
                        "required": ["url"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "render_page",
                    "description": "Render a JavaScript-heavy web page using a real browser (Playwright/Chromium). Use this when fetch_web_page returns empty or incomplete content because the page requires JavaScript to render (e.g. SPAs, React/Vue/Angular apps, dashboards, dynamic content).",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "The URL to render with a headless browser"
                            },
                            "include_html": {
                                "type": "boolean",
                                "description": "Whether to include the full rendered HTML in the response (default: false — text only)"
                            }
                        },
                        "required": ["url"]
                    }
                }
            }),
        ]
    }

    /// Generate tool schemas for system-tier agents (e.g. Topsi)
    pub fn get_system_tool_schemas() -> Vec<serde_json::Value> {
        // System agents like Topsi get the same tools as user agents
        Self::get_user_tool_schemas()
    }
}
