#![allow(dead_code)]
use db::models::{
    project::Project,
    task::{CreateTask, Task},
};
use rmcp::{ErrorData, handler::server::tool::Parameters, model::CallToolResult, tool};
// TODO(dbuuid): migrate Uuid → DbUuid — see planning/2026-03-17--plan--dbuuid-migration.md
use uuid::Uuid;

use super::{TaskServer, helpers::*, types::*};

impl TaskServer {
    #[tool(description = "List all projects accessible to the current user")]
    pub(super) async fn list_projects(&self) -> Result<CallToolResult, ErrorData> {
        let projects_result = if self.is_admin || self.user_id.is_none() {
            Project::find_all(&self.pool).await
        } else {
            // Safety: checked self.user_id.is_none() above, so this is guaranteed Some
            let user_id = self.user_id.unwrap_or_default();
            let user_id_bytes = user_id.to_string();

            #[derive(sqlx::FromRow)]
            struct ProjId {
                project_id: String,
            }

            let project_ids: Vec<String> = match sqlx::query_as::<_, ProjId>(
                "SELECT DISTINCT project_id FROM project_members WHERE user_id = ?",
            )
            .bind(&user_id_bytes)
            .fetch_all(&self.pool)
            .await
            {
                Ok(rows) => rows.into_iter().map(|r| r.project_id).collect(),
                Err(e) => {
                    return Ok(error_result(
                        "Failed to retrieve user projects",
                        Some(&e.to_string()),
                    ));
                }
            };

            let mut projects = Vec::new();
            for pid_str in project_ids {
                if let Ok(Some(project)) = Project::find_by_id(&self.pool, &pid_str).await {
                    projects.push(project);
                }
            }
            Ok(projects)
        };

        match projects_result {
            Ok(projects) => {
                let count = projects.len();
                let project_summaries: Vec<ProjectSummary> = projects
                    .into_iter()
                    .map(|project| ProjectSummary {
                        id: project.id.to_string(),
                        name: project.name,
                        git_repo_path: project.git_repo_path,
                        setup_script: project.setup_script,
                        cleanup_script: project.cleanup_script,
                        dev_script: project.dev_script,
                        created_at: project.created_at.to_rfc3339(),
                        updated_at: project.updated_at.to_rfc3339(),
                    })
                    .collect();

                Ok(success_json(&ListProjectsResponse {
                    success: true,
                    projects: project_summaries,
                    count,
                }))
            }
            Err(e) => Ok(error_result(
                "Failed to retrieve projects",
                Some(&e.to_string()),
            )),
        }
    }

