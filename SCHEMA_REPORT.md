# PCG-CC-MCP SQLite Schema Report
# Database: /home/pythia/pcg-cc-mcp/dev_assets/db.sqlite
# Generated: Thu Mar 26 01:00:12 AM UTC 2026
# Total tables: 253

================================================================================
TABLE: _sqlx_migrations
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|version|BIGINT|0||1
1|description|TEXT|1||0
2|installed_on|TIMESTAMP|1|CURRENT_TIMESTAMP|0
3|success|BOOLEAN|1||0
4|checksum|BLOB|1||0
5|execution_time|BIGINT|1||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
221

================================================================================
TABLE: activity_logs
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|task_id|TEXT|1||0
2|actor_id|TEXT|1||0
3|actor_type|TEXT|1||0
4|action|TEXT|1||0
5|previous_state|TEXT|0||0
6|new_state|TEXT|0||0
7|metadata|TEXT|0||0
8|timestamp|TEXT|1|CURRENT_TIMESTAMP|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|tasks|task_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
93

================================================================================
TABLE: agent_capabilities
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|agent_id|TEXT|1||1
1|capability_id|TEXT|1||2
2|proficiency_level|TEXT|0|'standard'|0
3|enabled|BOOLEAN|0|TRUE|0
4|custom_config|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agent_capability_definitions|capability_id|id|NO ACTION|CASCADE|NONE
1|0|agents|agent_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: agent_capability_definitions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|name|TEXT|1||0
2|category|TEXT|1||0
3|description|TEXT|0||0
4|required_tools|TEXT|0||0
5|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
12

================================================================================
TABLE: agent_conversation_messages
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|conversation_id|BLOB|1||0
2|role|TEXT|1||0
3|content|TEXT|1||0
4|tool_call_id|TEXT|0||0
5|tool_name|TEXT|0||0
6|tool_arguments|TEXT|0||0
7|tool_result|TEXT|0||0
8|input_tokens|INTEGER|0||0
9|output_tokens|INTEGER|0||0
10|model|TEXT|0||0
11|provider|TEXT|0||0
12|latency_ms|INTEGER|0||0
13|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agent_conversations|conversation_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
207

================================================================================
TABLE: agent_conversations
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|agent_id|BLOB|1||0
2|session_id|TEXT|1||0
3|project_id|BLOB|0||0
4|user_id|TEXT|0||0
5|status|TEXT|1|'active'|0
6|title|TEXT|0||0
7|context_snapshot|TEXT|0||0
8|created_at|TEXT|1|datetime('now', 'subsec')|0
9|updated_at|TEXT|1|datetime('now', 'subsec')|0
10|last_message_at|TEXT|0||0
11|message_count|INTEGER|1|0|0
12|total_input_tokens|INTEGER|0|0|0
13|total_output_tokens|INTEGER|0|0|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|SET NULL|NONE
1|0|agents|agent_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
34

================================================================================
TABLE: agent_execution_config
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|agent_id|TEXT|1||0
2|execution_profile_id|TEXT|0||0
3|execution_mode_override|TEXT|0||0
4|max_iterations_override|INTEGER|0||0
5|backpressure_commands_override|TEXT|0||0
6|system_prompt_prefix|TEXT|0||0
7|system_prompt_suffix|TEXT|0||0
8|project_type_backpressure|TEXT|0|'{}'|0
9|auto_commit_on_success|BOOLEAN|0|TRUE|0
10|auto_create_pr_on_complete|BOOLEAN|0|FALSE|0
11|require_tests_pass|BOOLEAN|0|TRUE|0
12|is_active|BOOLEAN|0|TRUE|0
13|created_at|TEXT|1|datetime('now', 'subsec')|0
14|updated_at|TEXT|1|datetime('now', 'subsec')|0
15|auto_watch_agent_ids|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agent_execution_profiles|execution_profile_id|id|NO ACTION|SET NULL|NONE
1|0|agents|agent_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
2

================================================================================
TABLE: agent_execution_profiles
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|name|TEXT|1||0
2|description|TEXT|0||0
3|execution_mode|TEXT|1|'standard'|0
4|max_iterations|INTEGER|0|50|0
5|completion_promise|TEXT|0|'<promise>TASK_COMPLETE</promise>'|0
6|exit_signal_key|TEXT|0|'EXIT_SIGNAL: true'|0
7|backpressure_commands|TEXT|0|'[]'|0
8|backpressure_fail_threshold|INTEGER|0|0|0
9|iteration_delay_ms|INTEGER|0|2000|0
10|iteration_timeout_ms|INTEGER|0|600000|0
11|total_timeout_ms|INTEGER|0|3600000|0
12|preserve_session|BOOLEAN|0|TRUE|0
13|context_window_strategy|TEXT|0|'fresh'|0
14|prompt_templates|TEXT|0|'{}'|0
15|created_at|TEXT|1|datetime('now', 'subsec')|0
16|updated_at|TEXT|1|datetime('now', 'subsec')|0
17|created_by|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
4

================================================================================
TABLE: agent_flow_events
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|agent_flow_id|BLOB|1||0
2|event_type|TEXT|1||0
3|event_data|TEXT|1||0
4|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agent_flows|agent_flow_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
783

================================================================================
TABLE: agent_flows
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|task_id|BLOB|1||0
2|flow_type|TEXT|1||0
3|status|TEXT|1|'planning'|0
4|planner_agent_id|BLOB|0||0
5|executor_agent_id|BLOB|0||0
6|verifier_agent_id|BLOB|0||0
7|current_phase|TEXT|1|'planning'|0
8|planning_started_at|TEXT|0||0
9|planning_completed_at|TEXT|0||0
10|execution_started_at|TEXT|0||0
11|execution_completed_at|TEXT|0||0
12|verification_started_at|TEXT|0||0
13|verification_completed_at|TEXT|0||0
14|flow_config|TEXT|0||0
15|handoff_instructions|TEXT|0||0
16|verification_score|REAL|0||0
17|human_approval_required|BOOLEAN|1|0|0
18|approved_by|TEXT|0||0
19|approved_at|TEXT|0||0
20|created_at|TEXT|1|datetime('now', 'subsec')|0
21|updated_at|TEXT|1|datetime('now', 'subsec')|0
22|clarification_request|TEXT|0||0
23|crm_deal_id|TEXT|0||0
24|cancel_deadline|TEXT|0||0
25|retry_count|INTEGER|1|0|0
26|last_error|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
85

================================================================================
TABLE: agent_interactions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|agent_id|TEXT|1||0
2|interaction_type|TEXT|1||0
3|task_id|TEXT|0||0
4|input_summary|TEXT|0||0
5|output_summary|TEXT|0||0
6|success|BOOLEAN|0||0
7|execution_time_ms|INTEGER|0||0
8|human_feedback|TEXT|0||0
9|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agents|agent_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: agent_sessions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|task_attempt_id|BLOB|1||0
2|executor|TEXT|0||0
3|agent_working_dir|TEXT|0||0
4|created_at|TEXT|1|datetime('now', 'subsec')|0
5|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|task_attempts|task_attempt_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
33

================================================================================
TABLE: agent_task_plans
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|execution_process_id|BLOB|1||0
2|plan_json|TEXT|1||0
3|current_step|INTEGER|0|0|0
4|total_steps|INTEGER|0||0
5|status|TEXT|1|'planning'|0
6|created_at|TEXT|1|datetime('now', 'subsec')|0
7|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|execution_processes|execution_process_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: agent_wallet_transactions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|wallet_id|BLOB|1||0
2|direction|TEXT|1||0
3|amount|INTEGER|1||0
4|description|TEXT|1|''|0
5|metadata|TEXT|0||0
6|task_id|BLOB|0||0
7|process_id|BLOB|0||0
8|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agent_wallets|wallet_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: agent_wallets
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|profile_key|TEXT|1||0
2|display_name|TEXT|1|''|0
3|budget_limit|INTEGER|1|0|0
4|spent_amount|INTEGER|1|0|0
5|created_at|TEXT|1|datetime('now', 'subsec')|0
6|updated_at|TEXT|1|datetime('now', 'subsec')|0
7|vibe_budget_limit|INTEGER|0||0
8|vibe_spent_amount|INTEGER|1|0|0
9|aptos_address|TEXT|0||0
10|aptos_private_key_encrypted|TEXT|0||0
11|aptos_funded|INTEGER|1|0|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: agents
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|wallet_address|TEXT|0||0
2|short_name|TEXT|1||0
3|designation|TEXT|1||0
4|description|TEXT|0||0
5|personality|TEXT|0||0
6|voice_style|TEXT|0||0
7|avatar_url|TEXT|0||0
8|capabilities|TEXT|0||0
9|tools|TEXT|0||0
10|functions|TEXT|0||0
11|default_model|TEXT|0||0
12|fallback_models|TEXT|0||0
13|model_config|TEXT|0||0
14|status|TEXT|1|'active'|0
15|autonomy_level|TEXT|1|'supervised'|0
16|max_concurrent_tasks|INTEGER|0|5|0
17|priority_weight|INTEGER|0|100|0
18|tasks_completed|INTEGER|0|0|0
19|tasks_failed|INTEGER|0|0|0
20|total_execution_time_ms|INTEGER|0|0|0
21|average_rating|REAL|0||0
22|version|TEXT|0|'1.0.0'|0
23|created_at|TEXT|1|datetime('now', 'subsec')|0
24|updated_at|TEXT|1|datetime('now', 'subsec')|0
25|created_by|TEXT|0||0
26|parent_agent_id|TEXT|0||0
27|team_id|TEXT|0||0
28|owner_id|BLOB|0||0
29|agent_tier|TEXT|1|'admin'|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agents|parent_agent_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
39

================================================================================
TABLE: airtable_bases
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|airtable_base_id|TEXT|1||0
3|airtable_base_name|TEXT|0||0
4|sync_enabled|INTEGER|1|1|0
5|default_table_id|TEXT|0||0
6|last_synced_at|TEXT|0||0
7|created_at|TEXT|1|datetime('now', 'subsec')|0
8|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: airtable_record_links
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|task_id|TEXT|1||0
2|airtable_record_id|TEXT|1||0
3|airtable_base_id|TEXT|1||0
4|airtable_table_id|TEXT|0||0
5|origin|TEXT|1|'airtable'|0
6|sync_status|TEXT|1|'synced'|0
7|last_sync_error|TEXT|0||0
8|airtable_record_url|TEXT|0||0
9|last_synced_at|TEXT|0||0
10|created_at|TEXT|1|datetime('now', 'subsec')|0
11|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|tasks|task_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: apn_cloud_capacity
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|INTEGER|0||1
1|measured_at|DATETIME|0|CURRENT_TIMESTAMP|0
2|total_cores|REAL|1||0
3|available_cores|REAL|1||0
4|total_ram_gb|REAL|1||0
5|available_ram_gb|REAL|1||0
6|gpus_available|INTEGER|1||0
7|master_nodes_online|INTEGER|1||0
8|relay_devices_online|INTEGER|1||0
9|tasks_queued|INTEGER|0|0|0
10|tasks_executing|INTEGER|0|0|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
1

================================================================================
TABLE: approval_gates
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|0||0
2|task_id|BLOB|0||0
3|name|TEXT|1||0
4|gate_type|TEXT|1||0
5|required_approvers|TEXT|1|'[]'|0
6|min_approvals|INTEGER|1|1|0
7|conditions|TEXT|0|'{}'|0
8|is_active|INTEGER|1|1|0
9|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|tasks|task_id|id|NO ACTION|CASCADE|NONE
1|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: artifact_reviews
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|artifact_id|BLOB|1||0
2|reviewer_id|TEXT|0||0
3|reviewer_agent_id|BLOB|0||0
4|reviewer_name|TEXT|0||0
5|review_type|TEXT|1||0
6|status|TEXT|1||0
7|feedback_text|TEXT|0||0
8|rating|INTEGER|0||0
9|revision_notes|TEXT|0||0
10|revision_deadline|TEXT|0||0
11|created_at|TEXT|1|datetime('now', 'subsec')|0
12|resolved_at|TEXT|0||0
13|resolved_by|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agents|reviewer_agent_id|id|NO ACTION|SET NULL|NONE
1|0|execution_artifacts|artifact_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: attempt_repos
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|task_attempt_id|BLOB|1||0
2|repo_id|BLOB|1||0
3|target_branch|TEXT|1||0
4|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|repos|repo_id|id|NO ACTION|CASCADE|NONE
1|0|task_attempts|task_attempt_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: backpressure_definitions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|name|TEXT|1||0
2|description|TEXT|0||0
3|project_type|TEXT|0||0
4|commands|TEXT|1||0
5|fail_on_any|BOOLEAN|0|TRUE|0
6|timeout_ms|INTEGER|0|300000|0
7|run_in_parallel|BOOLEAN|0|FALSE|0
8|priority|INTEGER|0|0|0
9|is_active|BOOLEAN|0|TRUE|0
10|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
7

================================================================================
TABLE: board_shares
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|board_id|TEXT|1||0
2|source_organization_id|TEXT|1||0
3|target_organization_id|TEXT|1||0
4|permission|TEXT|1|'editor'|0
5|share_type|TEXT|1|'collaboration'|0
6|shared_by|TEXT|1||0
7|is_active|INTEGER|1|1|0
8|created_at|TEXT|1|datetime('now')|0
9|updated_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
10

================================================================================
TABLE: brand_intake_tokens
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|organization_id|BLOB|1||0
2|token|TEXT|1||0
3|expires_at|TEXT|1||0
4|used_at|TEXT|0||0
5|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|organizations|organization_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: browser_actions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|browser_session_id|BLOB|1||0
2|action_type|TEXT|1||0
3|target_selector|TEXT|0||0
4|action_data|TEXT|0||0
5|result|TEXT|0||0
6|error_message|TEXT|0||0
7|duration_ms|INTEGER|0||0
8|screenshot_id|BLOB|0||0
9|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|browser_screenshots|screenshot_id|id|NO ACTION|NO ACTION|NONE
1|0|browser_sessions|browser_session_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: browser_allowlist
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|0||0
2|pattern|TEXT|1||0
3|pattern_type|TEXT|1|'glob'|0
4|description|TEXT|0||0
5|is_global|INTEGER|1|0|0
6|created_by|TEXT|0||0
7|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
4

================================================================================
TABLE: browser_screenshots
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|browser_session_id|BLOB|1||0
2|url|TEXT|1||0
3|page_title|TEXT|0||0
4|screenshot_path|TEXT|1||0
5|thumbnail_path|TEXT|0||0
6|baseline_screenshot_id|BLOB|0||0
7|diff_path|TEXT|0||0
8|diff_percentage|REAL|0||0
9|viewport_width|INTEGER|1||0
10|viewport_height|INTEGER|1||0
11|full_page|INTEGER|1|0|0
12|metadata|TEXT|0||0
13|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|browser_screenshots|baseline_screenshot_id|id|NO ACTION|NO ACTION|NONE
1|0|browser_sessions|browser_session_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: browser_sessions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|execution_process_id|BLOB|1||0
2|browser_type|TEXT|1|'chromium'|0
3|viewport_width|INTEGER|1|1280|0
4|viewport_height|INTEGER|1|720|0
5|headless|INTEGER|1|1|0
6|status|TEXT|1|'starting'|0
7|current_url|TEXT|0||0
8|error_message|TEXT|0||0
9|started_at|TEXT|1|datetime('now', 'subsec')|0
10|closed_at|TEXT|0||0
11|agent_flow_id|BLOB|0||0
12|session_type|TEXT|0||0
13|platform|TEXT|0||0
14|target_url|TEXT|0||0
15|recording_path|TEXT|0||0
16|recording_duration_ms|INTEGER|0||0
17|screenshots_count|INTEGER|0|0|0
18|success|INTEGER|0|0|0
19|action_log|TEXT|0||0
20|completed_at|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agent_flows|agent_flow_id|id|NO ACTION|SET NULL|NONE
1|0|execution_processes|execution_process_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: business_reports
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|person_id|BLOB|0||0
2|company_id|BLOB|0||0
3|report_type|TEXT|1|'business_audit'|0
4|title|TEXT|1||0
5|status|TEXT|1|'draft'|0
6|executive_summary|TEXT|0||0
7|company_overview|TEXT|0||0
8|pain_points|TEXT|1|'[]'|0
9|opportunities|TEXT|1|'[]'|0
10|recommended_services|TEXT|1|'[]'|0
11|next_steps|TEXT|1|'[]'|0
12|full_report_md|TEXT|0||0
13|intake_item_ids|TEXT|1|'[]'|0
14|call_log_ids|TEXT|1|'[]'|0
15|created_by|BLOB|0||0
16|created_at|TEXT|1|datetime('now','subsec')|0
17|updated_at|TEXT|1|datetime('now','subsec')|0
18|individual_profiles|TEXT|1|'[]'|0
19|market_analysis|TEXT|0||0
20|competitor_analysis|TEXT|1|'[]'|0
21|target_clients|TEXT|0||0
22|brand_positioning|TEXT|0||0
23|digital_presence|TEXT|0||0
24|sources|TEXT|1|'[]'|0
25|crm_deal_id|BLOB|0||0
26|review_status|TEXT|1|'pending_review'|0
27|reviewed_by|BLOB|0||0
28|reviewed_at|TEXT|0||0
29|review_notes|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|reviewed_by|id|NO ACTION|SET NULL|NONE
1|0|crm_deals|crm_deal_id|id|NO ACTION|SET NULL|NONE
2|0|users|created_by|id|NO ACTION|NO ACTION|NONE
3|0|companies|company_id|id|NO ACTION|SET NULL|NONE
4|0|persons|person_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
43

================================================================================
TABLE: call_intake_items
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|source_type|TEXT|1|'email'|0
2|source_ref|TEXT|0||0
3|raw_content|TEXT|0||0
4|subject|TEXT|0||0
5|from_email|TEXT|0||0
6|from_name|TEXT|0||0
7|call_date|TEXT|0||0
8|duration_seconds|INTEGER|0||0
9|status|TEXT|1|'pending'|0
10|processed_at|TEXT|0||0
11|error|TEXT|0||0
12|person_id|BLOB|0||0
13|company_id|BLOB|0||0
14|extracted_participants|TEXT|1|'[]'|0
15|extracted_topics|TEXT|1|'[]'|0
16|extracted_action_items|TEXT|1|'[]'|0
17|extracted_pain_points|TEXT|1|'[]'|0
18|extracted_sentiment|TEXT|0||0
19|call_summary|TEXT|0||0
20|report_id|BLOB|0||0
21|metadata|TEXT|1|'{}'|0
22|created_at|TEXT|1|datetime('now','subsec')|0
23|updated_at|TEXT|1|datetime('now','subsec')|0
24|extracted_individuals|TEXT|1|'[]'|0
25|extracted_businesses|TEXT|1|'[]'|0
26|crm_deal_id|BLOB|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|crm_deals|crm_deal_id|id|NO ACTION|SET NULL|NONE
1|0|business_reports|report_id|id|NO ACTION|SET NULL|NONE
2|0|companies|company_id|id|NO ACTION|SET NULL|NONE
3|0|persons|person_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
58

================================================================================
TABLE: call_logs
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|call_sid|TEXT|1||0
3|parent_call_sid|TEXT|0||0
4|account_sid|TEXT|0||0
5|from_number|TEXT|1||0
6|to_number|TEXT|1||0
7|from_formatted|TEXT|0||0
8|to_formatted|TEXT|0||0
9|caller_name|TEXT|0||0
10|direction|TEXT|1||0
11|status|TEXT|1||0
12|answered_by|TEXT|0||0
13|start_time|TEXT|0||0
14|end_time|TEXT|0||0
15|duration_seconds|INTEGER|0|0|0
16|recording_url|TEXT|0||0
17|recording_sid|TEXT|0||0
18|recording_duration|INTEGER|0||0
19|transcription|TEXT|0||0
20|transcription_status|TEXT|0||0
21|handled_by_agent_id|BLOB|0||0
22|conversation_id|BLOB|0||0
23|summary|TEXT|0||0
24|sentiment|TEXT|0||0
25|crm_contact_id|BLOB|0||0
26|crm_deal_id|BLOB|0||0
27|price|REAL|0||0
28|price_unit|TEXT|0|'USD'|0
29|metadata|TEXT|0||0
30|created_at|TEXT|1|datetime('now', 'subsec')|0
31|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|crm_deals|crm_deal_id|id|NO ACTION|SET NULL|NONE
1|0|crm_contacts|crm_contact_id|id|NO ACTION|SET NULL|NONE
2|0|agent_conversations|conversation_id|id|NO ACTION|SET NULL|NONE
3|0|agents|handled_by_agent_id|id|NO ACTION|SET NULL|NONE
4|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
10

