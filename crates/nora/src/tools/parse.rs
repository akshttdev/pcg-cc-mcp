//! Tool call parsing from LLM function calls

use super::types::*;
use super::ExecutiveTools;

#[allow(dead_code)]
impl ExecutiveTools {
    /// Parse a tool call from the LLM and convert it to NoraExecutiveTool
    pub fn parse_tool_call(name: &str, arguments: &serde_json::Value) -> Option<NoraExecutiveTool> {
        match name {
            "create_project" => {
                let name = arguments.get("name")?.as_str()?.to_string();
                let git_repo_path = arguments
                    .get("git_repo_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(NoraExecutiveTool::CreateProject {
                    name,
                    git_repo_path,
                    setup_script: None,
                    dev_script: None,
                })
            }
            "create_board" => {
                let project_id = arguments.get("project_id")?.as_str()?.to_string();
                let name = arguments.get("name")?.as_str()?.to_string();
                let description = arguments
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let board_type = arguments
                    .get("board_type")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::CreateBoard {
                    project_id,
                    name,
                    description,
                    board_type,
                })
            }
            "create_task" => {
                // New simplified API - takes project name instead of IDs
                let project_name = arguments.get("project_name")?.as_str()?.to_string();
                let title = arguments.get("title")?.as_str()?.to_string();
                let description = arguments
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let priority = arguments
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::CreateTaskInProject {
                    project_name,
                    title,
                    description,
                    priority,
                })
            }
            "get_project_tasks" => {
                let project_name = arguments.get("project_name")?.as_str()?.to_string();
                let status_filter = arguments
                    .get("status_filter")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::GetProjectTasks {
                    project_name,
                    status_filter,
                })
            }
            "get_project_details" => {
                let project_name = arguments.get("project_name")?.as_str()?.to_string();
                Some(NoraExecutiveTool::GetProjectDetails {
                    project_name,
                })
            }
            "delete_project" => {
                let project_name = arguments.get("project_name")?.as_str()?.to_string();
                Some(NoraExecutiveTool::DeleteProject { project_name })
            }
            "update_project" => {
                let project_name = arguments.get("project_name")?.as_str()?.to_string();
                let new_name = arguments.get("new_name").and_then(|v| v.as_str()).map(String::from);
                let new_description = arguments.get("new_description").and_then(|v| v.as_str()).map(String::from);
                Some(NoraExecutiveTool::UpdateProject { project_name, new_name, new_description })
            }
            "execute_workflow" => {
                let agent_id = arguments.get("agent_id")?.as_str()?.to_string();
                let workflow_id = arguments.get("workflow_id")?.as_str()?.to_string();
                let project_id = arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let inputs = arguments
                    .get("inputs")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    })
                    .unwrap_or_default();
                Some(NoraExecutiveTool::ExecuteWorkflow {
                    agent_id,
                    workflow_id,
                    project_id,
                    inputs,
                })
            }
            "cancel_workflow" => {
                let workflow_instance_id = arguments.get("workflow_instance_id")?.as_str()?.to_string();
                Some(NoraExecutiveTool::CancelWorkflow {
                    workflow_instance_id,
                })
            }
            "list_active_workflows" => {
                Some(NoraExecutiveTool::ListActiveWorkflows)
            }
            "list_available_workflows" => {
                let agent_id = arguments.get("agent_id").and_then(|v| v.as_str()).map(String::from);
                Some(NoraExecutiveTool::ListAvailableWorkflows { agent_id })
            }
            "send_email" => {
                let recipients: Vec<String> = arguments
                    .get("to")?
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                let subject = arguments.get("subject")?.as_str()?.to_string();
                let body = arguments.get("body")?.as_str()?.to_string();
                Some(NoraExecutiveTool::SendEmail {
                    recipients,
                    subject,
                    body,
                    priority: EmailPriority::Normal,
                })
            }
            "read_inbox" => {
                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v.min(50) as usize)
                    .unwrap_or(10);
                let owner_type = arguments.get("owner_type").and_then(|v| v.as_str()).map(String::from);
                let owner_id = arguments.get("owner_id").and_then(|v| v.as_str()).map(String::from);
                Some(NoraExecutiveTool::ReadInbox { limit, owner_type, owner_id })
            }
            "send_sms" => {
                let to = arguments.get("to")?.as_str()?.to_string();
                let message = arguments.get("message")?.as_str()?.to_string();
                Some(NoraExecutiveTool::SendSms { to, message })
            }
            "send_discord_message" => {
                let message = arguments.get("message")?.as_str()?.to_string();
                let mention_users = arguments
                    .get("mentions")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                Some(NoraExecutiveTool::SendDiscordMessage {
                    channel: String::new(), // Use default channel
                    message,
                    mention_users,
                })
            }
            "ingest_media_batch" => {
                let source_url = arguments.get("source_url")?.as_str()?.to_string();
                let reference_name = arguments
                    .get("reference_name")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let storage_tier = arguments.get("storage_tier")?.as_str()?.to_string();
                let checksum_required = arguments
                    .get("checksum_required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let project_id = arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let task_id = arguments
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::IngestMediaBatch {
                    source_url,
                    reference_name,
                    storage_tier,
                    checksum_required,
                    project_id,
                    task_id,
                })
            }
            "analyze_media_batch" => {
                let batch_id = arguments.get("batch_id")?.as_str()?.to_string();
                let brief = arguments.get("brief")?.as_str()?.to_string();
                let passes = arguments
                    .get("passes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(2) as u32;
                let deliverable_targets = arguments
                    .get("deliverable_targets")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_else(Vec::new);
                let project_id = arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let task_id = arguments
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::AnalyzeMediaBatch {
                    batch_id,
                    brief,
                    passes,
                    deliverable_targets,
                    project_id,
                    task_id,
                })
            }
            "generate_video_edits" => {
                let batch_id = arguments.get("batch_id")?.as_str()?.to_string();
                let deliverable_type = arguments.get("deliverable_type")?.as_str()?.to_string();
                let aspect_ratios = arguments
                    .get("aspect_ratios")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_else(|| vec!["16:9".to_string()]);
                let reference_style = arguments
                    .get("reference_style")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let include_captions = arguments
                    .get("include_captions")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let project_id = arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let task_id = arguments
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::GenerateVideoEdits {
                    batch_id,
                    deliverable_type,
                    aspect_ratios,
                    reference_style,
                    include_captions,
                    project_id,
                    task_id,
                })
            }
            "render_video_deliverables" => {
                let edit_session_id = arguments.get("edit_session_id")?.as_str()?.to_string();
                let destinations = arguments
                    .get("destinations")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let formats = arguments
                    .get("formats")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let priority = arguments
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .map(|value| match value.to_lowercase().as_str() {
                        "low" => VideoRenderPriority::Low,
                        "rush" => VideoRenderPriority::Rush,
                        _ => VideoRenderPriority::Standard,
                    })
                    .unwrap_or(VideoRenderPriority::Standard);
                let project_id = arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let task_id = arguments
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::RenderVideoDeliverables {
                    edit_session_id,
                    destinations,
                    formats,
                    priority,
                    project_id,
                    task_id,
                })
            }
            "run_visual_qc" => {
                let batch_id = arguments.get("batch_id")?.as_str()?.to_string();
                let candidates_per_clip = arguments
                    .get("candidates_per_clip")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                let min_composition_score = arguments
                    .get("min_composition_score")
                    .and_then(|v| v.as_f64());
                let target_aspect_ratio = arguments
                    .get("target_aspect_ratio")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let project_id = arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::RunVisualQc {
                    batch_id,
                    candidates_per_clip,
                    min_composition_score,
                    target_aspect_ratio,
                    project_id,
                })
            }
            "analyze_scenes" => {
                let batch_id = arguments.get("batch_id")?.as_str()?.to_string();
                let segment_interval = arguments.get("segment_interval").and_then(|v| v.as_f64());
                let project_id = arguments.get("project_id").and_then(|v| v.as_str()).map(String::from);
                Some(NoraExecutiveTool::AnalyzeScenes { batch_id, segment_interval, project_id })
            }
            "analyze_beat_grid" => {
                let audio_path = arguments.get("audio_path")?.as_str()?.to_string();
                let bpm_hint = arguments.get("bpm_hint").and_then(|v| v.as_f64());
                let beats_per_bar = arguments.get("beats_per_bar").and_then(|v| v.as_u64()).map(|v| v as u32);
                let project_id = arguments.get("project_id").and_then(|v| v.as_str()).map(String::from);
                Some(NoraExecutiveTool::AnalyzeBeatGrid { audio_path, bpm_hint, beats_per_bar, project_id })
            }
            "assemble_recap_edit" => {
                let batch_id = arguments.get("batch_id")?.as_str()?.to_string();
                let audio_path = arguments.get("audio_path")?.as_str()?.to_string();
                let bpm_hint = arguments.get("bpm_hint").and_then(|v| v.as_f64());
                let target_aspect_ratio = arguments.get("target_aspect_ratio").and_then(|v| v.as_str()).map(String::from);
                let project_id = arguments.get("project_id").and_then(|v| v.as_str()).map(String::from);
                let project_name = arguments.get("project_name").and_then(|v| v.as_str()).map(String::from);
                Some(NoraExecutiveTool::AssembleRecapEdit { batch_id, audio_path, bpm_hint, target_aspect_ratio, project_id, project_name })
            }
            "execute_render_script" => {
                let render_script = arguments.get("render_script")?.as_str()?.to_string();
                let render_output = arguments.get("render_output")
                    .and_then(|v| v.as_str())
                    .unwrap_or("output.mp4")
                    .to_string();
                let xml_path = arguments.get("xml_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(NoraExecutiveTool::ExecuteRenderScript { render_script, render_output, xml_path })
            }
            "search_music" => {
                let query = arguments.get("query").and_then(|v| v.as_str()).map(String::from);
                let moods = arguments.get("moods").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
                });
                let genres = arguments.get("genres").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
                });
                let min_bpm = arguments.get("min_bpm").and_then(|v| v.as_u64()).map(|v| v as u32);
                let max_bpm = arguments.get("max_bpm").and_then(|v| v.as_u64()).map(|v| v as u32);
                let min_duration = arguments.get("min_duration").and_then(|v| v.as_f64());
                let max_duration = arguments.get("max_duration").and_then(|v| v.as_f64());
                let instrumental = arguments.get("instrumental").and_then(|v| v.as_bool());
                let platforms = arguments.get("platforms").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
                });
                let page = arguments.get("page").and_then(|v| v.as_u64()).map(|v| v as u32);
                let per_page = arguments.get("per_page").and_then(|v| v.as_u64()).map(|v| v as u32);
                Some(NoraExecutiveTool::SearchMusic {
                    query, moods, genres, min_bpm, max_bpm, min_duration, max_duration,
                    instrumental, platforms, page, per_page,
                })
            }
            "download_music_track" => {
                let track_id = arguments.get("track_id")?.as_str()?.to_string();
                let filename = arguments.get("filename").and_then(|v| v.as_str()).map(String::from);
                let output_dir = arguments.get("output_dir").and_then(|v| v.as_str()).map(String::from);
                Some(NoraExecutiveTool::DownloadMusicTrack { track_id, filename, output_dir })
            }
            "recommend_music_for_video" => {
                let video_path = arguments.get("video_path").and_then(|v| v.as_str()).map(String::from);
                let content_type = arguments.get("content_type").and_then(|v| v.as_str()).map(String::from);
                let target_duration = arguments.get("target_duration").and_then(|v| v.as_f64());
                let auto_search = arguments.get("auto_search").and_then(|v| v.as_bool());
                Some(NoraExecutiveTool::RecommendMusicForVideo { video_path, content_type, target_duration, auto_search })
            }
            "preview_music_track" => {
                let track_id = arguments.get("track_id")?.as_str()?.to_string();
                Some(NoraExecutiveTool::PreviewMusicTrack { track_id })
            }
            "get_music_track_details" => {
                let track_id = arguments.get("track_id")?.as_str()?.to_string();
                Some(NoraExecutiveTool::GetMusicTrackDetails { track_id })
            }
            "analyze_music_track" => {
                let audio_path = arguments.get("audio_path")?.as_str()?.to_string();
                let bpm_hint = arguments.get("bpm_hint").and_then(|v| v.as_f64());
                Some(NoraExecutiveTool::AnalyzeMusicTrack { audio_path, bpm_hint })
            }
            "create_calendar_event" => {
                let title = arguments.get("title")?.as_str()?.to_string();
                let start_time_str = arguments.get("start_time")?.as_str()?;
                let end_time_str = arguments.get("end_time")?.as_str()?;

                // Parse ISO 8601 datetime strings
                let start_time = chrono::DateTime::parse_from_rfc3339(start_time_str)
                    .ok()?
                    .with_timezone(&chrono::Utc);
                let end_time = chrono::DateTime::parse_from_rfc3339(end_time_str)
                    .ok()?
                    .with_timezone(&chrono::Utc);

                let attendees = arguments
                    .get("attendees")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let location = arguments
                    .get("location")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                Some(NoraExecutiveTool::CreateCalendarEvent {
                    title,
                    start_time,
                    end_time,
                    attendees,
                    location,
                })
            }
            "render_page" => {
                let url = arguments.get("url")?.as_str()?.to_string();
                let include_html = arguments
                    .get("include_html")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Some(NoraExecutiveTool::RenderPage { url, include_html })
            }
            "search_web" => {
                let query = arguments.get("query")?.as_str()?.to_string();
                let max_results = arguments
                    .get("max_results")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5) as u32;
                Some(NoraExecutiveTool::SearchWeb {
                    query,
                    max_results,
                    search_type: SearchType::General,
                })
            }
            "fetch_web_page" => {
                let url = arguments.get("url")?.as_str()?.to_string();
                let extract_text = arguments
                    .get("extract_text")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                Some(NoraExecutiveTool::FetchWebPage { url, extract_text })
            }
            _ => None,
        }
    }
}
