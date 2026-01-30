# PCG CLI Design

## Overview

The `pcg` command provides a Claude Code-like interactive terminal experience natively integrated with the PCG Dashboard. It combines conversational AI assistance with task tracking, agent coordination, and VIBE cost management.

## Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│                           PCG CLI (pcg)                                │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────┐ │
│  │   Terminal   │    │   Session    │    │     Agent Router         │ │
│  │    REPL      │───▶│   Manager    │───▶│  (Duck/Nora/Scout/etc)   │ │
│  └──────────────┘    └──────────────┘    └──────────────────────────┘ │
│         │                   │                        │                 │
│         │                   │                        │                 │
│         ▼                   ▼                        ▼                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────┐ │
│  │   Output     │    │   MCP Task   │    │     PCG Backend API      │ │
│  │   Stream     │    │   Server     │    │  (REST + SSE + WebSocket)│ │
│  └──────────────┘    └──────────────┘    └──────────────────────────┘ │
│                             │                        │                 │
│                             │                        │                 │
│                             ▼                        ▼                 │
│                      ┌──────────────────────────────────────────────┐ │
│                      │              PCG Database                     │ │
│                      │  (Tasks, Sessions, TokenUsage, Workflows)     │ │
│                      └──────────────────────────────────────────────┘ │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

## Components

### 1. Terminal REPL (`crates/cli/src/repl.rs`)

The main interactive loop that provides the Claude Code-like experience:

```rust
pub struct PcgRepl {
    session: Arc<DevSession>,
    agent_router: AgentRouter,
    output_handler: OutputHandler,
    mcp_client: McpClient,
}

impl PcgRepl {
    pub async fn run(&mut self) -> Result<()> {
        // Display welcome banner with session info
        self.display_banner().await?;

        loop {
            // Show prompt with session context
            let prompt = self.build_prompt();

            // Read user input (supports multi-line, history, completion)
            let input = self.read_input(&prompt)?;

            // Handle special commands (/task, /agent, /cost, /session, etc.)
            if input.starts_with('/') {
                self.handle_command(&input).await?;
                continue;
            }

            // Route to appropriate agent and stream response
            let response = self.agent_router.process(&input, &self.session).await?;
            self.stream_response(response).await?;
        }
    }
}
```

### 2. Session Manager (`crates/cli/src/session.rs`)

Manages the DevelopmentSession lifecycle:

```rust
pub struct DevSession {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub started_at: DateTime<Utc>,
    pub git_start_sha: Option<String>,

    // Real-time metrics
    pub total_tokens: AtomicI64,
    pub total_vibe_cost: AtomicI64,
    pub tasks_created: AtomicI32,
    pub tasks_completed: AtomicI32,
}

impl DevSession {
    pub async fn start(project_id: Uuid, title: &str) -> Result<Self>;
    pub async fn complete(&self) -> Result<SessionReport>;
    pub async fn link_task(&self, task_id: Uuid, relationship: TaskRelationship) -> Result<()>;
    pub fn update_cost(&self, tokens: i64, vibe: i64);
}
```

### 3. Agent Router (`crates/cli/src/agents.rs`)

Routes requests to appropriate agents based on context and intent:

```rust
pub struct AgentRouter {
    duck_executor: Duck,           // Code execution
    nora_client: NoraClient,       // Research & workflows
    scout_client: ScoutClient,     // Memory & knowledge
    editron_client: EditronClient, // Media pipeline
}

impl AgentRouter {
    pub async fn process(&self, input: &str, session: &DevSession) -> Result<AgentResponse> {
        // Classify intent
        let intent = self.classify_intent(input).await?;

        match intent {
            Intent::Code { .. } => self.duck_executor.execute(input, session).await,
            Intent::Research { .. } => self.nora_client.research(input, session).await,
            Intent::Memory { .. } => self.scout_client.recall(input, session).await,
            Intent::Media { .. } => self.editron_client.process(input, session).await,
            Intent::Task { .. } => self.handle_task_action(input, session).await,
            Intent::Conversation => self.default_chat(input, session).await,
        }
    }
}
```

### 4. MCP Integration (`crates/cli/src/mcp.rs`)

Leverages existing MCP task server for project/task operations:

```rust
pub struct McpClient {
    transport: StdioTransport,
}

impl McpClient {
    pub async fn list_projects(&self) -> Result<Vec<Project>>;
    pub async fn list_tasks(&self, project_id: Uuid) -> Result<Vec<Task>>;
    pub async fn create_task(&self, project_id: Uuid, title: &str) -> Result<Task>;
    pub async fn update_task(&self, task_id: Uuid, status: TaskStatus) -> Result<Task>;
}
```

### 5. Output Handler (`crates/cli/src/output.rs`)

Handles streaming output with rich terminal formatting:

```rust
pub struct OutputHandler {
    terminal: Terminal,
    markdown_renderer: MarkdownRenderer,
    ansi_formatter: AnsiFormatter,
}

impl OutputHandler {
    pub async fn stream_response(&self, response: impl Stream<Item = OutputChunk>) {
        while let Some(chunk) = response.next().await {
            match chunk {
                OutputChunk::Text(text) => self.render_markdown(&text),
                OutputChunk::Code { lang, content } => self.render_code(&lang, &content),
                OutputChunk::Tool { name, status, output } => self.render_tool(&name, &status, &output),
                OutputChunk::Cost { tokens, vibe } => self.update_status_bar(tokens, vibe),
                OutputChunk::Task { action, task } => self.render_task_action(&action, &task),
            }
        }
    }
}
```

## CLI Commands

### Session Commands
```
pcg                          # Start interactive session in current directory
pcg --project <name>         # Start session with specific project
pcg --resume <session-id>    # Resume previous session
pcg status                   # Show current session status
```