================================================================================
TABLE: category_queues
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|social_account_id|BLOB|0||0
3|name|TEXT|1||0
4|category|TEXT|1||0
5|description|TEXT|0||0
6|color|TEXT|0||0
7|schedule_rule|TEXT|0||0
8|posts_per_slot|INTEGER|0|1|0
9|prioritize_new|INTEGER|0|1|0
10|rotation_enabled|INTEGER|0|0|0
11|shuffle_on_recycle|INTEGER|0|0|0
12|min_days_between_repeats|INTEGER|0|30|0
13|total_posts|INTEGER|0|0|0
14|posts_published|INTEGER|0|0|0
15|created_at|TEXT|1|datetime('now', 'subsec')|0
16|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|social_accounts|social_account_id|id|NO ACTION|CASCADE|NONE
1|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: checkpoint_definitions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|0||0
2|name|TEXT|1||0
3|description|TEXT|0||0
4|checkpoint_type|TEXT|1||0
5|config|TEXT|1|'{}'|0
6|requires_approval|INTEGER|1|1|0
7|auto_approve_after_minutes|INTEGER|0||0
8|priority|INTEGER|1|0|0
9|is_active|INTEGER|1|1|0
10|created_at|TEXT|1|datetime('now', 'subsec')|0
11|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
4

================================================================================
TABLE: cinematic_briefs
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|requester_id|TEXT|1||0
3|nora_session_id|TEXT|0||0
4|title|TEXT|1||0
5|summary|TEXT|0|''|0
6|script|TEXT|0|''|0
7|asset_ids|TEXT|1|'[]'|0
8|duration_seconds|INTEGER|0|30|0
9|fps|INTEGER|0|24|0
10|style_tags|TEXT|1|'[]'|0
11|status|TEXT|1|'pending'|0
12|llm_notes|TEXT|1|''|0
13|render_payload|TEXT|1|'{}'|0
14|output_assets|TEXT|1|'[]'|0
15|metadata|TEXT|1|'{}'|0
16|created_at|TEXT|1|datetime('now', 'subsec')|0
17|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: cinematic_shot_plans
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|brief_id|BLOB|1||0
2|shot_index|INTEGER|1||0
3|title|TEXT|1|''|0
4|prompt|TEXT|1|''|0
5|negative_prompt|TEXT|1|''|0
6|camera_notes|TEXT|1|''|0
7|duration_seconds|INTEGER|0|4|0
8|metadata|TEXT|1|'{}'|0
9|status|TEXT|1|'pending'|0
10|created_at|TEXT|1|datetime('now', 'subsec')|0
11|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|cinematic_briefs|brief_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: client_members
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|client_id|TEXT|1||0
2|user_id|BLOB|1||0
3|role|TEXT|1|'viewer'|0
4|granted_by|BLOB|0||0
5|granted_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|granted_by|id|NO ACTION|SET NULL|NONE
1|0|users|user_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: clients
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|organization_id|TEXT|1||0
2|name|TEXT|1||0
3|slug|TEXT|1||0
4|description|TEXT|0||0
5|logo_url|TEXT|0||0
6|website|TEXT|0||0
7|crm_contact_id|TEXT|0||0
8|is_active|INTEGER|1|1|0
9|created_at|TEXT|1|datetime('now')|0
10|updated_at|TEXT|1|datetime('now')|0
11|deleted_at|TEXT|0||0
12|deleted_by|TEXT|0||0
13|crm_enabled|INTEGER|1|0|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
96

================================================================================
TABLE: cloud_contributions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|organization_id|TEXT|1||0
2|cloud_file_id|TEXT|1||0
3|user_id|TEXT|0||0
4|wallet_address|TEXT|0||0
5|device_id|TEXT|0||0
6|contribution_type|TEXT|1|'upload'|0
7|channel|TEXT|1|'http'|0
8|project_id|TEXT|0||0
9|task_id|TEXT|0||0
10|description|TEXT|0||0
11|metadata|TEXT|0|'{}'|0
12|created_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|cloud_files|cloud_file_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: cloud_files
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|organization_id|TEXT|1||0
2|project_id|TEXT|0||0
3|task_id|TEXT|0||0
4|file_name|TEXT|1||0
5|file_path|TEXT|1||0
6|storage_volume|TEXT|1||0
7|content_hash|TEXT|0||0
8|file_size_bytes|INTEGER|1|0|0
9|mime_type|TEXT|0||0
10|source_type|TEXT|1|'upload'|0
11|source_id|TEXT|0||0
12|source_table|TEXT|0||0
13|visibility|TEXT|1|'org'|0
14|contributed_by|TEXT|0||0
15|contributor_wallet|TEXT|0||0
16|contributor_device|TEXT|0||0
17|created_at|TEXT|1|datetime('now')|0
18|updated_at|TEXT|1|datetime('now')|0
19|deleted_at|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: cms_faq_items
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|site_id|BLOB|1||0
2|category|TEXT|0||0
3|question|TEXT|1||0
4|answer|TEXT|1||0
5|sort_order|INTEGER|0|0|0
6|is_active|INTEGER|1|1|0
7|created_at|TEXT|1|datetime('now', 'subsec')|0
8|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|cms_sites|site_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
4

================================================================================
TABLE: cms_page_sections
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|site_id|BLOB|1||0
2|page_slug|TEXT|1||0
3|section_key|TEXT|1||0
4|content|TEXT|1|'{}'|0
5|sort_order|INTEGER|0|0|0
6|is_active|INTEGER|1|1|0
7|created_at|TEXT|1|datetime('now', 'subsec')|0
8|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|cms_sites|site_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
5

================================================================================
TABLE: cms_products
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|site_id|BLOB|1||0
2|slug|TEXT|1||0
3|name|TEXT|1||0
4|short_description|TEXT|0||0
5|long_description|TEXT|0||0
6|price_cents|INTEGER|1||0
7|currency|TEXT|1|'USD'|0
8|stripe_price_id|TEXT|0||0
9|image_url|TEXT|0||0
10|gallery_images|TEXT|0|'[]'|0
11|specs|TEXT|0|'{}'|0
12|features|TEXT|0|'[]'|0
13|is_active|INTEGER|1|1|0
14|is_featured|INTEGER|1|0|0
15|stock_status|TEXT|0|'in_stock'|0
16|sort_order|INTEGER|0|0|0
17|created_at|TEXT|1|datetime('now', 'subsec')|0
18|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|cms_sites|site_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
1

================================================================================
TABLE: cms_site_settings
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|site_id|BLOB|1||0
2|setting_key|TEXT|1||0
3|setting_value|TEXT|1||0
4|created_at|TEXT|1|datetime('now', 'subsec')|0
5|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|cms_sites|site_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
5

================================================================================
TABLE: cms_sites
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|slug|TEXT|1||0
2|name|TEXT|1||0
3|domain|TEXT|1||0
4|theme_config|TEXT|0|'{}'|0
5|is_active|INTEGER|1|1|0
6|created_at|TEXT|1|datetime('now', 'subsec')|0
7|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
1

================================================================================
TABLE: cms_user_roles
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|user_id|BLOB|1||0
2|site_id|BLOB|1||0
3|role|TEXT|1|'viewer'|0
4|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|cms_sites|site_id|id|NO ACTION|CASCADE|NONE
1|0|users|user_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: command_history
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|user_id|BLOB|0||0
2|command_type|TEXT|1||0
3|resource_id|BLOB|0||0
4|resource_type|TEXT|0||0
5|resource_name|TEXT|0||0
6|accessed_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: companies
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0|randomblob(16)|1
1|name|TEXT|1||0
2|slug|TEXT|0||0
3|website|TEXT|0||0
4|industry|TEXT|0||0
5|description|TEXT|0||0
6|logo_url|TEXT|0||0
7|headquarters|TEXT|0||0
8|intelligence_summary|TEXT|0||0
9|intelligence_raw|TEXT|0||0
10|intelligence_status|TEXT|1|'idle'|0
11|intelligence_last_run_at|TEXT|0||0
12|intelligence_confidence|REAL|0||0
13|intelligence_agent|TEXT|0||0
14|organization_id|BLOB|0||0
15|created_by_org_id|BLOB|0||0
16|created_at|TEXT|1|datetime('now', 'subsec')|0
17|updated_at|TEXT|1|datetime('now', 'subsec')|0
18|phone|TEXT|0||0
19|email|TEXT|0||0
20|address|TEXT|0||0
21|city|TEXT|0||0
22|country|TEXT|0||0
23|founded_year|INTEGER|0||0
24|employee_count|TEXT|0||0
25|cover_image_url|TEXT|0||0
26|tags|TEXT|0|'[]'|0
27|gmb_rating|REAL|0||0
28|gmb_review_count|INTEGER|0||0
29|gmb_place_id|TEXT|0||0
30|instagram_handle|TEXT|0||0
31|linkedin_url|TEXT|0||0
32|twitter_handle|TEXT|0||0
33|facebook_url|TEXT|0||0
34|whatsapp|TEXT|0||0
35|business_hours|TEXT|0||0
36|notes|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|organizations|created_by_org_id|id|NO ACTION|NO ACTION|NONE
1|0|organizations|organization_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
36

================================================================================
TABLE: company_brand_profiles
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1|randomblob(16)|1
1|company_id|BLOB|1||0
2|tagline|TEXT|0||0
3|primary_color|TEXT|1|'#2563EB'|0
4|secondary_color|TEXT|1|'#EC4899'|0
5|accent_color|TEXT|0||0
6|typography_heading|TEXT|0||0
7|typography_body|TEXT|0||0
8|logo_url|TEXT|0||0
9|industry|TEXT|0||0
10|market_position|TEXT|0||0
11|unique_value_proposition|TEXT|0||0
12|mission_statement|TEXT|0||0
13|vision_statement|TEXT|0||0
14|brand_values|TEXT|0||0
15|brand_voice|TEXT|0||0
16|brand_archetype|TEXT|0||0
17|target_audience|TEXT|0||0
18|icp_description|TEXT|0||0
19|icp_company_size|TEXT|0||0
20|icp_industries|TEXT|0||0
21|competitor_brands|TEXT|0||0
22|differentiators|TEXT|0||0
23|content_pillars|TEXT|0||0
24|content_tone|TEXT|0||0
25|website_url|TEXT|0||0
26|social_instagram|TEXT|0||0
27|social_twitter|TEXT|0||0
28|social_linkedin|TEXT|0||0
29|social_facebook|TEXT|0||0
30|social_youtube|TEXT|0||0
31|social_tiktok|TEXT|0||0
32|research_status|TEXT|1|'idle'|0
33|research_ran_at|TEXT|0||0
34|research_summary|TEXT|0||0
35|research_iterations|INTEGER|1|0|0
36|research_depth|INTEGER|1|0|0
37|founder_name|TEXT|0||0
38|founding_year|TEXT|0||0
39|key_clients|TEXT|0||0
40|estimated_team_size|TEXT|0||0
41|tech_stack|TEXT|0||0
42|geographic_focus|TEXT|0||0
43|funding_stage|TEXT|0||0
44|content_strategy_notes|TEXT|0||0
45|awards_and_recognition|TEXT|0||0
46|brand_gap_notes|TEXT|0||0
47|mood_board_urls|TEXT|0||0
48|clearbit_logo_url|TEXT|0||0
49|brand_photography_notes|TEXT|0||0
50|created_at|TEXT|1|datetime('now', 'subsec')|0
51|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|companies|company_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
1

================================================================================
TABLE: company_contact_methods
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0|randomblob(16)|1
1|company_id|BLOB|1||0
2|method_type|TEXT|1||0
3|label|TEXT|0||0
4|value|TEXT|1||0
5|is_primary|INTEGER|1|0|0
6|created_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|companies|company_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
3

================================================================================
TABLE: company_research_passes
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|company_name|TEXT|1||0
2|company_id|BLOB|0||0
3|intake_item_id|BLOB|0||0
4|pass_number|INTEGER|1|1|0
5|research_focus|TEXT|1||0
6|status|TEXT|1|'pending'|0
7|summary|TEXT|0||0
8|key_findings|TEXT|1|'{}'|0
9|sources|TEXT|1|'[]'|0
10|confidence_score|REAL|0|0.0|0
11|agent_used|TEXT|0||0
12|error|TEXT|0||0
13|created_at|TEXT|1|datetime('now','subsec')|0
14|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|call_intake_items|intake_item_id|id|NO ACTION|CASCADE|NONE
1|0|companies|company_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
330

================================================================================
TABLE: conference_workflows
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|conference_board_id|BLOB|1||0
2|conference_name|TEXT|1||0
3|start_date|TEXT|1||0
4|end_date|TEXT|1||0
5|location|TEXT|0||0
6|timezone|TEXT|0||0
7|website|TEXT|0||0
8|status|TEXT|0|'intake'|0
9|current_stage|TEXT|0||0
10|current_stage_started_at|TEXT|0||0
11|research_flow_id|BLOB|0||0
12|content_flow_id|BLOB|0||0
13|graphics_flow_id|BLOB|0||0
14|last_qa_score|REAL|0||0
15|last_qa_run_id|BLOB|0||0
16|speakers_count|INTEGER|0|0|0
17|sponsors_count|INTEGER|0|0|0
18|side_events_count|INTEGER|0|0|0
19|social_posts_scheduled|INTEGER|0|0|0
20|social_posts_published|INTEGER|0|0|0
21|target_platform_ids|TEXT|0||0
22|config_overrides|TEXT|0||0
23|last_error|TEXT|0||0
24|retry_count|INTEGER|0|0|0
25|created_at|TEXT|1|datetime('now', 'subsec')|0
26|updated_at|TEXT|1|datetime('now', 'subsec')|0
27|completed_at|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|project_boards|conference_board_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: content_index
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|content_type|TEXT|1||0
2|content_id|TEXT|1||0
3|owner_id|BLOB|1||0
4|primary_device_id|TEXT|1||0
5|replica_device_ids|TEXT|0||0
6|is_online|BOOLEAN|0|FALSE|0
7|last_available|TIMESTAMP|0||0
8|title|TEXT|0||0
9|description|TEXT|0||0
10|tags|TEXT|0||0
11|metadata|JSON|0||0
12|visibility|TEXT|0|'private'|0
13|created_at|TIMESTAMP|1|datetime('now')|0
14|updated_at|TIMESTAMP|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|device_registry|primary_device_id|id|NO ACTION|NO ACTION|NONE
1|0|users|owner_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: content_templates
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|name|TEXT|1||0
3|description|TEXT|0||0
4|template_type|TEXT|1||0
5|content_blocks|TEXT|1||0
6|variables|TEXT|0||0
7|categories|TEXT|0||0
8|platforms|TEXT|0||0
9|times_used|INTEGER|0|0|0
10|last_used_at|TEXT|0||0
11|created_by|TEXT|0||0
12|created_at|TEXT|1|datetime('now', 'subsec')|0
13|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: context_injections
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|execution_process_id|BLOB|1||0
2|injector_id|TEXT|1||0
3|injector_name|TEXT|0||0
4|injection_type|TEXT|1||0
5|content|TEXT|1||0
6|metadata|TEXT|0||0
7|acknowledged|INTEGER|1|0|0
8|acknowledged_at|TEXT|0||0
9|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|execution_processes|execution_process_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: conversation_sessions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|session_key|TEXT|1||0
2|primary_participant_role|TEXT|0||0
3|primary_participant_id|TEXT|0||0
4|primary_participant_label|TEXT|0||0
5|created_at|TEXT|1|datetime('now', 'subsec')|0
6|last_interaction_at|TEXT|1|datetime('now', 'subsec')|0
7|metadata|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: conversation_turns
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|session_id|TEXT|1||0
2|interaction_id|TEXT|0||0
3|speaker_role|TEXT|1||0
4|speaker_id|TEXT|0||0
5|speaker_label|TEXT|0||0
6|speaker_confidence|REAL|0||0
7|channel|TEXT|0||0
8|input_type|TEXT|0||0
9|content|TEXT|0||0
10|audio_reference|TEXT|0||0
11|metadata|TEXT|0||0
12|processing_time_ms|INTEGER|0||0
13|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|conversation_sessions|session_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: crm_activities
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|0||0
2|organization_id|TEXT|1||0
3|client_id|TEXT|0||0
4|crm_contact_id|TEXT|0||0
5|crm_deal_id|TEXT|0||0
6|activity_type|TEXT|1||0
7|subject|TEXT|0||0
8|description|TEXT|0||0
9|outcome|TEXT|0||0
10|email_message_id|TEXT|0||0
11|social_mention_id|TEXT|0||0
12|task_id|TEXT|0||0
13|performed_by_user|TEXT|0||0
14|performed_by_agent_id|TEXT|0||0
15|metadata|TEXT|0||0
16|duration_minutes|INTEGER|0||0
17|activity_at|TEXT|1||0
18|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|crm_deals|crm_deal_id|id|NO ACTION|SET NULL|NONE
1|0|crm_contacts|crm_contact_id|id|NO ACTION|CASCADE|NONE
2|0|clients|client_id|id|NO ACTION|SET NULL|NONE
3|0|organizations|organization_id|id|NO ACTION|CASCADE|NONE
4|0|projects|project_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
56

================================================================================
TABLE: crm_contacts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|0||0
2|organization_id|TEXT|1||0
3|client_id|TEXT|0||0
4|first_name|TEXT|0||0
5|last_name|TEXT|0||0
6|full_name|TEXT|0||0
7|email|TEXT|0||0
8|phone|TEXT|0||0
9|mobile|TEXT|0||0
10|avatar_url|TEXT|0||0
11|company_name|TEXT|0||0
12|job_title|TEXT|0||0
13|department|TEXT|0||0
14|linkedin_url|TEXT|0||0
15|twitter_handle|TEXT|0||0
16|website|TEXT|0||0
17|source|TEXT|0||0
18|lifecycle_stage|TEXT|0|'lead'|0
19|lead_score|INTEGER|0|0|0
20|last_activity_at|TEXT|0||0
21|last_contacted_at|TEXT|0||0
22|last_replied_at|TEXT|0||0
23|owner_user_id|TEXT|0||0
24|assigned_agent_id|TEXT|0||0
25|zoho_contact_id|TEXT|0||0
26|gmail_contact_id|TEXT|0||0
27|external_ids|TEXT|0||0
28|tags|TEXT|0||0
29|lists|TEXT|0||0
30|custom_fields|TEXT|0||0
31|address_line1|TEXT|0||0
32|address_line2|TEXT|0||0
33|city|TEXT|0||0
34|state|TEXT|0||0
35|postal_code|TEXT|0||0
36|country|TEXT|0||0
37|email_opt_in|INTEGER|0|1|0
38|sms_opt_in|INTEGER|0|0|0
39|do_not_contact|INTEGER|0|0|0
40|email_count|INTEGER|0|0|0
41|meeting_count|INTEGER|0|0|0
42|deal_count|INTEGER|0|0|0
43|total_revenue|REAL|0|0.0|0
44|source_data_source_id|TEXT|0||0
45|source_workflow_run_id|TEXT|0||0
46|created_at|TEXT|1|datetime('now', 'subsec')|0
47|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|clients|client_id|id|NO ACTION|SET NULL|NONE
1|0|organizations|organization_id|id|NO ACTION|CASCADE|NONE
2|0|projects|project_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
42