    #[tool(
        description = "List all members of a project with their roles. `project_id` is required!"
    )]
    pub(super) async fn list_project_members(
        &self,
        Parameters(ListProjectMembersRequest { project_id }): Parameters<ListProjectMembersRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::Content::text(
                        r#"{"success": false, "error": "Invalid project ID format"}"#,
                    ),
                ]));
            }
        };

        #[derive(sqlx::FromRow)]
        struct MemberRow {
            user_id: Vec<u8>,
            username: String,
            full_name: String,
            role: String,
            is_admin: i32,
        }

        let members = sqlx::query_as::<_, MemberRow>(
            r#"SELECT pm.user_id, u.username, u.full_name, pm.role, u.is_admin
               FROM project_members pm
               JOIN users u ON pm.user_id = u.id
               WHERE pm.project_id = ? AND u.is_active = 1"#,
        )
        .bind(project_uuid.to_string())
        .fetch_all(&self.pool)
        .await;

        match members {
            Ok(rows) => {
                let members: Vec<ProjectMemberSummary> = rows
                    .iter()
                    .filter_map(|r| {
                        Uuid::from_slice(&r.user_id)
                            .ok()
                            .map(|uid| ProjectMemberSummary {
                                user_id: uid.to_string(),
                                username: r.username.clone(),
                                full_name: r.full_name.clone(),
                                role: r.role.clone(),
                                is_admin: r.is_admin == 1,
                            })
                    })
                    .collect();
                let count = members.len();
                let response = ListProjectMembersResponse {
                    success: true,
                    members,
                    count,
                };
                Ok(rmcp::model::CallToolResult::success(vec![
                    rmcp::model::Content::text(to_json_pretty(&response)),
                ]))
            }
            Err(e) => {
                let msg = format!(r#"{{"success": false, "error": "{}"}}"#, e);
                Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::Content::text(msg),
                ]))
            }
        }
    }

    #[tool(
        description = "Scaffold a new project from a template with pre-configured tasks and board. Templates: 'software' (dev lifecycle), 'research' (deep investigation), 'marketing' (campaign management), 'client_onboarding' (new client setup). Returns the created project ID and task list."
    )]
    pub(super) async fn scaffold_project(
        &self,
        Parameters(req): Parameters<ScaffoldProjectRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let org_id = req
            .organization_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok())
            .map(|u| u.to_string());
        let client_id = req
            .client_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok())
            .map(|u| u.to_string());

        let template = req.template.to_lowercase();
        let tasks_def: Vec<(&str, &str, &str, Option<&str>, Option<&str>)> = match template.as_str()
        {
            "software" => vec![
                (
                    "Project Setup & Environment",
                    "Initialize repository, configure CI/CD, set up dev environment",
                    "high",
                    Some(
                        "Repository initialized, CI pipeline green, README with setup instructions",
                    ),
                    Some("Git repo with working build, CI config, developer onboarding docs"),
                ),
                (
                    "Requirements & Architecture",
                    "Define functional requirements, design system architecture, document API contracts",
                    "high",
                    Some(
                        "Requirements doc reviewed, architecture diagram approved, API contracts defined",
                    ),
                    Some(
                        "Architecture decision records (ADRs), API specification (OpenAPI/protobuf), system diagram",
                    ),
                ),
                (
                    "Core Implementation",
                    "Build core features per architecture spec",
                    "high",
                    Some("All core features implemented, unit tests passing, code review approved"),
                    Some("Source code with tests, PR merged to main"),
                ),
                (
                    "Testing & QA",
                    "Write integration tests, perform QA validation, fix bugs",
                    "medium",
                    Some("Test coverage > 80%, no critical bugs, QA sign-off"),
                    Some("Test suite, QA report, bug fix PRs"),
                ),
                (
                    "Documentation & Deploy",
                    "Write user docs, prepare deployment, create runbook",
                    "medium",
                    Some("Docs published, staging deployment successful, runbook reviewed"),
                    Some("User documentation, deployment scripts, operational runbook"),
                ),
            ],
            "research" => {
                let _topic = req.topic.as_deref().unwrap_or("the research subject");
                // We'll use the topic in descriptions below
                vec![
                    (
                        "Literature Review & Source Collection",
                        "Gather and catalog existing research, papers, and data sources",
                        "high",
                        Some(
                            "Minimum 10 primary sources identified and summarized, research gaps documented",
                        ),
                        Some("Annotated bibliography, source database, gap analysis document"),
                    ),
                    (
                        "Hypothesis Formation",
                        "Analyze collected sources, identify patterns, formulate research questions and hypotheses",
                        "high",
                        Some(
                            "Clear research questions defined, testable hypotheses documented, methodology selected",
                        ),
                        Some(
                            "Research proposal with hypotheses, methodology section, expected outcomes",
                        ),
                    ),
                    (
                        "Data Collection & Analysis",
                        "Collect primary data, run experiments or analyses, document findings",
                        "high",
                        Some(
                            "Data collection complete, statistical analysis performed, preliminary findings documented",
                        ),
                        Some(
                            "Raw data files, analysis notebooks/scripts, preliminary findings report",
                        ),
                    ),
                    (
                        "Synthesis & Insights",
                        "Synthesize findings, draw conclusions, identify implications and next steps",
                        "medium",
                        Some(
                            "Key insights documented, conclusions validated against hypotheses, implications mapped",
                        ),
                        Some("Research report with findings, implications matrix, recommendations"),
                    ),
                    (
                        "Report & Knowledge Integration",
                        "Write final report, integrate findings into knowledge base, identify follow-up research",
                        "medium",
                        Some(
                            "Final report reviewed and approved, knowledge base updated, follow-up topics identified",
                        ),
                        Some(
                            "Final research report, knowledge base entries, follow-up research proposals",
                        ),
                    ),
                ]
            }
            "marketing" => vec![
                (
                    "Campaign Strategy & Brief",
                    "Define target audience, messaging, channels, budget, and success metrics",
                    "high",
                    Some(
                        "Campaign brief approved, target personas defined, channel mix selected, KPIs set",
                    ),
                    Some(
                        "Campaign brief document, persona profiles, channel strategy, KPI dashboard setup",
                    ),
                ),
                (
                    "Content Creation",
                    "Produce campaign assets: copy, visuals, videos, landing pages",
                    "high",
                    Some(
                        "All assets created and reviewed, brand guidelines followed, A/B variants prepared",
                    ),
                    Some("Creative assets (copy, images, video), landing pages, A/B test variants"),
                ),
                (
                    "Channel Setup & Launch",
                    "Configure ad platforms, schedule posts, set up tracking, launch campaign",
                    "medium",
                    Some("All channels configured, tracking pixels verified, campaign live"),
                    Some("Platform configurations, UTM tracking sheet, launch confirmation"),
                ),
                (
                    "Monitor & Optimize",
                    "Track performance metrics, optimize targeting and spend, A/B test results",
                    "medium",
                    Some("Weekly reports delivered, optimizations applied, A/B tests concluded"),
                    Some("Performance reports, optimization log, A/B test results"),
                ),
                (
                    "Analysis & Retrospective",
                    "Compile final results, calculate ROI, document learnings for future campaigns",
                    "low",
                    Some(
                        "Final report with ROI delivered, learnings documented, recommendations for next campaign",
                    ),
                    Some("Campaign results report, ROI analysis, learnings document"),
                ),
            ],
            "client_onboarding" => {
                let _client = req.client_name.as_deref().unwrap_or("the client");
                vec![
                    (
                        "Discovery & Intake",
                        "Conduct intake call, gather requirements, understand business context and goals",
                        "critical",
                        Some(
                            "Intake form completed, requirements documented, stakeholders identified, timeline agreed",
                        ),
                        Some(
                            "Intake form, requirements document, stakeholder map, project timeline",
                        ),
                    ),
                    (
                        "Proposal & Agreement",
                        "Draft proposal, define scope of work, negotiate terms, execute agreement",
                        "critical",
                        Some("Proposal approved by client, SOW signed, payment terms agreed"),
                        Some("Signed proposal/SOW, payment schedule, project charter"),
                    ),
                    (
                        "Environment & Access Setup",
                        "Set up client workspace, configure access, create project structure",
                        "high",
                        Some(
                            "Client workspace created, all access provisioned, project boards set up",
                        ),
                        Some(
                            "Workspace configuration, access credentials (securely shared), project board with initial tasks",
                        ),
                    ),
                    (
                        "Kickoff & Alignment",
                        "Run kickoff meeting, align on process, establish communication cadence",
                        "high",
                        Some(
                            "Kickoff meeting completed, communication channels established, first sprint planned",
                        ),
                        Some("Kickoff meeting notes, communication plan, sprint 1 backlog"),
                    ),
                    (
                        "First Deliverable & Feedback Loop",
                        "Deliver first milestone, gather feedback, iterate on process",
                        "medium",
                        Some(
                            "First deliverable reviewed by client, feedback incorporated, process refined",
                        ),
                        Some("First deliverable, client feedback document, updated process notes"),
                    ),
                ]
            }
            _ => {
                return Ok(error_result(
                    "Unknown template. Valid: software, research, marketing, client_onboarding",
                    None,
                ));
            }
        };

        // Create the project
        let project_id = Uuid::new_v4();
        let create_project = db::models::project::CreateProject {
            name: req.name.clone(),
            git_repo_path: req.git_repo_path.clone(),
            use_existing_repo: true,
            setup_script: None,
            dev_script: None,
            cleanup_script: None,
            copy_files: None,
            organization_id: org_id,
            client_id,
            folder_id: None,
            parent_project_id: None,
        };

        let project =
            match Project::create(&self.pool, &create_project, &project_id.to_string()).await {
                Ok(p) => p,
                Err(e) => {
                    return Ok(error_result(
                        "Failed to create project",
                        Some(&e.to_string()),
                    ));
                }
            };

        // Create tasks from template
        let mut created_tasks = Vec::new();
        let created_by = self
            .user_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "mcp".to_string());

        for (i, (title, description, priority_str, criteria, output_fmt)) in
            tasks_def.iter().enumerate()
        {
            let task_id = Uuid::new_v4().to_string();
            let priority = parse_priority(priority_str);
            let create_task = CreateTask {
                project_id: project.id.clone(),
                pod_id: None,
                board_id: None,
                title: title.to_string(),
                description: Some(description.to_string()),
                parent_task_attempt: None,
                image_ids: None,
                priority,
                assignee_id: None,
                assignee_type: None,
                assigned_agent: None,
                agent_id: None,
                assigned_mcps: None,
                created_by: created_by.clone(),
                requires_approval: None,
                parent_task_id: None,
                tags: Some(vec![template.clone()]),
                due_date: None,
                custom_properties: None,
                scheduled_start: None,
                scheduled_end: None,
                screenshot: None,
                completion_criteria: criteria.map(|s| s.to_string()),
                output_format: output_fmt.map(|s| s.to_string()),
                collaborators: None,
            };

            match Task::create(&self.pool, &create_task, &task_id).await {
                Ok(_t) => {
                    created_tasks.push(serde_json::json!({
                        "order": i + 1,
                        "task_id": task_id.to_string(),
                        "title": title,
                        "priority": priority_str,
                        "completion_criteria": criteria,
                        "output_format": output_fmt,
                    }));
                }
                Err(e) => {
                    created_tasks.push(serde_json::json!({
                        "order": i + 1,
                        "title": title,
                        "error": e.to_string(),
                    }));
                }
            }
        }

        Ok(success_json(&serde_json::json!({
            "success": true,
            "project_id": project.id.to_string(),
            "project_name": project.name,
            "template": template,
            "tasks_created": created_tasks.len(),
            "tasks": created_tasks,
            "message": format!("Project '{}' scaffolded with {} tasks from '{}' template", project.name, created_tasks.len(), template),
        })))
    }
}