### Slash Commands (within REPL)
```
/task create <title>         # Create a new task
/task list                   # List tasks in current project
/task complete <id>          # Mark task as done
/task link <id>              # Link task to current session

/agent list                  # Show available agents
/agent switch <name>         # Switch primary agent
/agent spawn <name> <task>   # Spawn agent for specific task

/cost                        # Show current session costs
/cost history                # Show cost history

/session                     # Show session info
/session pause               # Pause current session
/session complete            # Complete session, generate report

/workflow list               # List available workflows
/workflow run <name>         # Execute a workflow

/help                        # Show all commands
```

## Data Flow

### 1. Starting a Session

```
User: pcg --project "PCG Dashboard"
           │
           ▼
┌─────────────────────────────────┐
│ 1. Load project from database   │
│ 2. Create DevelopmentSession    │
│ 3. Capture git SHA              │
│ 4. Initialize agent connections │
│ 5. Start REPL                   │
└─────────────────────────────────┘
           │
           ▼
╔═══════════════════════════════════════════════════════════════╗
║  PCG Development Session                                       ║
║  Project: PCG Dashboard | Session: a1b2c3d4                   ║
║  Started: 2026-01-26 10:30 | VIBE: 0                          ║
╠═══════════════════════════════════════════════════════════════╣
║                                                                ║
║  > Ready. What would you like to work on?                     ║
║                                                                ║
╚═══════════════════════════════════════════════════════════════╝
```

### 2. Processing a Request

```
User: "Add a logout button to the header"
           │
           ▼
┌─────────────────────────────────┐
│ 1. Parse input                  │
│ 2. Classify intent → Code       │
│ 3. Create task (auto)           │
│ 4. Route to Duck executor       │
│ 5. Stream response to terminal  │
│ 6. Update token/VIBE counts     │
│ 7. Link task to session         │
└─────────────────────────────────┘
           │
           ▼
╔═══════════════════════════════════════════════════════════════╗
║  ▶ Task Created: "Add logout button to header" (#t-4f7a)      ║
╠═══════════════════════════════════════════════════════════════╣
║                                                                ║
║  I'll add a logout button to the header component.            ║
║                                                                ║
║  Reading: src/components/Header.tsx                           ║
║  ──────────────────────────────────────────────────────────   ║
║  export function Header() {                                   ║
║    const { user, logout } = useAuth();                        ║
║    ...                                                        ║
║                                                                ║
║  Editing: src/components/Header.tsx                           ║
║  ──────────────────────────────────────────────────────────   ║
║  + <Button onClick={logout} variant="ghost">                  ║
║  +   <LogOut className="w-4 h-4 mr-2" />                      ║
║  +   Logout                                                   ║
║  + </Button>                                                  ║
║                                                                ║
║  ✓ Task completed: Added logout button                        ║
║                                                                ║
╠═══════════════════════════════════════════════════════════════╣
║  Tokens: 2,450 | VIBE: 122 | Tasks: 1 created, 1 completed    ║
╚═══════════════════════════════════════════════════════════════╝
```

### 3. Completing a Session

```
User: /session complete
           │
           ▼
┌─────────────────────────────────┐
│ 1. Capture final git SHA        │
│ 2. Calculate git diff stats     │
│ 3. Aggregate token/VIBE usage   │
│ 4. Generate session report      │
│ 5. Save to database             │
│ 6. Create task in Dashboard     │
└─────────────────────────────────┘
           │
           ▼
╔═══════════════════════════════════════════════════════════════╗
║  SESSION COMPLETED                                             ║
╠═══════════════════════════════════════════════════════════════╣
║                                                                ║
║  Duration: 2h 15m                                             ║
║  Files Changed: 12                                            ║
║  Lines: +234 / -45                                            ║
║  Commits: 3                                                   ║
║                                                                ║
║  Token Usage: 45,230 tokens                                   ║
║  VIBE Cost: 2,261 VIBE ($2.26 USD)                            ║
║                                                                ║
║  Tasks:                                                        ║
║  ✓ Add logout button to header                                ║
║  ✓ Fix navigation breadcrumb                                  ║
║  ✓ Update user avatar component                               ║
║                                                                ║
║  Report saved to: Development History board                   ║
║  View at: http://localhost:3000/projects/pcg/boards/dev-hist  ║
║                                                                ║
╚═══════════════════════════════════════════════════════════════╝
```

## Implementation Plan

### Phase 1: Core CLI Framework
- [ ] Create `crates/cli` with REPL infrastructure
- [ ] Implement session management
- [ ] Add basic command parsing
- [ ] Terminal output handling with rich formatting

### Phase 2: MCP Integration
- [ ] Connect to existing MCP task server
- [ ] Implement project/task operations
- [ ] Add task auto-creation from conversations

### Phase 3: Agent Integration
- [ ] Connect Duck executor for code operations
- [ ] Add agent routing logic
- [ ] Implement streaming output

### Phase 4: Cost Tracking
- [ ] Integrate TokenUsage tracking
- [ ] Real-time VIBE cost display
- [ ] Session cost aggregation

### Phase 5: Session Reports
- [ ] Implement git stats capture
- [ ] Auto-generate session reports
- [ ] Save to Development History board

## Configuration

The CLI reads configuration from `~/.pcg/config.toml`:

```toml
[server]
url = "http://localhost:3002"
api_key = "..."

[session]
auto_create_tasks = true
default_project = "PCG Dashboard"

[display]
theme = "dark"
show_cost_bar = true
markdown_rendering = true

[agents]
default = "duck"
nora_enabled = true
scout_enabled = true
```

## Environment Variables

```bash
PCG_SERVER_URL=http://localhost:3002
PCG_API_KEY=...
PCG_DEFAULT_PROJECT=...
PCG_AUTO_SESSION=true
```