================================================================================
TABLE: crm_deals
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|0||0
2|organization_id|TEXT|1||0
3|client_id|TEXT|0||0
4|crm_contact_id|TEXT|0||0
5|crm_pipeline_id|TEXT|0||0
6|crm_stage_id|TEXT|0||0
7|position|INTEGER|0|0|0
8|name|TEXT|1||0
9|description|TEXT|0||0
10|amount|REAL|0||0
11|currency|TEXT|0|'USD'|0
12|pipeline|TEXT|0|'default'|0
13|stage|TEXT|1|'lead'|0
14|probability|INTEGER|0|0|0
15|expected_close_date|TEXT|0||0
16|actual_close_date|TEXT|0||0
17|last_activity_at|TEXT|0||0
18|owner_user_id|TEXT|0||0
19|assigned_agent_id|TEXT|0||0
20|zoho_deal_id|TEXT|0||0
21|external_ids|TEXT|0||0
22|tags|TEXT|0||0
23|custom_fields|TEXT|0||0
24|lost_reason|TEXT|0||0
25|win_reason|TEXT|0||0
26|source_data_source_id|TEXT|0||0
27|source_workflow_run_id|TEXT|0||0
28|created_at|TEXT|1|datetime('now', 'subsec')|0
29|updated_at|TEXT|1|datetime('now', 'subsec')|0
30|proposal_text|TEXT|0||0
31|proposal_status|TEXT|1|'draft'|0
32|deck_url|TEXT|0||0
33|invoice_id|TEXT|0||0
34|won_at|TEXT|0||0
35|lost_at|TEXT|0||0
36|expedited|INTEGER|1|0|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|crm_pipeline_stages|crm_stage_id|id|NO ACTION|SET NULL|NONE
1|0|crm_pipelines|crm_pipeline_id|id|NO ACTION|SET NULL|NONE
2|0|crm_contacts|crm_contact_id|id|NO ACTION|SET NULL|NONE
3|0|clients|client_id|id|NO ACTION|SET NULL|NONE
4|0|organizations|organization_id|id|NO ACTION|CASCADE|NONE
5|0|projects|project_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
38

================================================================================
TABLE: crm_pipeline_stages
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|pipeline_id|TEXT|1||0
2|name|TEXT|1||0
3|description|TEXT|0||0
4|color|TEXT|1|'#6B7280'|0
5|position|INTEGER|1|0|0
6|is_closed|INTEGER|0|0|0
7|is_won|INTEGER|0|0|0
8|probability|INTEGER|0|0|0
9|auto_move_after_days|INTEGER|0||0
10|notify_on_enter|INTEGER|0|0|0
11|created_at|TEXT|1|datetime('now', 'subsec')|0
12|updated_at|TEXT|1|datetime('now', 'subsec')|0
13|stage_type|TEXT|0||0
14|stage_config|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|crm_pipelines|pipeline_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
3934

================================================================================
TABLE: crm_pipelines
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|0||0
2|organization_id|TEXT|1||0
3|client_id|TEXT|0||0
4|name|TEXT|1||0
5|description|TEXT|0||0
6|pipeline_type|TEXT|1||0
7|is_active|INTEGER|0|1|0
8|is_default|INTEGER|0|0|0
9|icon|TEXT|0||0
10|color|TEXT|0||0
11|created_at|TEXT|1|datetime('now', 'subsec')|0
12|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|clients|client_id|id|NO ACTION|SET NULL|NONE
1|0|organizations|organization_id|id|NO ACTION|CASCADE|NONE
2|0|projects|project_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
22

================================================================================
TABLE: custom_field_definitions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|name|TEXT|1||0
3|field_type|TEXT|1||0
4|required|INTEGER|1|0|0
5|options|TEXT|0||0
6|default_value|TEXT|0||0
7|metadata|TEXT|0||0
8|created_at|TEXT|1|datetime('now', 'subsec')|0
9|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: data_replication_state
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|source_device_id|TEXT|1||0
2|destination_device_id|TEXT|1||0
3|data_owner_id|BLOB|1||0
4|data_type|TEXT|1||0
5|last_sync_timestamp|TIMESTAMP|0||0
6|last_sync_version|INTEGER|0|0|0
7|current_version|INTEGER|0|0|0
8|sync_status|TEXT|0|'in_sync'|0
9|source_checksum|TEXT|0||0
10|destination_checksum|TEXT|0||0
11|sync_enabled|BOOLEAN|0|TRUE|0
12|sync_interval_minutes|INTEGER|0|5|0
13|auto_sync|BOOLEAN|0|TRUE|0
14|last_error|TEXT|0||0
15|error_count|INTEGER|0|0|0
16|created_at|TIMESTAMP|1|datetime('now')|0
17|updated_at|TIMESTAMP|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|data_owner_id|id|NO ACTION|CASCADE|NONE
1|0|device_registry|destination_device_id|id|NO ACTION|CASCADE|NONE
2|0|device_registry|source_device_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: data_sources
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|organization_id|TEXT|0||0
2|project_id|TEXT|0||0
3|created_by|TEXT|0||0
4|title|TEXT|1||0
5|description|TEXT|0||0
6|data_type|TEXT|1|'conversation'|0
7|file_type|TEXT|0||0
8|file_name|TEXT|0||0
9|file_path|TEXT|0||0
10|file_size_bytes|INTEGER|0||0
11|file_hash|TEXT|0||0
12|metadata|TEXT|1|'{}'|0
13|status|TEXT|1|'pending'|0
14|processing_error|TEXT|0||0
15|created_at|DATETIME|1|datetime('now', 'subsec')|0
16|updated_at|DATETIME|1|datetime('now', 'subsec')|0
17|archived_at|DATETIME|0||0
18|source_type|TEXT|1|'file'|0
19|content|TEXT|0||0
20|folder|TEXT|1|'Unfiled'|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
1713

================================================================================
TABLE: deal_transcripts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1|lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(6)))|1
1|deal_id|TEXT|1||0
2|intake_item_id|TEXT|0||0
3|call_log_id|TEXT|0||0
4|transcript_text|TEXT|0||0
5|summary|TEXT|0||0
6|matched_at|TEXT|1|datetime('now','subsec')|0
7|matched_by|TEXT|0||0
8|created_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|call_intake_items|intake_item_id|id|NO ACTION|SET NULL|NONE
1|0|crm_deals|deal_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
9

================================================================================
TABLE: deletion_audit_log
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|table_name|TEXT|1||0
2|record_id|BLOB|1||0
3|deleted_by|BLOB|0||0
4|deleted_at|TEXT|1|datetime('now')|0
5|record_data|TEXT|0||0
6|reason|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|deleted_by|id|NO ACTION|SET NULL|NONE
1|0|users|deleted_by|id|NO ACTION|SET NULL|NONE

--- Row count ---
0

================================================================================
TABLE: deliverables
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|proposal_id|BLOB|0||0
3|deliverable_type|TEXT|1|'other'|0
4|title|TEXT|1|''|0
5|description|TEXT|1|''|0
6|status|TEXT|1|'working'|0
7|revision_rounds_allowed|INTEGER|1|2|0
8|revision_rounds_used|INTEGER|1|0|0
9|working_file_url|TEXT|0||0
10|final_link|TEXT|0||0
11|due_date|TEXT|0||0
12|delivered_at|TEXT|0||0
13|created_at|TEXT|1|datetime('now','subsec')|0
14|updated_at|TEXT|1|datetime('now','subsec')|0
15|artifact_id|BLOB|0||0
16|knowledge_source_id|BLOB|0||0
17|client_decision|TEXT|0||0
18|client_decision_at|TEXT|0||0
19|client_decision_note|TEXT|0||0
20|source_clips|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|project_knowledge_sources|knowledge_source_id|id|NO ACTION|SET NULL|NONE
1|0|execution_artifacts|artifact_id|id|NO ACTION|SET NULL|NONE
2|0|proposals|proposal_id|id|NO ACTION|SET NULL|NONE
3|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
22

================================================================================
TABLE: device_project_access
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|device_id|TEXT|1||0
2|project_id|BLOB|1||0
3|access_level|TEXT|1||0
4|granted_at|DATETIME|0|CURRENT_TIMESTAMP|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE
1|0|devices|device_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
6

================================================================================
TABLE: device_registry
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|owner_id|BLOB|0||0
2|device_name|TEXT|1||0
3|device_type|TEXT|1||0
4|apn_node_id|TEXT|1||0
5|public_key|TEXT|1||0
6|is_online|BOOLEAN|1|FALSE|0
7|last_seen|TIMESTAMP|0||0
8|last_heartbeat|TIMESTAMP|0||0
9|storage_capacity_gb|INTEGER|0|0|0
10|serves_data|BOOLEAN|0|FALSE|0
11|accepts_storage_contracts|BOOLEAN|0|FALSE|0
12|ip_address|TEXT|0||0
13|location|TEXT|0||0
14|hardware_info|TEXT|0||0
15|created_at|TIMESTAMP|1|datetime('now')|0
16|updated_at|TIMESTAMP|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|owner_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: devices
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|owner_id|BLOB|1||0
2|hostname|TEXT|1||0
3|wallet_address|TEXT|1||0
4|device_tier|TEXT|1||0
5|uptime_percent|REAL|0|0.0|0
6|is_online|INTEGER|0|0|0
7|total_cores|INTEGER|1||0
8|total_ram_gb|INTEGER|1||0
9|gpu_available|INTEGER|0|0|0
10|gpu_model|TEXT|0||0
11|active_contribution_percent|REAL|0|10.0|0
12|idle_contribution_percent|REAL|0|80.0|0
13|is_primary_node|INTEGER|0|0|0
14|vibe_earned_total|REAL|0|0.0|0
15|last_seen|DATETIME|0||0
16|created_at|DATETIME|0|CURRENT_TIMESTAMP|0
17|updated_at|DATETIME|0|CURRENT_TIMESTAMP|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|owner_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
2

================================================================================
TABLE: dropbox_sources
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|account_id|TEXT|1||0
2|label|TEXT|1||0
3|source_url|TEXT|0||0
4|project_id|TEXT|0||0
5|storage_tier|TEXT|1||0
6|checksum_required|BOOLEAN|1|1|0
7|reference_name_template|TEXT|0||0
8|ingest_strategy|TEXT|1|'shared_link'|0
9|access_token|TEXT|0||0
10|cursor|TEXT|0||0
11|last_processed_at|TEXT|0||0
12|auto_ingest|BOOLEAN|1|1|0
13|created_at|TEXT|1||0
14|updated_at|TEXT|1||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
0

================================================================================
TABLE: edit_sessions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|batch_id|TEXT|1||0
2|deliverable_type|TEXT|1||0
3|aspect_ratios|TEXT|1|'[]'|0
4|reference_style|TEXT|0||0
5|include_captions|BOOLEAN|1|0|0
6|imovie_project|TEXT|1||0
7|status|TEXT|1||0
8|timelines|TEXT|1|'[]'|0
9|metadata|TEXT|1|'{}'|0
10|created_at|TEXT|1||0
11|updated_at|TEXT|1||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|media_batches|batch_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
3

================================================================================
TABLE: email_accounts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|0||0
2|provider|TEXT|1||0
3|account_type|TEXT|1|'primary'|0
4|email_address|TEXT|1||0
5|display_name|TEXT|0||0
6|avatar_url|TEXT|0||0
7|access_token|TEXT|0||0
8|refresh_token|TEXT|0||0
9|token_expires_at|TEXT|0||0
10|imap_host|TEXT|0||0
11|imap_port|INTEGER|0||0
12|smtp_host|TEXT|0||0
13|smtp_port|INTEGER|0||0
14|use_ssl|INTEGER|1|1|0
15|granted_scopes|TEXT|0||0
16|storage_used_bytes|INTEGER|0||0
17|storage_total_bytes|INTEGER|0||0
18|unread_count|INTEGER|1|0|0
19|metadata|TEXT|0||0
20|status|TEXT|1|'active'|0
21|last_sync_at|TEXT|0||0
22|last_error|TEXT|0||0
23|sync_enabled|INTEGER|1|1|0
24|sync_frequency_minutes|INTEGER|1|15|0
25|auto_reply_enabled|INTEGER|1|0|0
26|signature|TEXT|0||0
27|created_at|TEXT|1|datetime('now', 'subsec')|0
28|updated_at|TEXT|1|datetime('now', 'subsec')|0
29|owner_type|TEXT|1|'project'|0
30|owner_id|TEXT|1|''|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
3

================================================================================
TABLE: email_messages
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|email_account_id|BLOB|1||0
2|project_id|BLOB|1||0
3|provider_message_id|TEXT|1||0
4|thread_id|TEXT|0||0
5|from_address|TEXT|1||0
6|from_name|TEXT|0||0
7|to_addresses|TEXT|1||0
8|cc_addresses|TEXT|0||0
9|bcc_addresses|TEXT|0||0
10|reply_to|TEXT|0||0
11|subject|TEXT|0||0
12|body_text|TEXT|0||0
13|body_html|TEXT|0||0
14|snippet|TEXT|0||0
15|has_attachments|INTEGER|0|0|0
16|attachments|TEXT|0||0
17|labels|TEXT|0||0
18|is_read|INTEGER|0|0|0
19|is_starred|INTEGER|0|0|0
20|is_draft|INTEGER|0|0|0
21|is_sent|INTEGER|0|0|0
22|is_archived|INTEGER|0|0|0
23|is_spam|INTEGER|0|0|0
24|is_trash|INTEGER|0|0|0
25|in_reply_to|TEXT|0||0
26|references|TEXT|0||0
27|crm_contact_id|BLOB|0||0
28|crm_deal_id|BLOB|0||0
29|assigned_agent_id|BLOB|0||0
30|sentiment|TEXT|0||0
31|priority|TEXT|0|'normal'|0
32|needs_response|INTEGER|0|0|0
33|response_due_at|TEXT|0||0
34|responded_at|TEXT|0||0
35|received_at|TEXT|1||0
36|sent_at|TEXT|0||0
37|created_at|TEXT|1|datetime('now', 'subsec')|0
38|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agents|assigned_agent_id|id|NO ACTION|SET NULL|NONE
1|0|crm_deals|crm_deal_id|id|NO ACTION|SET NULL|NONE
2|0|crm_contacts|crm_contact_id|id|NO ACTION|SET NULL|NONE
3|0|projects|project_id|id|NO ACTION|CASCADE|NONE
4|0|email_accounts|email_account_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
33

================================================================================
TABLE: email_templates
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|name|TEXT|1||0
3|description|TEXT|0||0
4|category|TEXT|0||0
5|subject|TEXT|1||0
6|body_html|TEXT|1||0
7|body_text|TEXT|0||0
8|variables|TEXT|0||0
9|times_used|INTEGER|0|0|0
10|last_used_at|TEXT|0||0
11|sent_count|INTEGER|0|0|0
12|open_count|INTEGER|0|0|0
13|click_count|INTEGER|0|0|0
14|reply_count|INTEGER|0|0|0
15|created_by|TEXT|0||0
16|created_at|TEXT|1|datetime('now', 'subsec')|0
17|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: entities
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|entity_type|TEXT|1||0
2|canonical_name|TEXT|1||0
3|slug|TEXT|1||0
4|external_ids|TEXT|0||0
5|bio|TEXT|0||0
6|title|TEXT|0||0
7|company|TEXT|0||0
8|photo_url|TEXT|0||0
9|social_profiles|TEXT|0||0
10|social_analysis|TEXT|0||0
11|data_completeness|REAL|0|0.0|0
12|last_researched_at|TEXT|0||0
13|created_at|TEXT|1|datetime('now', 'subsec')|0
14|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
2

================================================================================
TABLE: entity_aliases
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|entity_id|BLOB|1||0
2|alias|TEXT|1||0
3|alias_type|TEXT|0|'nickname'|0
4|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|entities|entity_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: entity_appearances
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|entity_id|BLOB|1||0
2|conference_board_id|BLOB|1||0
3|appearance_type|TEXT|1||0
4|talk_title|TEXT|0||0
5|talk_description|TEXT|0||0
6|talk_slot|TEXT|0||0
7|research_artifact_id|BLOB|0||0
8|status|TEXT|0|'discovered'|0
9|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|project_boards|conference_board_id|id|NO ACTION|CASCADE|NONE
1|0|entities|entity_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: entity_graph_edges
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0|randomblob(16)|1
1|from_node_id|BLOB|1||0
2|to_node_id|BLOB|1||0
3|edge_type|TEXT|1||0
4|weight|REAL|0|1.0|0
5|metadata|TEXT|0||0
6|created_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|entity_graph_nodes|to_node_id|id|NO ACTION|CASCADE|NONE
1|0|entity_graph_nodes|from_node_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
335

================================================================================
TABLE: entity_graph_nodes
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0|randomblob(16)|1
1|node_type|TEXT|1||0
2|ref_id|TEXT|1||0
3|ref_table|TEXT|1||0
4|label|TEXT|1||0
5|metadata|TEXT|0||0
6|created_at|TEXT|1|datetime('now','subsec')|0
7|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
478

================================================================================
TABLE: execution_artifacts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|execution_process_id|BLOB|0||0
2|artifact_type|TEXT|1||0
3|title|TEXT|1||0
4|content|TEXT|0||0
5|file_path|TEXT|0||0
6|metadata|TEXT|0||0
7|phase|TEXT|0||0
8|created_by_agent_id|BLOB|0||0
9|review_status|TEXT|0|'none'|0
10|parent_artifact_id|BLOB|0||0
11|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agents|created_by_agent_id|id|NO ACTION|SET NULL|NONE
1|0|execution_processes|execution_process_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
39

================================================================================
TABLE: execution_checkpoints
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|execution_process_id|BLOB|1||0
2|checkpoint_definition_id|BLOB|0||0
3|checkpoint_data|TEXT|1||0
4|trigger_reason|TEXT|0||0
5|status|TEXT|1|'pending'|0
6|reviewer_id|TEXT|0||0
7|reviewer_name|TEXT|0||0
8|review_note|TEXT|0||0
9|reviewed_at|TEXT|0||0
10|expires_at|TEXT|0||0
11|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|checkpoint_definitions|checkpoint_definition_id|id|NO ACTION|NO ACTION|NONE
1|0|execution_processes|execution_process_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: execution_handoffs
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|execution_process_id|BLOB|1||0
2|from_actor_type|TEXT|1||0
3|from_actor_id|TEXT|1||0
4|from_actor_name|TEXT|0||0
5|to_actor_type|TEXT|1||0
6|to_actor_id|TEXT|1||0
7|to_actor_name|TEXT|0||0
8|handoff_type|TEXT|1||0
9|reason|TEXT|0||0
10|context_snapshot|TEXT|0||0
11|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|execution_processes|execution_process_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: execution_pause_history
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|execution_process_id|BLOB|1||0
2|action|TEXT|1||0
3|reason|TEXT|0||0
4|initiated_by|TEXT|1||0
5|initiated_by_name|TEXT|0||0
6|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|execution_processes|execution_process_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: execution_process_logs
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|execution_id|BLOB|0||1
1|logs|TEXT|1||0
2|byte_size|INTEGER|1||0
3|inserted_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|execution_processes|execution_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
17

================================================================================
TABLE: execution_processes
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|task_attempt_id|BLOB|1||0
2|status|TEXT|1|'running'|0
3|exit_code|INTEGER|0||0
4|started_at|TEXT|1|datetime('now', 'subsec')|0
5|completed_at|TEXT|0||0
6|created_at|TEXT|1|datetime('now', 'subsec')|0
7|updated_at|TEXT|1|datetime('now', 'subsec')|0
8|run_reason|TEXT|1|'setupscript'|0
9|executor_action|TEXT|1|''|0
10|dropped|BOOLEAN|1|0|0
11|after_head_commit|TEXT|0||0
12|before_head_commit|TEXT|0||0
13|slot_id|BLOB|0||0
14|priority|INTEGER|0|0|0
15|control_state|TEXT|0|'running'|0
16|paused_at|TEXT|0||0
17|pause_reason|TEXT|0||0
18|ralph_iteration|INTEGER|0||0
19|ralph_loop_id|TEXT|0||0
20|agent_session_id|BLOB|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|task_attempts|task_attempt_id|id|NO ACTION|CASCADE|NONE
1|0|agent_sessions|agent_session_id|id|NO ACTION|SET NULL|NONE
2|0|ralph_loop_state|ralph_loop_id|id|NO ACTION|NO ACTION|NONE
3|0|execution_slots|slot_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
17

