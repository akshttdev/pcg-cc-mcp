-- Seed 4 system workflows that were added programmatically but not present in the dev seed database.
-- Uses INSERT OR IGNORE so this is safe to run on databases that already have them.

INSERT OR IGNORE INTO workflow_definitions (id, owner_type, name, description, steps, is_system)
VALUES (
  'bug_triage_pipeline',
  'system',
  'Bug Triage Pipeline',
  'Analyze bug reports, categorize by severity, and create prioritized tasks.',
  '{"nodes":[{"id":"analyze_bugs","name":"Analyze Bug Reports","node_type":"llm_analyze","parameters":{"prompt_template":"Analyze the following bug reports and categorize each one.\nFor each bug, provide:\n- title: Short descriptive title\n- description: Detailed description of the issue\n- severity: One of critical, high, medium, low\n- component: Affected system component\n- reproducible: true/false\n- suggested_fix: Brief suggestion for resolution\n\nOutput as JSON with a top-level \"bugs\" array.\n\nContent:\n{{content}}","output_schema":"bugs[]"},"position":{"x":100.0,"y":200.0}},{"id":"filter_critical","name":"Filter Critical/High","node_type":"conditional","parameters":{"condition":"contains:critical","true_label":"has_critical","false_label":"no_critical"},"position":{"x":500.0,"y":200.0}},{"id":"create_tasks","name":"Create Bug Tasks","node_type":"output_tasks","parameters":{},"position":{"x":900.0,"y":200.0}}],"connections":[{"source":"analyze_bugs","target":"filter_critical","source_output":0,"target_input":0},{"source":"analyze_bugs","target":"create_tasks","source_output":0,"target_input":0}],"default_model":null}',
  1
);

INSERT OR IGNORE INTO workflow_definitions (id, owner_type, name, description, steps, is_system)
VALUES (
  'sprint_planning',
  'system',
  'Sprint Planning from Requirements',
  'Break down requirements documents into agent-ready tasks with completion criteria.',
  '{"nodes":[{"id":"extract_requirements","name":"Extract Requirements","node_type":"llm_extract","parameters":{"prompt_template":"Analyze the following requirements document and extract individual work items.\nFor each requirement, provide:\n- title: Task title (action-oriented)\n- description: Detailed task description with acceptance criteria\n- priority: One of critical, high, medium, low\n- estimated_effort: small, medium, large\n- completion_criteria: Bullet-pointed list of what must be true for this to be done\n- output_format: Expected deliverable\n- dependencies: Array of other task titles this depends on\n\nOutput as JSON with a top-level \"tasks\" array.\n\nContent:\n{{content}}","output_schema":"tasks[]"},"position":{"x":100.0,"y":200.0}},{"id":"output_sprint_tasks","name":"Create Sprint Tasks","node_type":"output_tasks","parameters":{},"position":{"x":500.0,"y":200.0}}],"connections":[{"source":"extract_requirements","target":"output_sprint_tasks","source_output":0,"target_input":0}],"default_model":null}',
  1
);

INSERT OR IGNORE INTO workflow_definitions (id, owner_type, name, description, steps, is_system)
VALUES (
  'client_onboarding',
  'system',
  'Client Onboarding Pipeline',
  'Process new client data, create CRM contacts, and generate onboarding task checklist.',
  '{"nodes":[{"id":"extract_client_info","name":"Extract Client Info","node_type":"llm_extract","parameters":{"prompt_template":"Extract client and contact information from the following data.\nProvide:\n- contacts: Array of people with first_name, last_name, email, phone, job_title, company_name\n- company: Object with name, website, industry, address\n- onboarding_notes: Any special requirements or notes mentioned\n\nOutput as JSON.\n\nContent:\n{{content}}","output_schema":"contacts[]"},"position":{"x":100.0,"y":200.0}},{"id":"create_contacts","name":"Create CRM Contacts","node_type":"output_crm_contacts","parameters":{},"position":{"x":500.0,"y":100.0}},{"id":"create_onboarding_tasks","name":"Create Onboarding Tasks","node_type":"output_tasks","parameters":{},"position":{"x":500.0,"y":300.0}}],"connections":[{"source":"extract_client_info","target":"create_contacts","source_output":0,"target_input":0},{"source":"extract_client_info","target":"create_onboarding_tasks","source_output":0,"target_input":0}],"default_model":null}',
  1
);

INSERT OR IGNORE INTO workflow_definitions (id, owner_type, name, description, steps, is_system)
VALUES (
  'content_pipeline',
  'system',
  'Content Analysis Pipeline',
  'Analyze content for SEO opportunities and create actionable content tasks.',
  '{"nodes":[{"id":"summarize_content","name":"Summarize Content","node_type":"llm_summarize","parameters":{"prompt_template":"Summarize the following content, identifying:\n- key_topics: Main topics covered\n- target_audience: Who this content is for\n- content_type: blog, documentation, marketing, technical, etc.\n- word_count: Approximate word count\n- summary: 2-3 sentence summary\n\nOutput as JSON.\n\nContent:\n{{content}}","output_schema":"summary"},"position":{"x":100.0,"y":200.0}},{"id":"analyze_seo","name":"SEO Analysis","node_type":"llm_analyze","parameters":{"prompt_template":"Based on the content summary below, perform an SEO analysis.\nProvide:\n- keywords: Array of target keywords with search intent\n- content_gaps: Topics that should be covered but aren''t\n- optimization_tasks: Specific actions to improve SEO\n- competitor_angles: Angles competitors might use\n\nOutput as JSON with arrays for each field.\n\nSummary:\n{{previous_results}}","output_schema":"seo_analysis"},"position":{"x":500.0,"y":200.0}},{"id":"create_content_tasks","name":"Create Content Tasks","node_type":"output_tasks","parameters":{},"position":{"x":900.0,"y":200.0}}],"connections":[{"source":"summarize_content","target":"analyze_seo","source_output":0,"target_input":0},{"source":"analyze_seo","target":"create_content_tasks","source_output":0,"target_input":0}],"default_model":null}',
  1
);