================================================================================
TABLE: execution_slots
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|task_attempt_id|BLOB|1||0
2|slot_type|TEXT|1||0
3|resource_weight|INTEGER|1|1|0
4|acquired_at|TEXT|1|datetime('now', 'subsec')|0
5|released_at|TEXT|0||0
6|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|task_attempts|task_attempt_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: execution_summaries
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|task_attempt_id|TEXT|1||0
2|execution_process_id|TEXT|0||0
3|files_modified|INTEGER|1|0|0
4|files_created|INTEGER|1|0|0
5|files_deleted|INTEGER|1|0|0
6|commands_run|INTEGER|1|0|0
7|commands_failed|INTEGER|1|0|0
8|tools_used|TEXT|0||0
9|completion_status|TEXT|1|'full'|0
10|blocker_summary|TEXT|0||0
11|error_summary|TEXT|0||0
12|execution_time_ms|INTEGER|1|0|0
13|human_rating|INTEGER|0||0
14|human_notes|TEXT|0||0
15|is_reference_example|BOOLEAN|1|FALSE|0
16|workflow_tags|TEXT|0||0
17|created_at|TEXT|1|datetime('now')|0
18|updated_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|execution_processes|execution_process_id|id|NO ACTION|SET NULL|NONE
1|0|task_attempts|task_attempt_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: executor_sessions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|task_attempt_id|BLOB|1||0
2|execution_process_id|BLOB|1||0
3|session_id|TEXT|0||0
4|prompt|TEXT|0||0
5|created_at|TEXT|1|datetime('now', 'subsec')|0
6|updated_at|TEXT|1|datetime('now', 'subsec')|0
7|summary|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|execution_processes|execution_process_id|id|NO ACTION|CASCADE|NONE
1|0|task_attempts|task_attempt_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
17

================================================================================
TABLE: favorites
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|user_id|BLOB|0||0
2|project_id|BLOB|1||0
3|position|INTEGER|0|0|0
4|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: follow_up_drafts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|task_attempt_id|TEXT|1||0
2|prompt|TEXT|1|''|0
3|queued|INTEGER|1|0|0
4|sending|INTEGER|1|0|0
5|version|INTEGER|1|0|0
6|variant|TEXT|0||0
7|image_ids|TEXT|0||0
8|created_at|DATETIME|1|CURRENT_TIMESTAMP|0
9|updated_at|DATETIME|1|CURRENT_TIMESTAMP|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|task_attempts|task_attempt_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
3

================================================================================
TABLE: gate_approvals
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|approval_gate_id|BLOB|1||0
2|execution_process_id|BLOB|1||0
3|approver_id|TEXT|1||0
4|approver_name|TEXT|0||0
5|decision|TEXT|1||0
6|comment|TEXT|0||0
7|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|execution_processes|execution_process_id|id|NO ACTION|CASCADE|NONE
1|0|approval_gates|approval_gate_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: gateway_requests
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|subscription_id|BLOB|1||0
2|listing_id|BLOB|0||0
3|service_type|TEXT|1||0
4|provider_node_id|TEXT|0||0
5|provider_wallet|TEXT|0||0
6|request_method|TEXT|0|'POST'|0
7|request_path|TEXT|0||0
8|request_size_bytes|INTEGER|1|0|0
9|response_size_bytes|INTEGER|1|0|0
10|status|TEXT|1|'pending'|0
11|error_message|TEXT|0||0
12|vibe_charged|REAL|1|0.0|0
13|vibe_rewarded|REAL|1|0.0|0
14|response_ms|INTEGER|0||0
15|created_at|TEXT|1|datetime('now','subsec')|0
16|fulfilled_at|TEXT|0|NULL|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|marketplace_listings|listing_id|id|NO ACTION|NO ACTION|NONE
1|0|marketplace_subscriptions|subscription_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
0

================================================================================
TABLE: images
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|file_path|TEXT|1||0
2|original_name|TEXT|1||0
3|mime_type|TEXT|0||0
4|size_bytes|INTEGER|0||0
5|hash|TEXT|1||0
6|created_at|TEXT|1|datetime('now', 'subsec')|0
7|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: invoices
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|invoice_number|TEXT|1||0
2|person_id|BLOB|0||0
3|organization_id|BLOB|0||0
4|project_id|BLOB|0||0
5|invoice_type|TEXT|1|'ar'|0
6|status|TEXT|1|'draft'|0
7|amount_usd|REAL|1|0.0|0
8|amount_vibe|INTEGER|1|0|0
9|currency|TEXT|1|'USD'|0
10|title|TEXT|0||0
11|description|TEXT|0||0
12|line_items|TEXT|1|'[]'|0
13|issue_date|TEXT|0||0
14|due_date|TEXT|0||0
15|paid_at|TEXT|0||0
16|payment_method|TEXT|0||0
17|payment_reference|TEXT|0||0
18|notes|TEXT|0||0
19|metadata|TEXT|1|'{}'|0
20|created_by|BLOB|0||0
21|created_at|TEXT|1|datetime('now', 'subsec')|0
22|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
4

================================================================================
TABLE: marketplace_listings
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|provider_type|TEXT|1|'apn_node'|0
2|provider_node_id|TEXT|0||0
3|provider_org_id|BLOB|0||0
4|provider_wallet|TEXT|1||0
5|service_type|TEXT|1||0
6|service_name|TEXT|1||0
7|service_description|TEXT|0||0
8|tags|TEXT|1|'[]'|0
9|pricing_model|TEXT|1|'per-request'|0
10|price_vibe|REAL|1|1.0|0
11|min_vibe|REAL|1|0.0|0
12|max_concurrent|INTEGER|0|10|0
13|endpoint_url|TEXT|0||0
14|region|TEXT|0||0
15|status|TEXT|1|'active'|0
16|uptime_pct|REAL|0|NULL|0
17|avg_response_ms|INTEGER|0|NULL|0
18|metadata|TEXT|1|'{}'|0
19|created_at|TEXT|1|datetime('now','subsec')|0
20|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|organizations|provider_org_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
1

================================================================================
TABLE: marketplace_subscriptions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|consumer_name|TEXT|1||0
2|consumer_type|TEXT|1|'external'|0
3|consumer_project_id|BLOB|0||0
4|consumer_org_id|BLOB|0||0
5|contact_email|TEXT|0||0
6|service_type|TEXT|1||0
7|listing_id|BLOB|0||0
8|api_key|TEXT|1||0
9|api_key_prefix|TEXT|1||0
10|vibe_budget|REAL|1|0.0|0
11|vibe_spent|REAL|1|0.0|0
12|auto_refill_threshold|REAL|0|NULL|0
13|auto_refill_amount|REAL|0|NULL|0
14|status|TEXT|1|'active'|0
15|suspend_reason|TEXT|0||0
16|created_at|TEXT|1|datetime('now','subsec')|0
17|updated_at|TEXT|1|datetime('now','subsec')|0
18|expires_at|TEXT|0|NULL|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|marketplace_listings|listing_id|id|NO ACTION|NO ACTION|NONE
1|0|organizations|consumer_org_id|id|NO ACTION|NO ACTION|NONE
2|0|projects|consumer_project_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
1

================================================================================
TABLE: media_assets
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|batch_id|BLOB|0||0
3|filename|TEXT|1||0
4|file_path|TEXT|1||0
5|file_size_bytes|INTEGER|0|0|0
6|mime_type|TEXT|0|'video/mp4'|0
7|duration_seconds|REAL|0||0
8|width|INTEGER|0||0
9|height|INTEGER|0||0
10|ai_description|TEXT|0||0
11|shot_type|TEXT|0||0
12|energy_level|REAL|0|0.5|0
13|motion_intensity|REAL|0|0.5|0
14|dominant_colors|TEXT|0|'[]'|0
15|scene_tags|TEXT|0|'[]'|0
16|ai_confidence|REAL|0|0.0|0
17|analysis_status|TEXT|0|'pending'|0
18|created_at|TEXT|0|datetime('now','subsec')|0
19|updated_at|TEXT|0|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
7

================================================================================
TABLE: media_assets_fts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|asset_id||0||0
1|description||0||0
2|shot_type||0||0
3|tags||0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
7

================================================================================
TABLE: media_assets_fts_config
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|k||1||1
1|v||0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
1

================================================================================
TABLE: media_assets_fts_content
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|INTEGER|0||1
1|c0||0||0
2|c1||0||0
3|c2||0||0
4|c3||0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
7

================================================================================
TABLE: media_assets_fts_data
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|INTEGER|0||1
1|block|BLOB|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
7

================================================================================
TABLE: media_assets_fts_docsize
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|INTEGER|0||1
1|sz|BLOB|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
7

================================================================================
TABLE: media_assets_fts_idx
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|segid||1||1
1|term||1||2
2|pgno||0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
5

================================================================================
TABLE: media_batch_analyses
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|batch_id|TEXT|1||0
2|brief|TEXT|1||0
3|summary|TEXT|1||0
4|passes_completed|INTEGER|1||0
5|deliverable_targets|TEXT|1|'[]'|0
6|hero_moments|TEXT|1|'[]'|0
7|insights|TEXT|1|'{}'|0
8|created_at|TEXT|1||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|media_batches|batch_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: media_batches
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|0||0
2|reference_name|TEXT|0||0
3|source_url|TEXT|1||0
4|storage_tier|TEXT|1||0
5|checksum_required|BOOLEAN|1|1|0
6|status|TEXT|1||0
7|file_count|INTEGER|1|0|0
8|total_size_bytes|INTEGER|1|0|0
9|last_error|TEXT|0||0
10|metadata|TEXT|1|'{}'|0
11|created_at|TEXT|1||0
12|updated_at|TEXT|1||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
4

================================================================================
TABLE: media_file_beat_analyses
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|media_file_id|TEXT|1||0
2|batch_id|TEXT|1||0
3|file_path|TEXT|1||0
4|bpm|REAL|1|120.0|0
5|beat_interval|REAL|1|0.5|0
6|total_beats|INTEGER|1|0|0
7|beats_per_bar|INTEGER|1|4|0
8|beats|TEXT|1|'[]'|0
9|sections|TEXT|1|'[]'|0
10|energy_curve|TEXT|1|'[]'|0
11|created_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|media_batches|batch_id|id|NO ACTION|CASCADE|NONE
1|0|media_files|media_file_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: media_file_scene_analyses
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|media_file_id|TEXT|1||0
2|batch_id|TEXT|1||0
3|file_path|TEXT|1||0
4|overall_energy|REAL|1|0.0|0
5|peak_energy_timestamp|REAL|1|0.0|0
6|dominant_content_type|TEXT|1|'ambient'|0
7|usable|BOOLEAN|1|1|0
8|segments|TEXT|1|'[]'|0
9|created_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|media_batches|batch_id|id|NO ACTION|CASCADE|NONE
1|0|media_files|media_file_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: media_file_tags
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|media_file_id|TEXT|1||0
2|batch_id|TEXT|1||0
3|tag|TEXT|1||0
4|source|TEXT|1||0
5|confidence|REAL|1|1.0|0
6|created_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|media_batches|batch_id|id|NO ACTION|CASCADE|NONE
1|0|media_files|media_file_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: media_file_visual_qcs
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|media_file_id|TEXT|1||0
2|batch_id|TEXT|1||0
3|file_path|TEXT|1||0
4|best_in_point|REAL|1|0.0|0
5|best_composition_score|REAL|1|0.0|0
6|qc_passed|BOOLEAN|1|0|0
7|summary|TEXT|1|''|0
8|analyzed_frames|TEXT|1|'[]'|0
9|created_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|media_batches|batch_id|id|NO ACTION|CASCADE|NONE
1|0|media_files|media_file_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: media_files
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|batch_id|TEXT|1||0
2|filename|TEXT|1||0
3|file_path|TEXT|1||0
4|size_bytes|INTEGER|1||0
5|checksum_sha256|TEXT|0||0
6|duration_seconds|REAL|0||0
7|resolution|TEXT|0||0
8|codec|TEXT|0||0
9|fps|REAL|0||0
10|metadata|TEXT|1|'{}'|0
11|created_at|TEXT|1||0
12|scene_analysis_status|TEXT|0|NULL|0
13|visual_qc_status|TEXT|0|NULL|0
14|beat_analysis_status|TEXT|0|NULL|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|media_batches|batch_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
22

================================================================================
TABLE: meeting_segments
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|meeting_session_id|TEXT|1||0
2|segment_index|INTEGER|1||0
3|speaker_label|TEXT|0||0
4|text|TEXT|1||0
5|confidence|REAL|0|0.0|0
6|start_time_ms|INTEGER|1||0
7|end_time_ms|INTEGER|1||0
8|is_topsi_addressed|BOOLEAN|0|0|0
9|metadata|TEXT|0|'{}'|0
10|created_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|meeting_sessions|meeting_session_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
6140

================================================================================
TABLE: meeting_sessions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|title|TEXT|1|'Untitled Meeting'|0
3|status|TEXT|1|'active'|0
4|started_by|TEXT|1||0
5|started_at|TEXT|1|datetime('now','subsec')|0
6|ended_at|TEXT|0||0
7|duration_seconds|INTEGER|0||0
8|participant_count|INTEGER|0|0|0
9|participants|TEXT|0|'[]'|0
10|transcript|TEXT|0|'[]'|0
11|notes|TEXT|0||0
12|metadata|TEXT|0|'{}'|0
13|topology_node_id|TEXT|0||0
14|access_level|TEXT|0|'admin'|0
15|shared_with|TEXT|0|'[]'|0
16|created_at|TEXT|1|datetime('now','subsec')|0
17|updated_at|TEXT|1|datetime('now','subsec')|0
18|linked_person_ids|TEXT|1|'[]'|0
19|company_id|BLOB|0||0
20|proposal_id|BLOB|0||0
21|attendee_person_ids|TEXT|1|'[]'|0
22|knowledge_source_id|TEXT|0||0
23|source_type|TEXT|1|'browser'|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|proposals|proposal_id|id|NO ACTION|SET NULL|NONE
1|0|companies|company_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
94

================================================================================
TABLE: merges
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|task_attempt_id|BLOB|1||0
2|merge_type|TEXT|1||0
3|merge_commit|TEXT|0||0
4|pr_number|INTEGER|0||0
5|pr_url|TEXT|0||0
6|pr_status|TEXT|0||0
7|pr_merged_at|TEXT|0||0
8|pr_merge_commit_sha|TEXT|0||0
9|created_at|TEXT|1|datetime('now', 'subsec')|0
10|target_branch_name|TEXT|1||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|task_attempts|task_attempt_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: model_pricing
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|model|TEXT|1||0
2|provider|TEXT|1||0
3|input_cost_per_million|INTEGER|1||0
4|output_cost_per_million|INTEGER|1||0
5|multiplier|REAL|1|2.0|0
6|effective_from|TEXT|1|datetime('now')|0
7|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
16

================================================================================
TABLE: nodes
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|owner_id|BLOB|1||0
2|hostname|TEXT|1||0
3|is_primary|INTEGER|0|0|0
4|node_type|TEXT|0|'master'|0
5|last_seen|DATETIME|0||0
6|created_at|DATETIME|0|CURRENT_TIMESTAMP|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|devices|id|id|NO ACTION|CASCADE|NONE
1|0|users|owner_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
1

================================================================================
TABLE: nora_classifier_predictions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|meeting_session_id|TEXT|1||0
2|segment_index|INTEGER|1||0
3|speaker_label|TEXT|1||0
4|utterance|TEXT|1||0
5|context_json|TEXT|1||0
6|predicted_speak|INTEGER|1||0
7|confidence|TEXT|1||0
8|reasoning|TEXT|0||0
9|was_wake_word_addressed|INTEGER|0||0
10|created_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
718

================================================================================
TABLE: nora_voice_config
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|INTEGER|0||1
1|config_json|TEXT|1||0
2|created_at|TIMESTAMP|1|CURRENT_TIMESTAMP|0
3|updated_at|TIMESTAMP|1|CURRENT_TIMESTAMP|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
1

================================================================================
TABLE: notifications
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|user_id|TEXT|1||0
2|organization_id|TEXT|0||0
3|title|TEXT|1||0
4|message|TEXT|1|''|0
5|notification_type|TEXT|1|'info'|0
6|source|TEXT|0||0
7|source_id|TEXT|0||0
8|read_at|TEXT|0||0
9|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: onboarding_segments
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|onboarding_id|BLOB|1||0
3|segment_type|TEXT|1||0
4|name|TEXT|1||0
5|assigned_agent_id|BLOB|0||0
6|assigned_agent_name|TEXT|0||0
7|status|TEXT|1|'pending'|0
8|recommendations|TEXT|0||0
9|user_decisions|TEXT|0||0
10|order_index|INTEGER|1||0
11|started_at|TEXT|0||0
12|completed_at|TEXT|0||0
13|created_at|TEXT|1|datetime('now','subsec')|0
14|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agents|assigned_agent_id|id|NO ACTION|SET NULL|NONE
1|0|project_onboarding|onboarding_id|id|NO ACTION|CASCADE|NONE
2|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: operator_rates
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|organization_id|BLOB|1||0
2|operator_id|BLOB|1||0
3|duration|TEXT|1|'hourly'|0
4|rate_vibe|INTEGER|1|0|0
5|rate_usd|REAL|1|0.0|0
6|notes|TEXT|0||0
7|created_at|TEXT|1|datetime('now','subsec')|0
8|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|operator_id|id|NO ACTION|CASCADE|NONE
1|0|organizations|organization_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: orchestration_contexts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|root_task_id|BLOB|1||0
2|project_id|BLOB|1||0
3|task_id|BLOB|0||0
4|entry_type|TEXT|1||0
5|title|TEXT|1||0
6|content|TEXT|1||0
7|source|TEXT|1||0
8|status|TEXT|1|'active'|0
9|priority|TEXT|1|'normal'|0
10|resolved_by|TEXT|0||0
11|resolved_at|DATETIME|0||0
12|metadata|TEXT|0||0
13|created_at|DATETIME|1|CURRENT_TIMESTAMP|0
14|updated_at|DATETIME|1|CURRENT_TIMESTAMP|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|tasks|root_task_id|id|NO ACTION|NO ACTION|NONE
1|0|projects|project_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
0

================================================================================
TABLE: org_cloud_settings
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|organization_id|TEXT|1||1
1|storage_quota_bytes|INTEGER|1|10737418240|0
2|viewer_can_download|INTEGER|1|1|0
3|member_can_upload|INTEGER|1|1|0
4|auto_index_data_sources|INTEGER|1|1|0
5|auto_index_artifacts|INTEGER|1|1|0
6|auto_index_media|INTEGER|1|1|0
7|nats_contribution_enabled|INTEGER|1|0|0
8|updated_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: org_invitations
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|organization_id|BLOB|1||0
2|invite_code|TEXT|1||0
3|role|TEXT|1|'member'|0
4|created_by|BLOB|1||0
5|created_at|TEXT|1|datetime('now', 'subsec')|0
6|expires_at|TEXT|1||0
7|accepted_by|BLOB|0||0
8|accepted_at|TEXT|0||0
9|max_uses|INTEGER|1|1|0
10|use_count|INTEGER|1|0|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|accepted_by|id|NO ACTION|NO ACTION|NONE
1|0|users|created_by|id|NO ACTION|NO ACTION|NONE
2|0|organizations|organization_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: org_onboarding
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|organization_id|BLOB|1||0
2|status|TEXT|1|'active'|0
3|current_phase|TEXT|1|'context_gathering'|0
4|context_data|TEXT|0||0
5|recommendations|TEXT|0||0
6|started_at|TEXT|1|datetime('now','subsec')|0
7|completed_at|TEXT|0||0
8|created_at|TEXT|1|datetime('now','subsec')|0
9|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|organizations|organization_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: org_onboarding_segments
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|organization_id|BLOB|1||0
2|onboarding_id|BLOB|1||0
3|segment_type|TEXT|1||0
4|name|TEXT|1||0
5|assigned_agent_id|BLOB|0||0
6|assigned_agent_name|TEXT|0||0
7|status|TEXT|1|'pending'|0
8|recommendations|TEXT|0||0
9|user_decisions|TEXT|0||0
10|order_index|INTEGER|1||0
11|started_at|TEXT|0||0
12|completed_at|TEXT|0||0
13|created_at|TEXT|1|datetime('now','subsec')|0
14|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|org_onboarding|onboarding_id|id|NO ACTION|CASCADE|NONE
1|0|organizations|organization_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: organization_brand_profiles
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|organization_id|BLOB|1||0
2|tagline|TEXT|0||0
3|primary_color|TEXT|1|'#2563EB'|0
4|secondary_color|TEXT|1|'#EC4899'|0
5|accent_color|TEXT|0||0
6|typography_heading|TEXT|0||0
7|typography_body|TEXT|0||0
8|logo_url|TEXT|0||0
9|industry|TEXT|0||0
10|market_position|TEXT|0||0
11|unique_value_proposition|TEXT|0||0
12|mission_statement|TEXT|0||0
13|vision_statement|TEXT|0||0
14|brand_values|TEXT|0||0
15|brand_voice|TEXT|0||0
16|brand_archetype|TEXT|0||0
17|target_audience|TEXT|0||0
18|icp_description|TEXT|0||0
19|icp_company_size|TEXT|0||0
20|icp_industries|TEXT|0||0
21|competitor_brands|TEXT|0||0
22|differentiators|TEXT|0||0
23|content_pillars|TEXT|0||0
24|content_tone|TEXT|0||0
25|website_url|TEXT|0||0
26|social_instagram|TEXT|0||0
27|social_twitter|TEXT|0||0
28|social_linkedin|TEXT|0||0
29|social_facebook|TEXT|0||0
30|social_youtube|TEXT|0||0
31|social_tiktok|TEXT|0||0
32|created_at|TEXT|1|datetime('now', 'subsec')|0
33|updated_at|TEXT|1|datetime('now', 'subsec')|0
34|research_status|TEXT|1|'idle'|0
35|research_ran_at|TEXT|0||0
36|research_summary|TEXT|0||0
37|mood_board_urls|TEXT|0||0
38|clearbit_logo_url|TEXT|0||0
39|brand_photography_notes|TEXT|0||0
40|research_iterations|INTEGER|1|0|0
41|research_depth|INTEGER|1|0|0
42|founder_name|TEXT|0||0
43|founding_year|TEXT|0||0
44|key_clients|TEXT|0||0
45|estimated_team_size|TEXT|0||0
46|tech_stack|TEXT|0||0
47|geographic_focus|TEXT|0||0
48|funding_stage|TEXT|0||0
49|content_strategy_notes|TEXT|0||0
50|awards_and_recognition|TEXT|0||0
51|brand_gap_notes|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|organizations|organization_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
2

================================================================================
TABLE: organization_members
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|organization_id|TEXT|1||0
2|user_id|BLOB|1||0
3|role|TEXT|1|'member'|0
4|joined_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|user_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
12

================================================================================
TABLE: organizations
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|name|TEXT|1||0
2|slug|TEXT|1||0
3|description|TEXT|0||0
4|avatar_url|TEXT|0||0
5|owner_id|TEXT|1||0
6|settings|TEXT|0|'{}'|0
7|is_active|INTEGER|1|1|0
8|created_at|TEXT|1|datetime('now')|0
9|updated_at|TEXT|1|datetime('now')|0
10|deleted_at|TEXT|0||0
11|deleted_by|TEXT|0||0
12|company_id|TEXT|0||0
13|invite_token|TEXT|0||0
14|pending_owner_email|TEXT|0||0
15|created_by_org_id|TEXT|0||0
16|address|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
10

================================================================================
TABLE: oss_libraries
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|name|TEXT|1||0
2|github_owner|TEXT|1||0
3|github_repo|TEXT|1||0
4|tracked_version|TEXT|0||0
5|latest_version|TEXT|0||0
6|last_checked_at|TEXT|0||0
7|check_interval_secs|INTEGER|1|3600|0
8|is_active|INTEGER|1|1|0
9|notes|TEXT|0||0
10|created_at|TEXT|1|datetime('now','subsec')|0
11|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
5

================================================================================
TABLE: oss_library_updates
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|library_id|BLOB|1||0
2|version|TEXT|1||0
3|release_url|TEXT|0||0
4|release_notes|TEXT|0||0
5|significance|TEXT|1|'patch'|0
6|agent_recommendation|TEXT|0||0
7|recommendation_status|TEXT|1|'pending'|0
8|agent_flow_id|BLOB|0||0
9|published_at|TEXT|0||0
10|created_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|oss_libraries|library_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
5

================================================================================
TABLE: pcg_router_models
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|name|TEXT|1||0
2|model_id|TEXT|1||0
3|provider|TEXT|1||0
4|provider_base_url|TEXT|0||0
5|api_key_env_var|TEXT|0||0
6|priority|INTEGER|1|10|0
7|context_window|INTEGER|0||0
8|max_output_tokens|INTEGER|0||0
9|supports_tools|INTEGER|1|1|0
10|supports_vision|INTEGER|1|0|0
11|cost_per_million_input|INTEGER|1|0|0
12|cost_per_million_output|INTEGER|1|0|0
13|is_enabled|INTEGER|1|1|0
14|created_at|TEXT|1|datetime('now','subsec')|0
15|updated_at|TEXT|1|datetime('now','subsec')|0
16|api_key_value|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
31

================================================================================
TABLE: peer_contributions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|peer_node_id|BLOB|1||0
2|period_start|TEXT|1||0
3|period_end|TEXT|1||0
4|uptime_seconds|INTEGER|1|0|0
5|cpu_units|INTEGER|1|0|0
6|gpu_units|INTEGER|1|0|0
7|bandwidth_bytes|INTEGER|1|0|0
8|storage_bytes|INTEGER|1|0|0
9|relay_messages|INTEGER|1|0|0
10|tasks_completed|INTEGER|1|0|0
11|tasks_failed|INTEGER|1|0|0
12|heartbeat_count|INTEGER|1|0|0
13|contribution_score|INTEGER|1|0|0
14|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|peer_nodes|peer_node_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
0

================================================================================
TABLE: peer_nodes
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|node_id|TEXT|1||0
2|peer_id|TEXT|0||0
3|wallet_address|TEXT|1||0
4|capabilities|TEXT|0||0
5|cpu_cores|INTEGER|0||0
6|ram_mb|INTEGER|0||0
7|storage_gb|INTEGER|0||0
8|gpu_available|BOOLEAN|0|0|0
9|gpu_model|TEXT|0||0
10|first_seen_at|TEXT|1|datetime('now', 'subsec')|0
11|last_heartbeat_at|TEXT|0||0
12|is_active|BOOLEAN|0|1|0
13|is_banned|BOOLEAN|0|0|0
14|ban_reason|TEXT|0||0
15|created_at|TEXT|1|datetime('now', 'subsec')|0
16|updated_at|TEXT|1|datetime('now', 'subsec')|0
17|hostname|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
12

================================================================================
TABLE: peer_rewards
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|peer_node_id|BLOB|1||0
2|contribution_id|BLOB|0||0
3|reward_type|TEXT|1||0
4|base_amount|INTEGER|1||0
5|multiplier|REAL|1|1.0|0
6|final_amount|INTEGER|1||0
7|status|TEXT|1|'pending'|0
8|batch_id|BLOB|0||0
9|aptos_tx_hash|TEXT|0||0
10|block_height|INTEGER|0||0
11|error_message|TEXT|0||0
12|retry_count|INTEGER|0|0|0
13|description|TEXT|0||0
14|metadata|TEXT|0||0
15|created_at|TEXT|1|datetime('now', 'subsec')|0
16|updated_at|TEXT|1|datetime('now', 'subsec')|0
17|distributed_at|TEXT|0||0
18|confirmed_at|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|reward_batches|batch_id|id|NO ACTION|NO ACTION|NONE
1|0|peer_contributions|contribution_id|id|NO ACTION|NO ACTION|NONE
2|0|peer_nodes|peer_node_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
28538

================================================================================
TABLE: peer_wallet_balances
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|peer_node_id|BLOB|0||1
1|pending_rewards|INTEGER|1|0|0
2|distributed_rewards|INTEGER|1|0|0
3|confirmed_rewards|INTEGER|1|0|0
4|onchain_balance|INTEGER|0||0
5|onchain_last_checked|TEXT|0||0
6|total_earned_lifetime|INTEGER|1|0|0
7|total_withdrawn|INTEGER|1|0|0
8|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|peer_nodes|peer_node_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
0

================================================================================
TABLE: pending_gates
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|approval_gate_id|BLOB|1||0
2|execution_process_id|BLOB|1||0
3|status|TEXT|1|'pending'|0
4|trigger_context|TEXT|0||0
5|approval_count|INTEGER|1|0|0
6|rejection_count|INTEGER|1|0|0
7|resolved_at|TEXT|0||0
8|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|execution_processes|execution_process_id|id|NO ACTION|CASCADE|NONE
1|0|approval_gates|approval_gate_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: permission_audit_log
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|user_id|BLOB|1||0
2|action|TEXT|1||0
3|resource_type|TEXT|1||0
4|resource_id|TEXT|0||0
5|details|TEXT|0|'{}'|0
6|performed_by|BLOB|1||0
7|performed_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|performed_by|id|NO ACTION|CASCADE|NONE
1|0|users|user_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: person_company_roles
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0|randomblob(16)|1
1|person_id|BLOB|1||0
2|company_id|BLOB|1||0
3|role|TEXT|1|'contact'|0
4|title|TEXT|0||0
5|is_primary|INTEGER|1|0|0
6|start_date|TEXT|0||0
7|end_date|TEXT|0||0
8|notes|TEXT|0||0
9|created_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|companies|company_id|id|NO ACTION|CASCADE|NONE
1|0|persons|person_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
37

================================================================================
TABLE: person_notes
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|person_id|TEXT|1||0
2|author_id|TEXT|0||0
3|text|TEXT|1||0
4|status|TEXT|1|'open'|0
5|attachments|TEXT|1|'[]'|0
6|proposal_id|TEXT|0||0
7|created_at|TEXT|1|datetime('now','subsec')|0
8|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|author_id|id|NO ACTION|SET NULL|NONE
1|0|persons|person_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
3

================================================================================
TABLE: person_organization_contacts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0|randomblob(16)|1
1|person_id|BLOB|1||0
2|organization_id|BLOB|1||0
3|context|TEXT|1|'contact'|0
4|notes|TEXT|0||0
5|added_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|organizations|organization_id|id|NO ACTION|CASCADE|NONE
1|0|persons|person_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
33

================================================================================
TABLE: person_research_passes
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|person_id|TEXT|1||0
2|pass_number|INTEGER|1|1|0
3|research_focus|TEXT|1||0
4|focus_prompt|TEXT|0||0
5|status|TEXT|1|'pending'|0
6|summary|TEXT|0||0
7|raw_results|TEXT|0||0
8|key_findings|TEXT|0|'[]'|0
9|search_queries|TEXT|0|'[]'|0
10|confidence_delta|REAL|0|0.0|0
11|agent_used|TEXT|0||0
12|tokens_used|INTEGER|0||0
13|error|TEXT|0||0
14|created_at|TEXT|1|datetime('now','subsec')|0
15|completed_at|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|persons|person_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: person_social_profiles
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|person_id|TEXT|1||0
2|platform|TEXT|1||0
3|handle|TEXT|0||0
4|profile_url|TEXT|0||0
5|follower_count|INTEGER|0||0
6|following_count|INTEGER|0||0
7|bio|TEXT|0||0
8|verified|INTEGER|1|0|0
9|raw_data|TEXT|0||0
10|last_synced_at|TEXT|0||0
11|created_at|TEXT|1|datetime('now', 'subsec')|0
12|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|persons|person_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
1

================================================================================
TABLE: persons
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|full_name|TEXT|1||0
2|email|TEXT|0||0
3|phone|TEXT|0||0
4|avatar_url|TEXT|0||0
5|person_type|TEXT|1|'contact'|0
6|financial_role|TEXT|1|'neutral'|0
7|client_profile|TEXT|0||0
8|business_stage|TEXT|0||0
9|lifecycle_stage|TEXT|1|'lead'|0
10|lead_score|INTEGER|1|0|0
11|company_name|TEXT|0||0
12|job_title|TEXT|0||0
13|website|TEXT|0||0
14|user_id|TEXT|0||0
15|crm_contact_id|TEXT|0||0
16|organization_id|TEXT|0||0
17|intelligence_summary|TEXT|0||0
18|intelligence_raw|TEXT|0||0
19|intelligence_last_run_at|TEXT|0||0
20|intelligence_confidence|REAL|1|0.0|0
21|notes|TEXT|0||0
22|tags|TEXT|1|'[]'|0
23|custom_fields|TEXT|1|'{}'|0
24|created_at|TEXT|1|datetime('now', 'subsec')|0
25|updated_at|TEXT|1|datetime('now', 'subsec')|0
26|intelligence_status|TEXT|1|'idle'|0
27|intelligence_agent|TEXT|0||0
28|emails|TEXT|1|'[]'|0
29|phones|TEXT|1|'[]'|0
30|assigned_to|TEXT|0||0
31|company_org_id|TEXT|0||0
32|company_id|TEXT|0||0
33|onboarding_channel|TEXT|0||0
34|preferred_contact|TEXT|0||0
35|research_pass_count|INTEGER|1|0|0
36|research_depth|TEXT|1|'shallow'|0
37|vibe|TEXT|0||0
38|online_presence_quality|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
66

================================================================================
TABLE: project_assets
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|pod_id|BLOB|0||0
3|category|TEXT|1|'file'|0
4|scope|TEXT|1|'team'|0
5|name|TEXT|1||0
6|storage_path|TEXT|1||0
7|checksum|TEXT|0||0
8|byte_size|INTEGER|0|0|0
9|mime_type|TEXT|0|''|0
10|metadata|TEXT|0|''|0
11|uploaded_by|TEXT|0|''|0
12|created_at|TEXT|1|datetime('now', 'subsec')|0
13|updated_at|TEXT|1|datetime('now', 'subsec')|0
14|board_id|BLOB|0||0
15|created_by_agent_id|BLOB|0||0
16|source_task_id|BLOB|0||0
17|workflow_phase|TEXT|0||0
18|version|INTEGER|0|1|0
19|parent_asset_id|BLOB|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|project_pods|pod_id|id|NO ACTION|SET NULL|NONE
1|0|projects|project_id|id|NO ACTION|CASCADE|NONE
2|0|project_assets|parent_asset_id|id|NO ACTION|SET NULL|NONE
3|0|tasks|source_task_id|id|NO ACTION|SET NULL|NONE
4|0|agents|created_by_agent_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
0

================================================================================
TABLE: project_boards
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|project_id|TEXT|1||0
2|name|TEXT|1||0
3|slug|TEXT|1||0
4|board_type|TEXT|1||0
5|description|TEXT|0||0
6|metadata|TEXT|0||0
7|created_at|TEXT|1|datetime('now', 'subsec')|0
8|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
82

================================================================================
TABLE: project_brand_profiles
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|tagline|TEXT|0||0
3|industry|TEXT|0||0
4|primary_color|TEXT|0|'#2563EB'|0
5|secondary_color|TEXT|0|'#EC4899'|0
6|brand_voice|TEXT|0||0
7|target_audience|TEXT|0||0
8|logo_asset_id|BLOB|0||0
9|guidelines_asset_id|BLOB|0||0
10|created_at|TEXT|1|datetime('now', 'subsec')|0
11|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|project_assets|guidelines_asset_id|id|NO ACTION|SET NULL|NONE
1|0|project_assets|logo_asset_id|id|NO ACTION|SET NULL|NONE
2|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: project_collaborators
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|project_id|BLOB|1||0
2|user_id|BLOB|1||0
3|role|TEXT|1|'viewer'|0
4|can_read|BOOLEAN|0|TRUE|0
5|can_write|BOOLEAN|0|FALSE|0
6|can_delete|BOOLEAN|0|FALSE|0
7|can_invite|BOOLEAN|0|FALSE|0
8|can_manage|BOOLEAN|0|FALSE|0
9|invited_by|BLOB|0||0
10|invited_at|TIMESTAMP|0||0
11|accepted_at|TIMESTAMP|0||0
12|status|TEXT|0|'active'|0
13|created_at|TIMESTAMP|1|datetime('now')|0
14|updated_at|TIMESTAMP|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|invited_by|id|NO ACTION|NO ACTION|NONE
1|0|users|user_id|id|NO ACTION|CASCADE|NONE
2|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: project_controller_config
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|name|TEXT|1|'Controller'|0
3|personality|TEXT|1|'professional'|0
4|system_prompt|TEXT|0||0
5|voice_id|TEXT|0||0
6|avatar_url|TEXT|0||0
7|model|TEXT|0|'gpt-4o-mini'|0
8|temperature|REAL|0|0.7|0
9|max_tokens|INTEGER|0|2048|0
10|created_at|TEXT|1|datetime('now')|0
11|updated_at|TEXT|1|datetime('now')|0
12|enabled_tools|TEXT|0|'[]'|0
13|topology_auto_refresh|INTEGER|0|1|0
14|autonomy_level|TEXT|0|'supervised'|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: project_controller_conversations
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|user_id|TEXT|1||0
3|title|TEXT|0||0
4|created_at|TEXT|1|datetime('now')|0
5|updated_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|user_id|id|NO ACTION|CASCADE|NONE
1|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: project_controller_messages
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|conversation_id|TEXT|1||0
2|role|TEXT|1||0
3|content|TEXT|1||0
4|tokens_used|INTEGER|0||0
5|created_at|TEXT|1|datetime('now')|0
6|tool_call_id|TEXT|0||0
7|tool_name|TEXT|0||0
8|tool_arguments|TEXT|0||0
9|tool_result|TEXT|0||0
10|topology_context|TEXT|0||0
11|route_taken|TEXT|0||0
12|input_tokens|INTEGER|0||0
13|output_tokens|INTEGER|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|project_controller_conversations|conversation_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: project_folders
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|organization_id|BLOB|1||0
2|client_id|BLOB|0||0
3|name|TEXT|1||0
4|sort_order|INTEGER|1|0|0
5|is_active|INTEGER|1|1|0
6|created_at|TEXT|1|datetime('now')|0
7|updated_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|clients|client_id|id|NO ACTION|SET NULL|NONE
1|0|organizations|organization_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
8

================================================================================
TABLE: project_knowledge_sources
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|0||0
2|source_type|TEXT|1||0
3|source_id|TEXT|1||0
4|source_title|TEXT|1||0
5|source_summary|TEXT|0||0
6|coverage_score|REAL|1|0.0|0
7|is_active|INTEGER|1|1|0
8|is_stale|INTEGER|1|0|0
9|auto_registered|INTEGER|1|0|0
10|last_refreshed_at|TEXT|1|datetime('now','subsec')|0
11|created_at|TEXT|1|datetime('now','subsec')|0
12|updated_at|TEXT|1|datetime('now','subsec')|0
13|owner_type|TEXT|1|'project'|0
14|owner_id|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
248

================================================================================
TABLE: project_members
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|user_id|BLOB|1||0
3|role|TEXT|1|'viewer'|0
4|permissions|TEXT|1|'{}'|0
5|granted_by|BLOB|0||0
6|granted_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|granted_by|id|NO ACTION|SET NULL|NONE
1|0|users|user_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
116

================================================================================
TABLE: project_onboarding
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|status|TEXT|1|'active'|0
3|current_phase|TEXT|1|'context_gathering'|0
4|context_data|TEXT|0||0
5|recommendations|TEXT|0||0
6|started_at|TEXT|1|datetime('now','subsec')|0
7|completed_at|TEXT|0||0
8|created_at|TEXT|1|datetime('now','subsec')|0
9|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: project_pods
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|title|TEXT|1||0
3|description|TEXT|0|''|0
4|status|TEXT|1|'active'|0
5|lead|TEXT|0|''|0
6|created_at|TEXT|1|datetime('now', 'subsec')|0
7|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: project_repos
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|project_id|BLOB|1||0
2|repo_id|BLOB|1||0
3|is_default|INTEGER|1|0|0
4|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|repos|repo_id|id|NO ACTION|CASCADE|NONE
1|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
54

================================================================================
TABLE: projects
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|name|TEXT|1||0
2|git_repo_path|TEXT|1|''|0
3|setup_script|TEXT|0|''|0
4|created_at|TEXT|1|datetime('now', 'subsec')|0
5|updated_at|TEXT|1|datetime('now', 'subsec')|0
6|dev_script|TEXT|0|''|0
7|cleanup_script|TEXT|0||0
8|copy_files|TEXT|0||0
9|max_concurrent_agents|INTEGER|0|3|0
10|max_concurrent_browser_agents|INTEGER|0|1|0
11|vibe_budget_limit|INTEGER|0||0
12|vibe_spent_amount|INTEGER|1|0|0
13|owner_id|TEXT|0||0
14|deleted_at|TEXT|0||0
15|deleted_by|TEXT|0||0
16|aptos_address|TEXT|0||0
17|aptos_private_key_encrypted|TEXT|0||0
18|aptos_funded|INTEGER|1|0|0
19|total_vibe_deposited|INTEGER|1|0|0
20|total_vibe_withdrawn|INTEGER|1|0|0
21|organization_id|TEXT|0||0
22|client_id|TEXT|0||0
23|folder_id|TEXT|0||0
24|parent_project_id|TEXT|0||0
25|sort_order|INTEGER|1|0|0
26|primary_device_id|TEXT|0||0
27|is_shared|BOOLEAN|0|FALSE|0
28|collaboration_enabled|BOOLEAN|0|FALSE|0
29|slug|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
78

================================================================================
TABLE: projects_backup_20260215
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id||0||0
1|name|TEXT|0||0
2|git_repo_path|TEXT|0||0
3|setup_script|TEXT|0||0
4|created_at|TEXT|0||0
5|updated_at|TEXT|0||0
6|dev_script|TEXT|0||0
7|cleanup_script|TEXT|0||0
8|copy_files|TEXT|0||0
9|max_concurrent_agents|INT|0||0
10|max_concurrent_browser_agents|INT|0||0
11|vibe_budget_limit|INT|0||0
12|vibe_spent_amount|INT|0||0
13|owner_id||0||0
14|deleted_at|TEXT|0||0
15|deleted_by||0||0
16|aptos_address|TEXT|0||0
17|aptos_private_key_encrypted|TEXT|0||0
18|aptos_funded|INT|0||0
19|total_vibe_deposited|INT|0||0
20|total_vibe_withdrawn|INT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
33

================================================================================
TABLE: proposals
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|lead_id|BLOB|0||0
2|organization_id|BLOB|0||0
3|owner_id|BLOB|0||0
4|project_id|BLOB|0||0
5|status|TEXT|1|'drafted'|0
6|title|TEXT|1|''|0
7|description|TEXT|1|''|0
8|quote_amount_vibe|INTEGER|1|0|0
9|deal_type|TEXT|1|'one-off'|0
10|sent_at|TEXT|0||0
11|seen_at|TEXT|0||0
12|verbal_at|TEXT|0||0
13|signed_at|TEXT|0||0
14|declined_at|TEXT|0||0
15|created_at|TEXT|1|datetime('now','subsec')|0
16|updated_at|TEXT|1|datetime('now','subsec')|0
17|contact_ids|TEXT|1|'[]'|0
18|company_id|BLOB|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|companies|company_id|id|NO ACTION|NO ACTION|NONE
1|0|projects|project_id|id|NO ACTION|SET NULL|NONE
2|0|users|owner_id|id|NO ACTION|SET NULL|NONE
3|0|organizations|organization_id|id|NO ACTION|SET NULL|NONE
4|0|persons|lead_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
37

================================================================================
TABLE: pulse_alert_rules
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|BLOB|1||0
2|organization_id|BLOB|0||0
3|name|TEXT|1|''|0
4|conditions|TEXT|1|'[]'|0
5|actions|TEXT|1|'[]'|0
6|priority|TEXT|1|'normal'|0
7|enabled|INTEGER|1|1|0
8|trigger_count|INTEGER|1|0|0
9|last_triggered_at|TEXT|0||0
10|created_at|TEXT|1|datetime('now', 'subsec')|0
11|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: pulse_alerts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|BLOB|1||0
2|organization_id|BLOB|0||0
3|rule_id|TEXT|1||0
4|content_item_id|TEXT|1||0
5|priority|TEXT|1|'normal'|0
6|acknowledged|INTEGER|1|0|0
7|auto_task_id|BLOB|0||0
8|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|tasks|auto_task_id|id|NO ACTION|SET NULL|NONE
1|0|pulse_content_items|content_item_id|id|NO ACTION|CASCADE|NONE
2|0|pulse_alert_rules|rule_id|id|NO ACTION|CASCADE|NONE
3|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: pulse_collection_runs
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|BLOB|1||0
2|organization_id|BLOB|0||0
3|source_id|TEXT|1||0
4|started_at|TEXT|1|datetime('now', 'subsec')|0
5|completed_at|TEXT|0||0
6|items_collected|INTEGER|1|0|0
7|items_new|INTEGER|1|0|0
8|items_duplicate|INTEGER|1|0|0
9|status|TEXT|1|'running'|0
10|error|TEXT|0||0
11|vibe_cost|REAL|0|0|0
12|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: pulse_content_items
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|BLOB|1||0
2|organization_id|BLOB|0||0
3|source_id|TEXT|1||0
4|source_type|TEXT|1||0
5|content_hash|TEXT|1||0
6|url|TEXT|1|''|0
7|title|TEXT|1|''|0
8|body|TEXT|0||0
9|summary|TEXT|0||0
10|author|TEXT|0||0
11|published_at|TEXT|0||0
12|collected_at|TEXT|1|datetime('now', 'subsec')|0
13|relevance_score|REAL|0||0
14|extracted_entities|TEXT|0||0
15|pcg_status|TEXT|1|'new'|0
16|linked_task_id|BLOB|0||0
17|linked_entity_id|TEXT|0||0
18|linked_crm_contact_id|BLOB|0||0
19|enrichment_vibe_cost|REAL|0|0|0
20|created_at|TEXT|1|datetime('now', 'subsec')|0
21|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|crm_contacts|linked_crm_contact_id|id|NO ACTION|SET NULL|NONE
1|0|tasks|linked_task_id|id|NO ACTION|SET NULL|NONE
2|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: pulse_sources
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|BLOB|1||0
2|organization_id|BLOB|0||0
3|source_id|TEXT|1||0
4|source_type|TEXT|1||0
5|name|TEXT|1|''|0
6|url|TEXT|1|''|0
7|config|TEXT|0||0
8|enabled|INTEGER|1|1|0
9|status|TEXT|1|'active'|0
10|last_fetch_at|TEXT|0||0
11|last_error|TEXT|0||0
12|collection_interval_secs|INTEGER|1|300|0
13|category|TEXT|0||0
14|created_at|TEXT|1|datetime('now', 'subsec')|0
15|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: pulse_tracking_configs
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|BLOB|1||0
2|organization_id|BLOB|0||0
3|keywords|TEXT|1|'[]'|0
4|entities|TEXT|1|'{}'|0
5|llm_enabled|INTEGER|1|0|0
6|llm_model|TEXT|0|'llama3.2'|0
7|notification_config|TEXT|0|'{}'|0
8|created_at|TEXT|1|datetime('now', 'subsec')|0
9|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: quickbooks_accounts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|organization_id|TEXT|1||0
2|realm_id|TEXT|1||0
3|company_name|TEXT|0||0
4|access_token|TEXT|0||0
5|refresh_token|TEXT|0||0
6|token_expires_at|TEXT|0||0
7|environment|TEXT|1|'sandbox'|0
8|sync_enabled|INTEGER|1|1|0
9|sync_frequency_minutes|INTEGER|1|60|0
10|last_sync_at|TEXT|0||0
11|sync_invoices|INTEGER|1|1|0
12|sync_customers|INTEGER|1|1|0
13|sync_payments|INTEGER|1|1|0
14|sync_expenses|INTEGER|1|1|0
15|sync_time_tracking|INTEGER|1|0|0
16|status|TEXT|1|'active'|0
17|last_error|TEXT|0||0
18|metadata|TEXT|0||0
19|connected_by|TEXT|0||0
20|created_at|TEXT|1|datetime('now', 'subsec')|0
21|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: quickbooks_entity_map
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|quickbooks_account_id|TEXT|1||0
2|pcg_entity_type|TEXT|1||0
3|pcg_entity_id|TEXT|1||0
4|qbo_entity_type|TEXT|1||0
5|qbo_entity_id|TEXT|1||0
6|qbo_sync_token|TEXT|0||0
7|last_synced_at|TEXT|0||0
8|sync_direction|TEXT|1|'bidirectional'|0
9|sync_status|TEXT|1|'synced'|0
10|last_error|TEXT|0||0
11|created_at|TEXT|1|datetime('now', 'subsec')|0
12|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|quickbooks_accounts|quickbooks_account_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: ralph_iterations
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|ralph_loop_id|TEXT|1||0
2|execution_process_id|TEXT|0||0
3|iteration_number|INTEGER|1||0
4|status|TEXT|1|'running'|0
5|completion_signal_found|BOOLEAN|0|FALSE|0
6|exit_signal_found|BOOLEAN|0|FALSE|0
7|backpressure_results|TEXT|0|'{}'|0
8|all_backpressure_passed|BOOLEAN|0||0
9|tokens_used|INTEGER|0|0|0
10|cost_cents|INTEGER|0|0|0
11|duration_ms|INTEGER|0||0
12|started_at|TEXT|1|datetime('now', 'subsec')|0
13|completed_at|TEXT|0||0
14|output_summary|TEXT|0||0
15|files_modified|INTEGER|0|0|0
16|commits_made|INTEGER|0|0|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|execution_processes|execution_process_id|id|NO ACTION|NO ACTION|NONE
1|0|ralph_loop_state|ralph_loop_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: ralph_loop_state
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|task_attempt_id|TEXT|1||0
2|agent_id|TEXT|0||0
3|current_iteration|INTEGER|1|0|0
4|max_iterations|INTEGER|1|50|0
5|session_id|TEXT|0||0
6|status|TEXT|1|'initializing'|0
7|completion_promise|TEXT|0||0
8|completion_detected_at|TEXT|0||0
9|final_validation_passed|BOOLEAN|0||0
10|total_tokens_used|INTEGER|0|0|0
11|total_cost_cents|INTEGER|0|0|0
12|started_at|TEXT|1|datetime('now', 'subsec')|0
13|completed_at|TEXT|0||0
14|last_iteration_at|TEXT|0||0
15|last_error|TEXT|0||0
16|consecutive_failures|INTEGER|0|0|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agents|agent_id|id|NO ACTION|NO ACTION|NONE
1|0|task_attempts|task_attempt_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: render_jobs
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|edit_session_id|TEXT|1||0
2|destinations|TEXT|1|'[]'|0
3|formats|TEXT|1|'[]'|0
4|priority|TEXT|1||0
5|status|TEXT|1||0
6|progress_percent|REAL|0||0
7|last_error|TEXT|0||0
8|output_urls|TEXT|1|'[]'|0
9|metadata|TEXT|1|'{}'|0
10|created_at|TEXT|1||0
11|updated_at|TEXT|1||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|edit_sessions|edit_session_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: repos
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|path|TEXT|1||0
2|name|TEXT|1||0
3|display_name|TEXT|1||0
4|setup_script|TEXT|0||0
5|cleanup_script|TEXT|0||0
6|archive_script|TEXT|0||0
7|copy_files|TEXT|0||0
8|parallel_setup_script|INTEGER|1|0|0
9|dev_server_script|TEXT|0||0
10|default_target_branch|TEXT|0||0
11|default_working_dir|TEXT|0||0
12|created_at|TEXT|1|datetime('now', 'subsec')|0
13|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
54

================================================================================
TABLE: research_stage_results
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|workflow_id|BLOB|1||0
2|stage_name|TEXT|1||0
3|stage_order|INTEGER|1||0
4|started_at|TEXT|1||0
5|completed_at|TEXT|0||0
6|execution_time_ms|INTEGER|0||0
7|status|TEXT|1|'pending'|0
8|results_summary|TEXT|0||0
9|artifact_ids|TEXT|0||0
10|items_discovered|INTEGER|0|0|0
11|items_processed|INTEGER|0|0|0
12|items_failed|INTEGER|0|0|0
13|qa_run_id|BLOB|0||0
14|qa_passed|INTEGER|0||0
15|error_message|TEXT|0||0
16|retry_count|INTEGER|0|0|0
17|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|workflow_qa_runs|qa_run_id|id|NO ACTION|NO ACTION|NONE
1|0|conference_workflows|workflow_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: review_comments
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|deliverable_id|BLOB|1||0
2|token_id|BLOB|0||0
3|timecode_seconds|REAL|0||0
4|author_name|TEXT|0|'Reviewer'|0
5|author_email|TEXT|0||0
6|content|TEXT|1||0
7|is_resolved|INTEGER|0|0|0
8|resolved_by|TEXT|0||0
9|resolved_at|TEXT|0||0
10|created_at|TEXT|0|datetime('now','subsec')|0
11|updated_at|TEXT|0|datetime('now','subsec')|0
12|annotation_data|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
6

================================================================================
TABLE: review_tokens
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|token|TEXT|1||0
2|deliverable_id|BLOB|0||0
3|artifact_id|BLOB|0||0
4|artifact_video_url|TEXT|0||0
5|artifact_title|TEXT|0||0
6|created_by|BLOB|0||0
7|expires_at|TEXT|0||0
8|view_count|INTEGER|1|0|0
9|is_active|INTEGER|1|1|0
10|created_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
10

================================================================================
TABLE: reward_batches
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|batch_number|INTEGER|1||0
2|total_rewards|INTEGER|1||0
3|total_amount|INTEGER|1||0
4|from_wallet|TEXT|1||0
5|aptos_tx_hash|TEXT|0||0
6|block_height|INTEGER|0||0
7|gas_used|INTEGER|0||0
8|status|TEXT|1|'pending'|0
9|error_message|TEXT|0||0
10|retry_count|INTEGER|0|0|0
11|created_at|TEXT|1|datetime('now', 'subsec')|0
12|updated_at|TEXT|1|datetime('now', 'subsec')|0
13|submitted_at|TEXT|0||0
14|confirmed_at|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
1101

================================================================================
TABLE: reward_distribution_log
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|event_type|TEXT|1||0
2|peer_node_id|BLOB|0||0
3|reward_id|BLOB|0||0
4|batch_id|BLOB|0||0
5|amount|INTEGER|0||0
6|description|TEXT|0||0
7|metadata|TEXT|0||0
8|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|reward_batches|batch_id|id|NO ACTION|NO ACTION|NONE
1|0|peer_rewards|reward_id|id|NO ACTION|NO ACTION|NONE
2|0|peer_nodes|peer_node_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
0

================================================================================
TABLE: scheduled_meeting_invitees
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0|randomblob(16)|1
1|scheduled_meeting_id|BLOB|1||0
2|person_id|BLOB|1||0
3|channel|TEXT|1||0
4|channel_address|TEXT|1||0
5|status|TEXT|1|'pending'|0
6|sent_at|TEXT|0||0
7|created_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|persons|person_id|id|NO ACTION|CASCADE|NONE
1|0|scheduled_meetings|scheduled_meeting_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: scheduled_meetings
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0|randomblob(16)|1
1|proposal_id|BLOB|1||0
2|scheduled_at|TEXT|1||0
3|duration_min|INTEGER|1|60|0
4|location|TEXT|0||0
5|agenda|TEXT|0||0
6|channel|TEXT|1||0
7|invite_status|TEXT|1|'pending'|0
8|invite_sent_at|TEXT|0||0
9|notes|TEXT|0||0
10|created_at|TEXT|1|datetime('now','subsec')|0
11|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|proposals|proposal_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: scratch
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|scratch_type|TEXT|1||0
2|payload|TEXT|1|'{}'|0
3|created_at|TEXT|1|datetime('now', 'subsec')|0
4|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: sessions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|user_id|BLOB|1||0
2|token_hash|TEXT|1||0
3|ip_address|TEXT|0||0
4|user_agent|TEXT|0||0
5|expires_at|TEXT|1||0
6|created_at|TEXT|1|datetime('now')|0
7|last_used_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|user_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
55

================================================================================
TABLE: shop_collections
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|name|TEXT|1||0
2|slug|TEXT|1||0
3|description|TEXT|0||0
4|cover_image_url|TEXT|0||0
5|is_active|INTEGER|1|1|0
6|sort_order|INTEGER|1|0|0
7|created_at|TEXT|1|datetime('now','subsec')|0
8|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
4

================================================================================
TABLE: shop_customers
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|email|TEXT|1||0
2|full_name|TEXT|1||0
3|phone|TEXT|0||0
4|address_line1|TEXT|0||0
5|address_line2|TEXT|0||0
6|city|TEXT|0||0
7|state|TEXT|0||0
8|zip|TEXT|0||0
9|country|TEXT|1|'US'|0
10|created_at|TEXT|1|datetime('now','subsec')|0
11|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: shop_newsletter
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|email|TEXT|1||0
2|source|TEXT|0|'footer'|0
3|created_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: shop_order_items
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|order_id|TEXT|1||0
2|product_id|TEXT|1||0
3|product_name|TEXT|1||0
4|size|TEXT|0||0
5|quantity|INTEGER|1|1|0
6|unit_price_cents|INTEGER|1||0
7|total_cents|INTEGER|1||0
8|created_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|shop_products|product_id|id|NO ACTION|NO ACTION|NONE
1|0|shop_orders|order_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: shop_orders
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|customer_id|TEXT|1||0
2|order_number|TEXT|1||0
3|status|TEXT|1|'pending'|0
4|subtotal_cents|INTEGER|1||0
5|shipping_cents|INTEGER|1|0|0
6|tax_cents|INTEGER|1|0|0
7|total_cents|INTEGER|1||0
8|shipping_name|TEXT|0||0
9|shipping_address_line1|TEXT|0||0
10|shipping_address_line2|TEXT|0||0
11|shipping_city|TEXT|0||0
12|shipping_state|TEXT|0||0
13|shipping_zip|TEXT|0||0
14|shipping_country|TEXT|1|'US'|0
15|shipping_method|TEXT|0||0
16|tracking_number|TEXT|0||0
17|notes|TEXT|0||0
18|shipped_at|TEXT|0||0
19|delivered_at|TEXT|0||0
20|created_at|TEXT|1|datetime('now','subsec')|0
21|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|shop_customers|customer_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
0

================================================================================
TABLE: shop_products
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|collection_id|TEXT|0||0
2|name|TEXT|1||0
3|slug|TEXT|1||0
4|description|TEXT|0||0
5|details|TEXT|0||0
6|price_cents|INTEGER|1||0
7|compare_at_cents|INTEGER|0||0
8|images|TEXT|1|'[]'|0
9|sizes|TEXT|1|'[]'|0
10|category|TEXT|0||0
11|tags|TEXT|1|'[]'|0
12|is_active|INTEGER|1|1|0
13|is_featured|INTEGER|1|0|0
14|stock_status|TEXT|1|'in_stock'|0
15|sort_order|INTEGER|1|0|0
16|created_at|TEXT|1|datetime('now','subsec')|0
17|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|shop_collections|collection_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
19

================================================================================
TABLE: shop_settings
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|key|TEXT|1||1
1|value|TEXT|1||0
2|updated_at|TEXT|1|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
6

================================================================================
TABLE: side_events
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|conference_workflow_id|BLOB|1||0
2|platform|TEXT|0||0
3|platform_event_id|TEXT|0||0
4|name|TEXT|1||0
5|description|TEXT|0||0
6|event_date|TEXT|0||0
7|start_time|TEXT|0||0
8|end_time|TEXT|0||0
9|venue_name|TEXT|0||0
10|venue_address|TEXT|0||0
11|latitude|REAL|0||0
12|longitude|REAL|0||0
13|event_url|TEXT|0||0
14|registration_url|TEXT|0||0
15|organizer_name|TEXT|0||0
16|organizer_url|TEXT|0||0
17|relevance_score|REAL|0||0
18|relevance_reason|TEXT|0||0
19|capacity|INTEGER|0||0
20|registered_count|INTEGER|0||0
21|is_featured|INTEGER|0|0|0
22|requires_registration|INTEGER|0|1|0
23|is_free|INTEGER|0|0|0
24|price_info|TEXT|0||0
25|status|TEXT|0|'discovered'|0
26|created_at|TEXT|1|datetime('now', 'subsec')|0
27|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|conference_workflows|conference_workflow_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: sms_messages
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|message_sid|TEXT|1||0
3|account_sid|TEXT|0||0
4|messaging_service_sid|TEXT|0||0
5|from_number|TEXT|1||0
6|to_number|TEXT|1||0
7|body|TEXT|1||0
8|num_segments|INTEGER|0|1|0
9|num_media|INTEGER|0|0|0
10|media_urls|TEXT|0||0
11|direction|TEXT|1||0
12|status|TEXT|1||0
13|error_code|TEXT|0||0
14|error_message|TEXT|0||0
15|handled_by_agent_id|BLOB|0||0
16|conversation_id|BLOB|0||0
17|auto_response|TEXT|0||0
18|sentiment|TEXT|0||0
19|crm_contact_id|BLOB|0||0
20|crm_deal_id|BLOB|0||0
21|is_read|INTEGER|0|0|0
22|is_starred|INTEGER|0|0|0
23|needs_response|INTEGER|0|0|0
24|responded_at|TEXT|0||0
25|price|REAL|0||0
26|price_unit|TEXT|0|'USD'|0
27|date_sent|TEXT|0||0
28|date_created|TEXT|0||0
29|metadata|TEXT|0||0
30|created_at|TEXT|1|datetime('now', 'subsec')|0
31|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|crm_deals|crm_deal_id|id|NO ACTION|SET NULL|NONE
1|0|crm_contacts|crm_contact_id|id|NO ACTION|SET NULL|NONE
2|0|agent_conversations|conversation_id|id|NO ACTION|SET NULL|NONE
3|0|agents|handled_by_agent_id|id|NO ACTION|SET NULL|NONE
4|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: social_accounts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|platform|TEXT|1||0
3|account_type|TEXT|1|'personal'|0
4|platform_account_id|TEXT|1||0
5|username|TEXT|0||0
6|display_name|TEXT|0||0
7|profile_url|TEXT|0||0
8|avatar_url|TEXT|0||0
9|access_token|TEXT|0||0
10|refresh_token|TEXT|0||0
11|token_expires_at|TEXT|0||0
12|follower_count|INTEGER|0||0
13|following_count|INTEGER|0||0
14|post_count|INTEGER|0||0
15|metadata|TEXT|0||0
16|status|TEXT|1|'active'|0
17|last_sync_at|TEXT|0||0
18|last_error|TEXT|0||0
19|created_at|TEXT|1|datetime('now', 'subsec')|0
20|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: social_mentions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|social_account_id|BLOB|1||0
2|project_id|BLOB|1||0
3|mention_type|TEXT|1||0
4|platform|TEXT|1||0
5|platform_mention_id|TEXT|1||0
6|author_username|TEXT|0||0
7|author_display_name|TEXT|0||0
8|author_avatar_url|TEXT|0||0
9|author_follower_count|INTEGER|0||0
10|author_is_verified|INTEGER|0|0|0
11|content|TEXT|0||0
12|media_urls|TEXT|0||0
13|parent_post_id|BLOB|0||0
14|parent_platform_id|TEXT|0||0
15|status|TEXT|1|'unread'|0
16|sentiment|TEXT|0||0
17|priority|TEXT|0|'normal'|0
18|replied_at|TEXT|0||0
19|replied_by|TEXT|0||0
20|reply_content|TEXT|0||0
21|assigned_agent_id|BLOB|0||0
22|auto_response_sent|INTEGER|0|0|0
23|received_at|TEXT|1||0
24|created_at|TEXT|1|datetime('now', 'subsec')|0
25|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agents|assigned_agent_id|id|NO ACTION|SET NULL|NONE
1|0|social_posts|parent_post_id|id|NO ACTION|SET NULL|NONE
2|0|projects|project_id|id|NO ACTION|CASCADE|NONE
3|0|social_accounts|social_account_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: social_posts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|social_account_id|BLOB|0||0
3|task_id|BLOB|0||0
4|content_type|TEXT|1|'post'|0
5|caption|TEXT|0||0
6|content_blocks|TEXT|0||0
7|media_urls|TEXT|0||0
8|hashtags|TEXT|0||0
9|mentions|TEXT|0||0
10|platforms|TEXT|1||0
11|platform_specific|TEXT|0||0
12|status|TEXT|1|'draft'|0
13|scheduled_for|TEXT|0||0
14|published_at|TEXT|0||0
15|category|TEXT|0||0
16|queue_position|INTEGER|0||0
17|is_evergreen|INTEGER|1|0|0
18|recycle_after_days|INTEGER|0||0
19|last_recycled_at|TEXT|0||0
20|created_by_agent_id|BLOB|0||0
21|approved_by|TEXT|0||0
22|approved_at|TEXT|0||0
23|platform_post_id|TEXT|0||0
24|platform_url|TEXT|0||0
25|publish_error|TEXT|0||0
26|impressions|INTEGER|0|0|0
27|reach|INTEGER|0|0|0
28|likes|INTEGER|0|0|0
29|comments|INTEGER|0|0|0
30|shares|INTEGER|0|0|0
31|saves|INTEGER|0|0|0
32|clicks|INTEGER|0|0|0
33|engagement_rate|REAL|0|0.0|0
34|created_at|TEXT|1|datetime('now', 'subsec')|0
35|updated_at|TEXT|1|datetime('now', 'subsec')|0
36|conference_workflow_id|BLOB|0||0
37|entity_id|BLOB|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|entities|entity_id|id|NO ACTION|SET NULL|NONE
1|0|conference_workflows|conference_workflow_id|id|NO ACTION|SET NULL|NONE
2|0|agents|created_by_agent_id|id|NO ACTION|SET NULL|NONE
3|0|tasks|task_id|id|NO ACTION|SET NULL|NONE
4|0|social_accounts|social_account_id|id|NO ACTION|SET NULL|NONE
5|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: social_publish_log
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|post_id|BLOB|1||0
2|account_id|BLOB|1||0
3|attempt_number|INTEGER|0|1|0
4|status|TEXT|1||0
5|platform_post_id|TEXT|0||0
6|platform_url|TEXT|0||0
7|error_message|TEXT|0||0
8|response_body|TEXT|0||0
9|duration_ms|INTEGER|0||0
10|attempted_at|TEXT|0|datetime('now','subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: sqlite_sequence
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|name||0||0
1|seq||0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
1

================================================================================
TABLE: storage_contracts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|client_id|BLOB|1||0
2|provider_device_id|TEXT|1||0
3|storage_size_gb|INTEGER|1||0
4|monthly_rate_vibe|INTEGER|1||0
5|transfer_rate_vibe_per_gb|REAL|1|0.1|0
6|uptime_requirement_percent|INTEGER|1|99|0
7|start_date|TIMESTAMP|1||0
8|end_date|TIMESTAMP|0||0
9|auto_renew|BOOLEAN|0|TRUE|0
10|escrow_amount|INTEGER|1|0|0
11|escrow_address|TEXT|0||0
12|status|TEXT|1|'active'|0
13|actual_storage_used_gb|REAL|0|0|0
14|total_data_transferred_gb|REAL|0|0|0
15|uptime_percent|REAL|0|100|0
16|created_at|TIMESTAMP|1|datetime('now')|0
17|updated_at|TIMESTAMP|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|device_registry|provider_device_id|id|NO ACTION|CASCADE|NONE
1|0|users|client_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: storage_metrics
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|contract_id|TEXT|1||0
2|period_start|TIMESTAMP|1||0
3|period_end|TIMESTAMP|1||0
4|avg_storage_gb|REAL|1||0
5|max_storage_gb|REAL|1||0
6|data_uploaded_gb|REAL|0|0|0
7|data_downloaded_gb|REAL|0|0|0
8|total_checks|INTEGER|1||0
9|successful_checks|INTEGER|1||0
10|uptime_percent|REAL|0||0
11|storage_charge_vibe|INTEGER|0|0|0
12|transfer_charge_vibe|INTEGER|0|0|0
13|uptime_multiplier|REAL|0|1.0|0
14|total_charge_vibe|INTEGER|0|0|0
15|paid|BOOLEAN|0|FALSE|0
16|created_at|TIMESTAMP|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|storage_contracts|contract_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: storage_provider_earnings
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|provider_device_id|TEXT|1||0
2|contract_id|TEXT|1||0
3|period_start|TIMESTAMP|1||0
4|period_end|TIMESTAMP|1||0
5|storage_earnings_vibe|INTEGER|0|0|0
6|transfer_earnings_vibe|INTEGER|0|0|0
7|uptime_bonus_vibe|INTEGER|0|0|0
8|total_earnings_vibe|INTEGER|0|0|0
9|paid|BOOLEAN|0|FALSE|0
10|payment_tx_hash|TEXT|0||0
11|paid_at|TIMESTAMP|0||0
12|created_at|TIMESTAMP|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|storage_contracts|contract_id|id|NO ACTION|CASCADE|NONE
1|0|device_registry|provider_device_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: tags
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|0||0
2|organization_id|BLOB|0||0
3|name|TEXT|1||0
4|color|TEXT|0|'#gray'|0
5|content|TEXT|1|''|0
6|created_at|TEXT|1|datetime('now', 'subsec')|0
7|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|organizations|organization_id|id|NO ACTION|CASCADE|NONE
1|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: task_artifacts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|task_id|BLOB|1||1
1|artifact_id|BLOB|1||2
2|artifact_role|TEXT|1|'supporting'|0
3|display_order|INTEGER|0|0|0
4|pinned|INTEGER|1|0|0
5|added_at|TEXT|1|datetime('now', 'subsec')|0
6|added_by|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|execution_artifacts|artifact_id|id|NO ACTION|CASCADE|NONE
1|0|tasks|task_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
56

================================================================================
TABLE: task_attempts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|task_id|BLOB|1||0
2|executor|TEXT|0||0
3|created_at|TEXT|1|datetime('now', 'subsec')|0
4|updated_at|TEXT|1|datetime('now', 'subsec')|0
5|base_branch|TEXT|1|'main'|0
6|worktree_deleted|BOOLEAN|1|FALSE|0
7|setup_completed_at|DATETIME|0||0
8|container_ref|TEXT|0||0
9|branch|TEXT|0||0
10|deleted_at|TEXT|0||0
11|deleted_by|BLOB|0||0
12|archived|INTEGER|1|0|0
13|pinned|INTEGER|1|0|0
14|name|TEXT|0||0
15|seen_at|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|tasks|task_id|id|NO ACTION|CASCADE|NONE
1|0|users|deleted_by|id|NO ACTION|SET NULL|NONE

--- Row count ---
39

================================================================================
TABLE: task_comments
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|task_id|TEXT|1||0
2|author_id|TEXT|1||0
3|author_type|TEXT|1||0
4|content|TEXT|1||0
5|comment_type|TEXT|1|'comment'|0
6|parent_comment_id|TEXT|0||0
7|mentions|TEXT|0||0
8|metadata|TEXT|0||0
9|created_at|TEXT|1|CURRENT_TIMESTAMP|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|task_comments|parent_comment_id|id|NO ACTION|CASCADE|NONE
1|0|tasks|task_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: task_dependencies
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|source_task_id|BLOB|1||0
3|target_task_id|BLOB|1||0
4|dependency_type|TEXT|1||0
5|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|tasks|target_task_id|id|NO ACTION|CASCADE|NONE
1|0|tasks|source_task_id|id|NO ACTION|CASCADE|NONE
2|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: task_executions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|task_id|BLOB|1||0
2|user_id|BLOB|1||0
3|execution_type|TEXT|1||0
4|executed_on_device_ids|TEXT|0||0
5|cost_vibe|REAL|0|0.0|0
6|vibe_balance_before|REAL|0||0
7|vibe_balance_after|REAL|0||0
8|status|TEXT|1||0
9|started_at|DATETIME|0||0
10|completed_at|DATETIME|0||0
11|duration_seconds|INTEGER|0||0
12|cores_used|INTEGER|0||0
13|ram_used_gb|REAL|0||0
14|error_message|TEXT|0||0
15|created_at|DATETIME|0|CURRENT_TIMESTAMP|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|user_id|id|NO ACTION|CASCADE|NONE
1|0|tasks|task_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
8

================================================================================
TABLE: task_images
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|task_id|BLOB|1||0
2|image_id|BLOB|1||0
3|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|images|image_id|id|NO ACTION|CASCADE|NONE
1|0|tasks|task_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: task_tags
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|task_id|BLOB|1||0
2|tag_id|BLOB|1||0
3|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|tags|tag_id|id|NO ACTION|CASCADE|NONE
1|0|tasks|task_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: task_templates
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|0||0
2|title|TEXT|1||0
3|description|TEXT|0||0
4|template_name|TEXT|1||0
5|created_at|TEXT|1|datetime('now', 'subsec')|0
6|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
3

================================================================================
TABLE: tasks
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|0||1
1|project_id|TEXT|0||0
2|title|TEXT|1||0
3|description|TEXT|0||0
4|status|TEXT|1|'todo'|0
5|created_at|TEXT|1|datetime('now', 'subsec')|0
6|updated_at|TEXT|1|datetime('now', 'subsec')|0
7|parent_task_attempt|TEXT|0||0
8|priority|TEXT|1|'medium'|0
9|assignee_id|TEXT|0||0
10|assigned_agent|TEXT|0||0
11|assigned_mcps|TEXT|0||0
12|created_by|TEXT|1|'system'|0
13|requires_approval|INTEGER|1|0|0
14|approval_status|TEXT|0||0
15|parent_task_id|TEXT|0||0
16|tags|TEXT|0||0
17|due_date|TEXT|0||0
18|pod_id|TEXT|0||0
19|custom_properties|TEXT|0||0
20|scheduled_start|TEXT|0||0
21|scheduled_end|TEXT|0||0
22|board_id|TEXT|0||0
23|collaborators|TEXT|0||0
24|agent_id|TEXT|0||0
25|autonomy_mode|TEXT|0|'agent_assisted'|0
26|onboarding_segment_id|TEXT|0||0
27|workflow_phase|TEXT|0||0
28|approved_by|TEXT|0||0
29|approved_at|TEXT|0||0
30|approval_options|TEXT|0||0
31|approval_selection|TEXT|0||0
32|deleted_at|TEXT|0||0
33|deleted_by|TEXT|0||0
34|created_by_user_id|TEXT|0||0
35|execution_config|TEXT|0||0
36|screenshot|TEXT|0|NULL|0
37|crm_deal_id|TEXT|0||0
38|source_data_source_id|TEXT|0||0
39|source_workflow_run_id|TEXT|0||0
40|assignee_type|TEXT|0||0
41|completion_criteria|TEXT|0||0
42|output_format|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
988

================================================================================
TABLE: tasks_fts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|task_id||0||0
1|title||0||0
2|description||0||0
3|tags||0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
367

================================================================================
TABLE: tasks_fts_config
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|k||1||1
1|v||0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
1

================================================================================
TABLE: tasks_fts_content
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|INTEGER|0||1
1|c0||0||0
2|c1||0||0
3|c2||0||0
4|c3||0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
367

================================================================================
TABLE: tasks_fts_data
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|INTEGER|0||1
1|block|BLOB|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
53

================================================================================
TABLE: tasks_fts_docsize
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|INTEGER|0||1
1|sz|BLOB|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
367

================================================================================
TABLE: tasks_fts_idx
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|segid||1||1
1|term||1||2
2|pgno||0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
51

================================================================================
TABLE: time_entries
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|task_id|BLOB|1||0
3|description|TEXT|0||0
4|start_time|TEXT|1||0
5|end_time|TEXT|0||0
6|duration_seconds|INTEGER|0||0
7|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|tasks|task_id|id|NO ACTION|CASCADE|NONE
1|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: token_usage
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|task_attempt_id|BLOB|0||0
2|agent_id|BLOB|0||0
3|project_id|BLOB|1||0
4|model|TEXT|1||0
5|provider|TEXT|1|'anthropic'|0
6|input_tokens|INTEGER|1|0|0
7|output_tokens|INTEGER|1|0|0
8|total_tokens|INTEGER|1|0|0
9|cost_cents|INTEGER|0||0
10|operation_type|TEXT|0||0
11|metadata|TEXT|0||0
12|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE
1|0|agents|agent_id|id|NO ACTION|SET NULL|NONE
2|0|task_attempts|task_attempt_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
0

================================================================================
TABLE: topiclip_captured_events
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|session_id|TEXT|1||0
2|event_type|TEXT|1||0
3|event_data|TEXT|1||0
4|narrative_role|TEXT|0||0
5|significance_score|REAL|0|0.5|0
6|assigned_symbol|TEXT|0||0
7|symbol_prompt|TEXT|0||0
8|affected_node_ids|TEXT|0||0
9|affected_edge_ids|TEXT|0||0
10|occurred_at|TEXT|1||0
11|created_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|topiclip_sessions|session_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: topiclip_config
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|default_style|TEXT|0|'surreal'|0
3|color_palette|TEXT|0||0
4|visual_density|TEXT|0|'medium'|0
5|motion_intensity|TEXT|0|'moderate'|0
6|llm_model|TEXT|0|'claude-sonnet-4-20250514'|0
7|interpretation_temperature|REAL|0|0.8|0
8|output_resolution|TEXT|0|'768x432'|0
9|output_fps|INTEGER|0|8|0
10|output_format|TEXT|0|'webp'|0
11|significance_algorithm|TEXT|0|'weighted'|0
12|include_event_types|TEXT|0||0
13|exclude_event_types|TEXT|0||0
14|custom_symbol_mappings|TEXT|0||0
15|metadata|TEXT|0||0
16|created_at|TEXT|1|datetime('now')|0
17|updated_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: topiclip_daily_schedule
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|scheduled_time|TEXT|1||0
3|timezone|TEXT|0|'UTC'|0
4|is_enabled|INTEGER|1|1|0
5|current_streak|INTEGER|0|0|0
6|longest_streak|INTEGER|0|0|0
7|total_clips_generated|INTEGER|0|0|0
8|last_generation_date|TEXT|0||0
9|min_significance_threshold|REAL|0|0.3|0
10|force_daily|INTEGER|0|0|0
11|created_at|TEXT|1|datetime('now')|0
12|updated_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: topiclip_sessions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|title|TEXT|1||0
3|day_number|INTEGER|1||0
4|trigger_type|TEXT|1||0
5|primary_theme|TEXT|0||0
6|emotional_arc|TEXT|0||0
7|narrative_summary|TEXT|0||0
8|artistic_prompt|TEXT|0||0
9|negative_prompt|TEXT|0||0
10|symbol_mapping|TEXT|0||0
11|status|TEXT|1|'pending'|0
12|cinematic_brief_id|TEXT|0||0
13|output_asset_ids|TEXT|0||0
14|duration_seconds|INTEGER|0|4|0
15|llm_notes|TEXT|0||0
16|error_message|TEXT|0||0
17|events_analyzed|INTEGER|0|0|0
18|significance_score|REAL|0||0
19|period_start|TEXT|0||0
20|period_end|TEXT|0||0
21|created_at|TEXT|1|datetime('now')|0
22|updated_at|TEXT|1|datetime('now')|0
23|delivered_at|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|cinematic_briefs|cinematic_brief_id|id|NO ACTION|SET NULL|NONE
1|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: topiclip_symbol_library
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|event_pattern|TEXT|1||0
2|symbol_name|TEXT|1||0
3|symbol_description|TEXT|0||0
4|prompt_template|TEXT|1||0
5|negative_template|TEXT|0||0
6|theme_affinity|TEXT|0||0
7|emotional_range|TEXT|0||0
8|suggested_colors|TEXT|0||0
9|motion_type|TEXT|0||0
10|is_default|INTEGER|1|1|0
11|created_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
10

================================================================================
TABLE: topology_clusters
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|name|TEXT|1||0
3|purpose|TEXT|0||0
4|node_ids|TEXT|1||0
5|leader_node_id|TEXT|0||0
6|is_active|INTEGER|1|1|0
7|formed_at|TEXT|1|datetime('now')|0
8|dissolved_at|TEXT|0||0
9|metadata|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|topology_nodes|leader_node_id|id|NO ACTION|SET NULL|NONE
1|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: topology_edges
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|from_node_id|TEXT|1||0
3|to_node_id|TEXT|1||0
4|edge_type|TEXT|1||0
5|weight|REAL|0|1.0|0
6|status|TEXT|1|'active'|0
7|metadata|TEXT|0||0
8|created_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|topology_nodes|to_node_id|id|NO ACTION|CASCADE|NONE
1|0|topology_nodes|from_node_id|id|NO ACTION|CASCADE|NONE
2|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: topology_invariants
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|invariant_type|TEXT|1||0
3|rule|TEXT|1||0
4|severity|TEXT|1|'warning'|0
5|is_active|INTEGER|1|1|0
6|created_at|TEXT|1|datetime('now')|0
7|last_checked_at|TEXT|0||0
8|last_violation_at|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: topology_issues
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|issue_type|TEXT|1||0
3|severity|TEXT|1|'warning'|0
4|affected_nodes|TEXT|0||0
5|affected_edges|TEXT|0||0
6|description|TEXT|1||0
7|suggested_action|TEXT|0||0
8|resolved_at|TEXT|0||0
9|resolution_notes|TEXT|0||0
10|created_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: topology_nodes
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|node_type|TEXT|1||0
3|ref_id|TEXT|1||0
4|capabilities|TEXT|0||0
5|status|TEXT|1|'active'|0
6|metadata|TEXT|0||0
7|weight|REAL|0|1.0|0
8|created_at|TEXT|1|datetime('now')|0
9|updated_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: topology_routes
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|goal|TEXT|1||0
3|path|TEXT|1||0
4|edges|TEXT|1||0
5|total_weight|REAL|0||0
6|status|TEXT|1|'planned'|0
7|started_at|TEXT|0||0
8|completed_at|TEXT|0||0
9|rerouted_from|TEXT|0||0
10|metadata|TEXT|0||0
11|created_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|topology_routes|rerouted_from|id|NO ACTION|SET NULL|NONE
1|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: topology_snapshots
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|project_id|TEXT|1||0
2|snapshot|TEXT|1||0
3|trigger|TEXT|0||0
4|node_count|INTEGER|0||0
5|edge_count|INTEGER|0||0
6|cluster_count|INTEGER|0||0
7|issues_detected|TEXT|0||0
8|created_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: topsi_user_settings
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|user_id|TEXT|0||1
1|default_confirmation_mode|TEXT|1|'confirm_destructive'|0
2|per_tool_overrides|TEXT|0||0
3|auto_approve_timeout_minutes|INTEGER|0||0
4|created_at|DATETIME|1|datetime('now', 'subsec')|0
5|updated_at|DATETIME|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
1

================================================================================
TABLE: trigger_executions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|trigger_id|TEXT|1||0
2|workflow_run_id|TEXT|0||0
3|status|TEXT|1|'pending'|0
4|started_at|TEXT|1|datetime('now', 'subsec')|0
5|completed_at|TEXT|0||0
6|duration_ms|INTEGER|0||0
7|error|TEXT|0||0
8|records_staged|INTEGER|1|0|0
9|source_type|TEXT|0||0
10|source_id|TEXT|0||0
11|metadata|TEXT|0||0
12|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|workflow_triggers|trigger_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: user_invitations
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1|randomblob(16)|1
1|token|TEXT|1||0
2|host_id|BLOB|1||0
3|invitee_email|TEXT|1||0
4|invitee_name|TEXT|0||0
5|invitee_user_id|BLOB|0||0
6|status|TEXT|1|'pending'|0
7|project_id|BLOB|0||0
8|project_role|TEXT|0|'viewer'|0
9|expires_at|TEXT|1|datetime('now', '+7 days')|0
10|accepted_at|TEXT|0||0
11|created_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|SET NULL|NONE
1|0|users|invitee_user_id|id|NO ACTION|SET NULL|NONE
2|0|users|host_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
1

================================================================================
TABLE: user_knowledge_sources
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0|randomblob(16)|1
1|user_id|BLOB|1||0
2|source_type|TEXT|1||0
3|source_id|TEXT|1||0
4|source_title|TEXT|1|''|0
5|source_summary|TEXT|0||0
6|related_company_id|BLOB|0||0
7|related_person_id|BLOB|0||0
8|related_project_id|BLOB|0||0
9|coverage_score|REAL|1|0.0|0
10|is_active|INTEGER|1|1|0
11|is_stale|INTEGER|1|0|0
12|last_refreshed_at|TEXT|1|datetime('now', 'subsec')|0
13|created_at|TEXT|1|datetime('now', 'subsec')|0
14|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|related_project_id|id|NO ACTION|NO ACTION|NONE
1|0|persons|related_person_id|id|NO ACTION|NO ACTION|NONE
2|0|companies|related_company_id|id|NO ACTION|NO ACTION|NONE
3|0|users|user_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: user_platform_roles
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|user_id|TEXT|1||0
2|role|TEXT|1||0
3|granted_by|TEXT|0||0
4|granted_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
2

================================================================================
TABLE: users
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1||1
1|username|TEXT|1||0
2|email|TEXT|1||0
3|password_hash|TEXT|1||0
4|full_name|TEXT|1||0
5|avatar_url|TEXT|0||0
6|is_active|INTEGER|1|1|0
7|is_admin|INTEGER|1|0|0
8|last_login_at|TEXT|0||0
9|created_at|TEXT|1|datetime('now')|0
10|updated_at|TEXT|1|datetime('now')|0
11|created_by|BLOB|0||0
12|external_provider|TEXT|0||0
13|external_id|TEXT|0||0
14|deleted_at|TEXT|0||0
15|deleted_by|BLOB|0||0
16|vibe_balance|REAL|0|100.0|0
17|user_role|TEXT|1|'guest'|0
18|invited_by|BLOB|0||0
19|home_project_id|BLOB|0||0
20|person_id|BLOB|0||0
21|skill_level|TEXT|0||0
22|compliance_status|TEXT|1|'good'|0
23|primary_tools|TEXT|0||0
24|home_organization_id|BLOB|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|organizations|home_organization_id|id|NO ACTION|NO ACTION|NONE
1|0|users|invited_by|id|NO ACTION|SET NULL|NONE
2|0|users|deleted_by|id|NO ACTION|SET NULL|NONE

--- Row count ---
19

================================================================================
TABLE: users_backup_20260215
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id||0||0
1|username|TEXT|0||0
2|email|TEXT|0||0
3|password_hash|TEXT|0||0
4|full_name|TEXT|0||0
5|avatar_url|TEXT|0||0
6|is_active|INT|0||0
7|is_admin|INT|0||0
8|last_login_at|TEXT|0||0
9|created_at|TEXT|0||0
10|updated_at|TEXT|0||0
11|created_by||0||0
12|external_provider|TEXT|0||0
13|external_id|TEXT|0||0
14|deleted_at|TEXT|0||0
15|deleted_by||0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
8

================================================================================
TABLE: vibe_deposits
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|tx_hash|TEXT|1||0
3|sender_address|TEXT|1||0
4|amount_vibe|INTEGER|1||0
5|status|TEXT|1|'pending'|0
6|block_height|INTEGER|0||0
7|detected_at|TEXT|1|datetime('now', 'subsec')|0
8|credited_at|TEXT|0||0
9|error_message|TEXT|0||0
10|created_at|TEXT|1|datetime('now', 'subsec')|0
11|updated_at|TEXT|1|datetime('now', 'subsec')|0
12|payment_method|TEXT|1|'aptos_vibe'|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
9

================================================================================
TABLE: vibe_ledger
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|user_id|BLOB|1||0
2|device_id|TEXT|0||0
3|amount|REAL|1||0
4|transaction_type|TEXT|1||0
5|balance_before|REAL|1||0
6|balance_after|REAL|1||0
7|task_execution_id|BLOB|0||0
8|description|TEXT|0||0
9|metadata|TEXT|0||0
10|created_at|DATETIME|0|CURRENT_TIMESTAMP|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|user_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
11

================================================================================
TABLE: vibe_transactions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|source_type|TEXT|1||0
2|source_id|BLOB|1||0
3|amount_vibe|INTEGER|1||0
4|input_tokens|INTEGER|0||0
5|output_tokens|INTEGER|0||0
6|model|TEXT|0||0
7|provider|TEXT|0||0
8|calculated_cost_cents|INTEGER|0||0
9|aptos_tx_hash|TEXT|0||0
10|aptos_tx_status|TEXT|0||0
11|task_id|BLOB|0||0
12|task_attempt_id|BLOB|0||0
13|process_id|BLOB|0||0
14|description|TEXT|0||0
15|metadata|TEXT|0||0
16|created_at|TEXT|1|datetime('now', 'subsec')|0
17|updated_at|TEXT|1|datetime('now', 'subsec')|0
18|on_chain_synced|INTEGER|1|0|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
222

================================================================================
TABLE: vibe_withdrawals
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|destination_address|TEXT|1||0
3|amount_vibe|INTEGER|1||0
4|status|TEXT|1|'pending'|0
5|tx_hash|TEXT|0||0
6|requested_at|TEXT|1|datetime('now', 'subsec')|0
7|processed_at|TEXT|0||0
8|error_message|TEXT|0||0
9|created_at|TEXT|1|datetime('now', 'subsec')|0
10|updated_at|TEXT|1|datetime('now', 'subsec')|0
11|payment_method|TEXT|1|'aptos_vibe'|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|NO ACTION|NONE

--- Row count ---
0

================================================================================
TABLE: views
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|name|TEXT|1||0
3|view_type|TEXT|1||0
4|filters|TEXT|0|'{}'|0
5|sorts|TEXT|0|'[]'|0
6|visible_properties|TEXT|0|'[]'|0
7|created_at|TEXT|1|datetime('now', 'subsec')|0
8|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: virtual_spaces
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|1|randomblob(16)|1
1|owner_id|BLOB|1||0
2|space_name|TEXT|1||0
3|description|TEXT|0||0
4|world_x|REAL|1|0.0|0
5|world_y|REAL|1|0.0|0
6|world_z|REAL|1|0.0|0
7|spawn_x|REAL|1|0.0|0
8|spawn_y|REAL|1|1.8|0
9|spawn_z|REAL|1|5.0|0
10|theme|TEXT|0|'default'|0
11|is_public|INTEGER|1|0|0
12|max_guests|INTEGER|1|20|0
13|metadata|TEXT|0||0
14|created_at|TEXT|1|datetime('now')|0
15|updated_at|TEXT|1|datetime('now')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|users|owner_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
3

================================================================================
TABLE: wide_research_sessions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|agent_flow_id|BLOB|0||0
2|parent_agent_id|BLOB|0||0
3|task_description|TEXT|1||0
4|total_subagents|INTEGER|1||0
5|completed_subagents|INTEGER|0|0|0
6|failed_subagents|INTEGER|0|0|0
7|status|TEXT|1|'spawning'|0
8|parallelism_limit|INTEGER|0|10|0
9|timeout_per_subagent|INTEGER|0|300000|0
10|aggregated_result_artifact_id|BLOB|0||0
11|created_at|TEXT|1|datetime('now', 'subsec')|0
12|completed_at|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|execution_artifacts|aggregated_result_artifact_id|id|NO ACTION|SET NULL|NONE
1|0|agents|parent_agent_id|id|NO ACTION|SET NULL|NONE
2|0|agent_flows|agent_flow_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: wide_research_subagents
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|session_id|BLOB|1||0
2|subagent_index|INTEGER|1||0
3|target_item|TEXT|1||0
4|target_metadata|TEXT|0||0
5|status|TEXT|1|'pending'|0
6|execution_process_id|BLOB|0||0
7|result_artifact_id|BLOB|0||0
8|error_message|TEXT|0||0
9|started_at|TEXT|0||0
10|completed_at|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|execution_artifacts|result_artifact_id|id|NO ACTION|SET NULL|NONE
1|0|execution_processes|execution_process_id|id|NO ACTION|SET NULL|NONE
2|0|wide_research_sessions|session_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: workflow_artifacts
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|workflow_id|TEXT|1||0
2|artifact_type|TEXT|1||0
3|title|TEXT|1||0
4|content|TEXT|0||0
5|file_url|TEXT|0||0
6|metadata|TEXT|0||0
7|created_at|TEXT|1|datetime('now', 'subsec')|0
8|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|conference_workflows|workflow_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: workflow_definitions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|organization_id|BLOB|0||0
2|name|TEXT|1||0
3|description|TEXT|0||0
4|steps|TEXT|1|'[]'|0
5|is_system|BOOLEAN|1|0|0
6|created_at|DATETIME|1|datetime('now', 'subsec')|0
7|updated_at|DATETIME|1|datetime('now', 'subsec')|0
8|owner_type|TEXT|1|'system'|0
9|owner_id|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
6

================================================================================
TABLE: workflow_events
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|workflow_id|BLOB|1||0
2|event_type|TEXT|1||0
3|stage_name|TEXT|0||0
4|entity_id|BLOB|0||0
5|artifact_id|BLOB|0||0
6|event_data|TEXT|0||0
7|message|TEXT|1||0
8|severity|TEXT|0|'info'|0
9|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|conference_workflows|workflow_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: workflow_executions
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|agent_id|TEXT|1||0
2|workflow_id|TEXT|1||0
3|workflow_name|TEXT|1||0
4|project_id|TEXT|0||0
5|state|TEXT|1||0
6|context|TEXT|1||0
7|current_stage|INTEGER|1|0|0
8|created_tasks|TEXT|0||0
9|deliverables|TEXT|0||0
10|started_at|TEXT|1||0
11|updated_at|TEXT|1||0
12|completed_at|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|SET NULL|NONE

--- Row count ---
0

================================================================================
TABLE: workflow_output_staging
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|workflow_run_id|TEXT|1||0
2|workflow_id|TEXT|1||0
3|node_id|TEXT|1||0
4|data_source_id|TEXT|0||0
5|organization_id|TEXT|0||0
6|project_id|TEXT|0||0
7|target_type|TEXT|1||0
8|record_data|TEXT|1||0
9|status|TEXT|1|'pending_review'|0
10|duplicate_of_id|TEXT|0||0
11|duplicate_of_type|TEXT|0||0
12|confidence|REAL|0||0
13|error_message|TEXT|0||0
14|reviewed_by|TEXT|0||0
15|reviewed_at|DATETIME|0||0
16|committed_at|DATETIME|0||0
17|created_at|DATETIME|1|datetime('now', 'subsec')|0
18|updated_at|DATETIME|1|datetime('now', 'subsec')|0
19|validation_errors|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: workflow_qa_runs
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|workflow_id|BLOB|1||0
2|stage_name|TEXT|1||0
3|artifact_id|BLOB|0||0
4|checklist_items|TEXT|1||0
5|confidence_level|TEXT|1||0
6|overall_score|REAL|1||0
7|decision|TEXT|1||0
8|retry_guidance|TEXT|0||0
9|escalation_reason|TEXT|0||0
10|agent_id|BLOB|0||0
11|execution_time_ms|INTEGER|0||0
12|tokens_used|INTEGER|0||0
13|created_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|agents|agent_id|id|NO ACTION|NO ACTION|NONE
1|0|conference_workflows|workflow_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

================================================================================
TABLE: workflow_runs
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|workflow_id|TEXT|1||0
2|workflow_name|TEXT|1||0
3|data_source_id|TEXT|0||0
4|organization_id|TEXT|0||0
5|project_id|TEXT|0||0
6|model_used|TEXT|0||0
7|status|TEXT|1|'completed'|0
8|total_input_tokens|INTEGER|0|0|0
9|total_output_tokens|INTEGER|0|0|0
10|total_estimated_cost_micros|INTEGER|0|0|0
11|total_records_staged|INTEGER|0|0|0
12|total_records_approved|INTEGER|0|0|0
13|total_records_rejected|INTEGER|0|0|0
14|total_records_committed|INTEGER|0|0|0
15|total_duplicates_found|INTEGER|0|0|0
16|total_validation_errors|INTEGER|0|0|0
17|node_count|INTEGER|0|0|0
18|llm_node_count|INTEGER|0|0|0
19|duration_ms|INTEGER|0||0
20|started_at|TEXT|1|datetime('now', 'subsec')|0
21|completed_at|TEXT|0||0
22|created_at|TEXT|1|datetime('now', 'subsec')|0
23|content_hash|TEXT|0||0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
0

================================================================================
TABLE: workflow_triggers
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|TEXT|1||1
1|workflow_id|TEXT|1||0
2|name|TEXT|1|''|0
3|enabled|INTEGER|1|0|0
4|trigger_type|TEXT|1|'data_source_created'|0
5|filter_data_source_types|TEXT|0||0
6|filter_organization_id|TEXT|0||0
7|filter_project_id|TEXT|0||0
8|filter_tags|TEXT|0||0
9|model_override|TEXT|0||0
10|auto_approve|INTEGER|1|0|0
11|webhook_secret|TEXT|0||0
12|webhook_url|TEXT|0||0
13|cooldown_seconds|INTEGER|1|0|0
14|max_retries|INTEGER|1|0|0
15|last_error|TEXT|0||0
16|retry_count|INTEGER|1|0|0
17|next_retry_at|TEXT|0||0
18|last_triggered_at|TEXT|0||0
19|trigger_count|INTEGER|1|0|0
20|created_at|TEXT|1|datetime('now', 'subsec')|0
21|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
(none)

--- Row count ---
1

================================================================================
TABLE: zoho_integrations
================================================================================

--- PRAGMA table_info (cid|name|type|notnull|dflt_value|pk) ---
0|id|BLOB|0||1
1|project_id|BLOB|1||0
2|zoho_org_id|TEXT|0||0
3|zoho_domain|TEXT|0||0
4|access_token|TEXT|0||0
5|refresh_token|TEXT|0||0
6|token_expires_at|TEXT|0||0
7|granted_scopes|TEXT|0||0
8|sync_contacts|INTEGER|0|1|0
9|sync_deals|INTEGER|0|1|0
10|sync_activities|INTEGER|0|1|0
11|sync_direction|TEXT|0|'bidirectional'|0
12|contact_field_mapping|TEXT|0||0
13|deal_field_mapping|TEXT|0||0
14|last_contact_sync_at|TEXT|0||0
15|last_deal_sync_at|TEXT|0||0
16|last_activity_sync_at|TEXT|0||0
17|status|TEXT|1|'active'|0
18|last_error|TEXT|0||0
19|created_at|TEXT|1|datetime('now', 'subsec')|0
20|updated_at|TEXT|1|datetime('now', 'subsec')|0

--- PRAGMA foreign_key_list (id|seq|table|from|to|on_update|on_delete|match) ---
0|0|projects|project_id|id|NO ACTION|CASCADE|NONE

--- Row count ---
0

