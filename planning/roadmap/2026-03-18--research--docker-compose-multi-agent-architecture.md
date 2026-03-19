# Docker/Docker Compose Architecture Patterns for Multi-Agent Orchestration — Deep Research

**Date:** 2026-03-18
**Type:** Research / Reference
**Purpose:** Extract principles, tools, and techniques from Docker and Docker Compose service-based architecture patterns applicable to our ORCHA agent orchestration system and task management processes.

---

## 1. Docker Compose for Multi-Agent Service Architecture

### 1.1 Service-Per-Agent Pattern

The dominant pattern is **one container per agent**, where each agent is a self-contained Docker service with its own Dockerfile, dependencies, and lifecycle. Docker's official [compose-for-agents](https://github.com/docker/compose-for-agents) repository demonstrates this across 14 example implementations spanning LangGraph, CrewAI, Google ADK, Agno, and more.

**Sequential Pipeline Pattern:**

```yaml
version: "3.9"
services:
  ingestor:
    build: ./agents/ingestor
    volumes:
      - ./data:/data
    restart: "no"

  summarizer:
    build: ./agents/summarizer
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    depends_on:
      ingestor:
        condition: service_completed_successfully
    volumes:
      - ./data:/data
    deploy:
      resources:
        limits:
          memory: 512M
    restart: "no"

  prioritizer:
    build: ./agents/prioritizer
    depends_on:
      summarizer:
        condition: service_completed_successfully
    volumes:
      - ./data:/data
    restart: "no"

  formatter:
    build: ./agents/formatter
    depends_on:
      prioritizer:
        condition: service_completed_successfully
    volumes:
      - ./data:/data
      - ./output:/output
    restart: "no"
```

Key design decisions:
- `condition: service_completed_successfully` enforces strict ordering (standard `depends_on` only waits for startup, not completion)
- Shared volumes (`./data:/data`) enable file-based inter-agent communication
- `restart: "no"` prevents re-execution of completed pipeline stages
- Per-service memory limits (`512M`) prevent resource starvation

**Application to ORCHA:** A task management system could model each coding agent as a service that reads task assignments from a shared volume or database, executes work in isolation, and writes results back. The `service_completed_successfully` condition maps to our FSM state transitions — one agent's completion triggers the next phase.

### 1.2 Shared Services Pattern

Common shared infrastructure services across agent deployments:
- **Databases**: PostgreSQL, SQLite (mounted via volumes)
- **Message queues**: RabbitMQ, Redis
- **Vector stores**: Qdrant, Weaviate, Chroma (each has official Docker images)
- **MCP Gateway**: Docker's [MCP Gateway](https://github.com/docker/mcp-gateway) consolidates multiple MCP servers into a single endpoint

**ADK + MCP Gateway example:**

```yaml
services:
  adk:
    build: .
    ports:
      - "8080:8080"
    environment:
      - MCPGATEWAY_ENDPOINT=http://mcp-gateway:8811/sse
    depends_on:
      - mcp-gateway
    models:
      gemma3:
        endpoint_var: MODEL_RUNNER_URL
        model_var: MODEL_RUNNER_MODEL

  mcp-gateway:
    image: docker/mcp-gateway:latest
    use_api_socket: true
    command:
      - --transport=sse
      - --servers=duckduckgo

models:
  gemma3:
    model: ai/gemma3:4B-Q4_0
    context_size: 10000
```

Notable: Docker Compose now supports a **top-level `models` block** that makes AI models first-class citizens alongside services.

**Application to ORCHA:** The MCP Gateway pattern maps to our MCP server consolidation needs. Rather than each agent container independently connecting to MCP servers, a gateway service provides a unified endpoint. This reduces connection overhead and enables centralized access control.

### 1.3 Init Container Pattern

Docker Compose lacks native init containers, but the pattern is replicated using `depends_on` + `service_completed_successfully`:

```yaml
services:
  init-migrate:
    image: myapp-migrations
    restart: "no"
    command: ["sh", "-c", "run-migrations.sh"]

  init-seed:
    depends_on:
      init-migrate:
        condition: service_completed_successfully
    restart: "no"
    command: ["sh", "-c", "seed-data.sh"]

  app:
    depends_on:
      init-seed:
        condition: service_completed_successfully
```

Chain: `init-wait → init-migrate → init-seed → init-index → app`. Always set `restart: "no"` on init services.

**Application to ORCHA:** Before agent services start, init containers can: run DB migrations, validate configuration, seed data, pre-warm vector stores, and verify LLM API connectivity. Failures in init prevent downstream agents from starting in a broken state.

### 1.4 Health Checks

```yaml
healthcheck:
  test: ["CMD-SHELL", "pg_isready -U postgres"]
  interval: 5s
  timeout: 2s
  retries: 10
  start_period: 30s
```

The `start_period` provides grace time before retries count, preventing premature failure during database initialization. Other services can use `depends_on: condition: service_healthy` to wait for true readiness.

**Application to ORCHA:** Agent services should expose health endpoints that verify: LLM API connectivity, database connection, workspace filesystem access, and MCP server availability. The orchestrator only dispatches tasks to agents reporting healthy status.

---

## 2. Container Orchestration Patterns for AI Agents

### 2.1 Dynamic Container Spawning

Two primary approaches:

1. **Docker Compose `--scale`**: Scale specific services dynamically: `docker compose up --scale worker=5`
2. **Docker Swarm replicas**: `docker service create --name ai-agent --replicas 5` with automatic load balancing

For more sophisticated scaling, containers can be spawned programmatically via the Docker API — the orchestrator creates/destroys agent containers as tasks arrive and complete.

### 2.2 Resource Limits and Isolation

```yaml
deploy:
  resources:
    limits:
      memory: 512M
      cpus: "1.0"
```

For AI workloads, resource limits are critical because LLM inference and agent code execution can consume unbounded memory if not constrained. Every agent service should have explicit memory and CPU limits.

### 2.3 GPU Sharing and Allocation

**Modern shorthand (Compose v2.30.0+):**
```yaml
services:
  inference:
    image: myorg/model-server:latest
    gpus:
      - driver: nvidia
        count: 1
```

**Extended syntax with device assignment:**
```yaml
deploy:
  resources:
    reservations:
      devices:
        - driver: nvidia
          device_ids: ["1"]
          capabilities: [gpu]
```

Multi-service GPU allocation assigns specific GPUs to different services via `device_ids`, preventing contention. NVIDIA MIG (Multi-Instance GPU) can partition A100 GPUs into up to 7 instances supporting 8 concurrent workloads.

**Docker Offload** (2025-2026) transparently runs GPU-heavy containers on cloud infrastructure from your local Docker environment, solving the hardware constraint problem for development.

### 2.4 Ephemeral vs Long-Lived Containers

| Container Lifecycle | Use Case | Restart Policy |
|--------------------|----------|----------------|
| **Ephemeral** | Per-task agent execution, sandboxed code runs | `restart: "no"` |
| **Long-lived** | Databases, vector stores, MCP gateways, message queues | `restart: unless-stopped` |
| **Pooled** | Pre-warmed agent containers claimed from a pool | Custom lifecycle management |

**Key insight:** Ephemeral containers per task provide the strongest isolation guarantee — malicious code injected during runtime vanishes when the container is destroyed. Long-lived services maintain state across agent lifecycles.

### 2.5 Auto-Scaling Based on Queue Depth

For Kubernetes deployments, **KEDA** (Kubernetes Event-Driven Autoscaling) scales agent pools based on queue depth:
- HPA calculates: `desired_replicas = total_queue_depth / target_per_pod`
- Can scale to zero when idle (cost optimization)
- Supports RabbitMQ, SQS, Redis, Prometheus metrics as triggers

**Application to ORCHA:** Ephemeral containers per coding task provide the strongest isolation. Long-lived agent services with queue-based scaling are better for throughput. A hybrid approach — pooled warm containers claimed per task, destroyed after — balances isolation with startup latency.

---

## 3. Service Mesh and Inter-Service Communication

### 3.1 Service Discovery

Docker Compose provides **automatic DNS-based service discovery** on custom networks. Services reference each other by name (e.g., `DATABASE_URL: postgresql://db:5432/myapp`). The default bridge network does NOT provide DNS resolution — always use named networks.

### 3.2 Network Segmentation

```yaml
networks:
  public:      # Load balancers, APIs exposed to external traffic
  backend:     # Databases, caches, message queues
    internal: true
  workers:     # Worker processes with restricted access
    internal: true
```

The `internal: true` flag prevents containers on that network from reaching the internet — critical for sandboxing agent code execution while allowing database access.

**Application to ORCHA:** Agent execution containers should be on an internal network with access to databases and MCP servers but no internet access. Only the orchestrator and specific tool services (Git, CI/CD) need external connectivity.

### 3.3 Communication Patterns

| Pattern | Mechanism | Use Case | Trade-offs |
|---------|-----------|----------|------------|
| **Synchronous REST/gRPC** | Direct HTTP calls between services | Real-time agent coordination | Tight coupling, cascading failures |
| **Async Message Queue** | RabbitMQ/Redis broker | Task distribution, event-driven | Complexity, eventual consistency |
| **Shared Volume** | File-based data exchange on mounted volumes | Data pipelines, artifact passing | No real-time notification, filesystem limits |
| **SSE/WebSocket** | Server-Sent Events via MCP Gateway | Tool call streaming | One-directional (SSE), connection management |

**gRPC advantages** over REST for agent communication: binary serialization via Protocol Buffers, HTTP/2 multiplexing, bidirectional streaming. For Docker health checks with gRPC services, use `grpc_health_probe` instead of HTTP endpoints.

### 3.4 MCP Gateway as Service Mesh

Docker's MCP Gateway acts as a unified control plane, consolidating multiple MCP servers into a single endpoint:

```yaml
mcp-gateway:
  image: docker/mcp-gateway:latest
  use_api_socket: true
  command:
    - --transport=sse
    - --servers=context7,node-code-sandbox
    - --catalog=/mcp-gateway-catalog.yaml
  volumes:
    - ./mcp-gateway-catalog.yaml:/mcp-gateway-catalog.yaml:ro
```

Individual MCP servers can have different isolation levels — sandbox servers run with `withNetworkMode("none")` while documentation servers retain network access.

### 3.5 A2A Protocol (Agent-to-Agent)

Google's [Agent2Agent protocol](https://a2a-protocol.org/latest/) (launched April 2025, now under Linux Foundation governance) enables inter-agent communication:
- Agents advertise capabilities via **Agent Cards** (JSON format)
- Client agents discover and delegate tasks to remote agents
- Supports text, audio, and video streaming
- Backed by 50+ partners including Atlassian, Salesforce, LangChain
- Deployable on Docker, Cloud Run, and GKE

**Application to ORCHA:** The A2A protocol enables agent services to discover each other's capabilities at runtime. An orchestrator container could query Agent Cards from available agent services to determine which can handle a given task — dynamic capability-based routing without hardcoded assignments.

---

## 4. Docker Compose Profiles and Environment Management

### 4.1 Profile-Based Service Selection

```yaml
services:
  app:
    image: myapp:latest
    # No profile = always starts

  debug-tools:
    image: busybox
    profiles: [debug]

  monitoring:
    image: prometheus
    profiles: [monitoring]

  llm-server:
    profiles: [gpu]
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              device_ids: ["1"]
              capabilities: [gpu]
```

Activation: `docker compose --profile debug --profile monitoring up` or `COMPOSE_PROFILES=debug,monitoring`

Common profile patterns:
- **infra-only**: Infrastructure in containers, app code runs natively
- **mock vs real**: Different service implementations per environment
- **CI optimization**: Exclude heavy services like LLM servers during CI
- **AI/ML workloads**: GPU-dependent services isolated from general development

### 4.2 Override Files

Docker Compose automatically merges `docker-compose.yml` with `docker-compose.override.yml`:

```
docker-compose.yml           # Base configuration
docker-compose.override.yml  # Development defaults (auto-merged)
docker-compose.staging.yml   # Staging overrides
docker-compose.prod.yml      # Production overrides
```

Usage: `docker compose -f docker-compose.yml -f docker-compose.prod.yml up`

**Warning**: If `docker-compose.override.yml` exists on a production server, development settings silently apply. Use explicit `-f` flags in production.

### 4.3 Secrets Management

Docker Compose secrets mount files to `/run/secrets/` rather than exposing values as environment variables:

```yaml
secrets:
  openai-api-key:
    file: ./secrets/openai-key.txt

services:
  agent:
    secrets:
      - openai-api-key
```

**Limitation**: Plain Docker Compose doesn't encrypt secrets at rest — file mounting only. For production, use HashiCorp Vault with AppRole authentication + Vault Agent sidecars, or cloud-native secret managers.

**Application to ORCHA:** Profiles enable running the system in different configurations without code changes: `docker compose --profile dev up` for local development, `--profile gpu --profile monitoring` for full stack with observability. LLM API keys should use the secrets mount pattern, not environment variables.

---

## 5. Real-World Multi-Agent Docker Deployments

### 5.1 Docker's compose-for-agents Repository

The [docker/compose-for-agents](https://github.com/docker/compose-for-agents) repository (14 examples) demonstrates:

| Framework | Pattern | Example |
|-----------|---------|---------|
| **LangGraph** | Single agent + SQL via MCP | Database interaction agent |
| **CrewAI** | Multi-agent role delegation | Marketing strategy agents |
| **Google ADK** | Multi-agent structured definitions | E-commerce agent (sock-shop) |
| **Agno** | Tool orchestration | GitHub issue summarizer |
| **Minions** | Hybrid local-remote | Cost-optimized reasoning |

**Three-file configuration strategy:**
- `compose.yaml` — base services
- `compose.dmr.yaml` — Docker Model Runner (local GPU inference)
- `compose.openai.yaml` — OpenAI API override

This enables switching between local and cloud LLM providers without changing agent code.

### 5.2 OpenHands (formerly OpenDevin)

Architecture:
- Agents access containers via **SSH**, preserving human-initiated remote development semantics
- Only project/task-specific files exposed via workspace mounting
- Containers torn down post-session for filesystem integrity
- 68.6k+ GitHub stars, $18.8M Series A funding
- Containerized evaluation for SWE-Bench: per-task sandbox containers with pre-configured testbeds

**Key pattern:** SSH-based agent-to-container communication means agents use the same tools and workflows as human developers. No special agent API required.

### 5.3 SWE-ReX (from SWE-agent)

[SWE-ReX](https://github.com/SWE-agent/SWE-ReX) provides runtime abstraction for sandboxed execution:
- Agent code unchanged regardless of backend (local, Docker, AWS, Modal, Fargate)
- Massively parallel: demonstrated running 30 SWE-bench instances simultaneously
- Multiple parallel shell sessions within a single environment
- Support for interactive CLI tools (ipython, gdb)

**Key pattern:** Runtime abstraction layer — agents don't know or care whether they're running in Docker, on bare metal, or in a cloud VM. The execution environment is pluggable.

### 5.4 Cerebras + Docker Compose Secure Agent

```yaml
mcp-gateway:
  image: docker/mcp-gateway:latest
  command:
    - --transport=sse
    - --servers=context7,node-code-sandbox
    - --catalog=/mcp-gateway-catalog.yaml
```

Sandbox container created via Testcontainers with `withNetworkMode("none")` to disable all network access, preventing data exfiltration while allowing other tools (documentation servers) to retain connectivity.

**Key pattern:** Per-tool network isolation. Code execution sandboxes have no network; documentation tools have full network. The MCP Gateway routes to both transparently.

**Application to ORCHA:** The three-file configuration strategy (base + local-inference + cloud-inference) maps directly to our need for environment-aware LLM routing. Agents write code in sandboxed containers; the orchestrator chooses between local and cloud LLM providers based on task complexity and cost tier.

---

## 6. Docker-in-Docker and Sandboxed Execution

### 6.1 Docker Sandboxes (November 2025)

**Architecture**: MicroVM-based isolation (macOS/Windows). Each sandbox runs in a lightweight microVM with a **private Docker daemon**, completely separate from the host Docker daemon.

**Security model:**
- Agents cannot access host Docker daemon, containers, or files outside the workspace
- Sensitive directories (SSH keys, AWS credentials, Documents) inaccessible
- Workspace synced at identical absolute paths (host `/Users/alice/projects/myapp` mirrors exactly inside sandbox)
- Template images run as non-root with sudo access
- Claude Code launched with `--dangerously-skip-permissions` by default inside sandbox

**CLI:**
```bash
docker sandbox run claude ~/my-project    # Create/reuse sandbox
docker sandbox ls                          # List running sandboxes
docker sandbox run --mount-docker-socket   # Enable Docker-in-Docker
docker sandbox run -e VAR=value            # Pass env vars
docker sandbox rm <id>                     # Remove sandbox
```

**State persistence**: Installed packages and dependencies persist across sessions. Git `user.name`/`user.email` auto-injected. Docker credentials stored in volumes.

**One sandbox per workspace**: Running `docker sandbox run` reuses existing containers in the same directory.

### 6.2 DinD vs Docker Socket vs Sysbox vs MicroVM

| Approach | Security | Use Case | Trade-offs |
|----------|----------|----------|------------|
| **DinD** (`--privileged`) | Low | CI/CD, testing | Privileged mode grants host-level access |
| **Docker Socket Mount** | Medium | Agent needs Docker commands | Can control host containers |
| **Sysbox** | High | Nested containers unprivileged | Additional runtime dependency |
| **MicroVM** (Docker Sandboxes) | Highest | Production agent sandboxing | macOS/Windows only, experimental |

Docker is moving from container-based to **microVM-based** sandboxing for defense-in-depth, with plans to improve Docker-in-Docker experience within sandboxes.

### 6.3 Workspace Isolation Patterns

Three levels of workspace isolation for coding agents:

1. **Shared volume** — All agents read/write the same mounted directory. Simple but creates conflicts.
2. **Copy-on-write** — Each agent gets a filesystem snapshot (Docker layer caching or overlay mounts). Agents see the same base but their writes are isolated.
3. **Full isolation** — Each agent gets its own workspace volume. Results merged by the orchestrator. Most isolated but requires explicit merge strategy.

**Application to ORCHA:** For coding agents, the copy-on-write pattern via Git worktrees inside containers provides the best balance — agents share the same repo but work on isolated branches. Docker Sandboxes with workspace mounting give us microVM-level isolation while maintaining absolute path compatibility.

---

## 7. Observability and Logging in Containerized Agent Systems

### 7.1 LGTM Stack (Loki + Grafana + Tempo + Metrics)

**Architecture**: Application → OpenTelemetry Collector → Routes to:
- **Tempo** (traces via gRPC)
- **Loki** (logs via HTTP)
- **Prometheus** (metrics via scrape endpoint)
- **Grafana** connects to all three for unified dashboards

The OpenTelemetry Collector implements three pipelines with memory limiting (512 MiB) and batching (5-second timeouts). Prometheus scrapes the Collector at 15-second intervals.

OpenTelemetry reached full maturity in 2025: GA status for all telemetry signals (metrics, traces, logs), auto-instrumentation for 20+ languages including Rust.

### 7.2 Distributed Tracing

Jaeger traces requests as they flow through multiple agent services, visualizing dependencies and identifying performance bottlenecks. A single task flowing through plan → code → test → review agents generates a trace spanning all four services.

### 7.3 Agent Fleet Metrics

Key metrics for multi-agent orchestration monitoring:
- Task completion rates and durations per agent type
- LLM API call latency, error rates, and token usage
- Container resource utilization (CPU, memory, GPU)
- Task queue depth and processing throughput
- Agent health check status and uptime
- Cost per task (LLM tokens × price)

### 7.4 Compose-Native Observability Stack

The entire LGTM stack is deployable as Docker Compose services:

```yaml
services:
  otel-collector:
    image: otel/opentelemetry-collector-contrib:latest
    volumes:
      - ./otel-config.yaml:/etc/otelcol-contrib/config.yaml
    ports:
      - "4317:4317"  # gRPC
      - "4318:4318"  # HTTP

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3001:3000"
    volumes:
      - grafana-data:/var/lib/grafana

  loki:
    image: grafana/loki:latest

  tempo:
    image: grafana/tempo:latest

  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
```

**Application to ORCHA:** The LGTM stack as Compose services provides everything needed for agent fleet observability without external infrastructure. OpenTelemetry auto-instrumentation means our Rust/Axum backend can emit traces with the `tracing-opentelemetry` crate. Custom Grafana dashboards visualize agent throughput, error rates, cost, and resource consumption.

---

## 8. Production Patterns and Best Practices

### 8.1 When to Graduate from Compose to Kubernetes

| Stay with Docker Compose | Migrate to Kubernetes |
|--------------------------|----------------------|
| Local development and testing | Production high availability |
| Single-host deployments | Multi-host orchestration |
| Small-scale staging | Dynamic auto-scaling |
| Teams under 10 engineers | RBAC and network policies |
| Simple agent topologies | Complex microservice interdependencies |

**Conversion path**: Docker's experimental **Compose Bridge** converts Compose files to Kubernetes manifests or Helm charts. Services → Deployments (stateless) or StatefulSets (databases). Volumes → PersistentVolumeClaims. Environment variables split into ConfigMaps (non-sensitive) and Secrets (sensitive).

### 8.2 Docker Compose Watch for Development

```yaml
develop:
  watch:
    - action: sync
      path: ./src
      target: /app/src
      ignore:
        - node_modules/
    - action: rebuild
      path: package.json
    - action: sync+restart
      path: ./config
      target: /app/config
```

Three actions: `sync` (hot reload for interpreted languages), `rebuild` (compiled languages, dependency changes), `sync+restart` (config changes). Launch: `docker compose up --watch`.

**Application to ORCHA:** During development, `compose watch` with `sync` action on agent prompt templates enables hot-reloading prompt changes without container rebuilds. The `rebuild` action triggers for Rust code changes.

### 8.3 Persistent Storage Patterns

```yaml
volumes:
  db-data:           # Named volume (Docker-managed, production-ready)
  agent-workspace:   # Per-agent workspace data
  vector-store:      # Embedding database persistence
```

Named volumes stored in `/var/lib/docker/volumes/`, platform-independent. Bind mounts (`./data:/data`) suitable for development; named volumes for production.

### 8.4 Blue/Green Deployments

Pattern: Run two copies (blue + green). Deploy update to inactive copy, health-check it, then switch load balancer traffic. Use `docker compose stop -t 90` to allow in-flight agent tasks to complete before stopping the old version.

**Application to ORCHA:** Blue/green deployments enable zero-downtime agent updates. The 90-second grace period is critical — coding agents may be mid-task and need time to complete or checkpoint their work before shutdown.

### 8.5 Production Checklist

1. Use official/verified base images
2. Set memory and CPU limits on every service
3. Configure health checks for all services
4. Use `restart: unless-stopped` for persistent services
5. Use secrets (file-mounted), not environment variables, for credentials
6. Direct logs to external systems (not `docker logs`)
7. Use named volumes for all persistent data
8. Pin image versions (never use `:latest` in production)
9. Use multi-stage builds to minimize image size
10. Scan images for vulnerabilities

**Compose limitations**: No multi-host orchestration, no automatic failover, no rolling updates, no service mesh networking. These require Kubernetes or Docker Swarm.

---

## 9. Comparison: Docker/Compose Patterns vs ORCHA Patterns

| Dimension | Docker/Compose Best Practices | ORCHA (Current) | Gap/Opportunity |
|-----------|------------------------------|-----------------|-----------------|
| **Agent isolation** | Container-per-agent with resource limits, network isolation, filesystem isolation | Agents run as in-process executors sharing server resources | Containerize agent execution for isolation + resource limits |
| **Workspace isolation** | Docker Sandboxes (microVM), Git worktrees in containers, copy-on-write volumes | Git worktrees managed by WorktreeManager service | Add container-level isolation around worktree execution |
| **Service architecture** | Service-per-agent with shared infrastructure (DB, queue, MCP gateway) | Monolithic server with in-process agents | Decompose into orchestrator service + agent worker services + shared infra |
| **Code execution sandboxing** | MicroVM sandboxes, network-disabled containers, ephemeral per-task | Agent code runs with server process permissions | Add sandboxed execution containers for agent-generated code |
| **Inter-agent communication** | DNS service discovery + REST/gRPC + message queues + shared volumes | Direct function calls (in-process) | Add message queue for async task distribution when containerized |
| **MCP server management** | MCP Gateway consolidates servers, per-tool network isolation | Individual MCP server connections per agent | Add MCP Gateway service for centralized access + isolation |
| **Health monitoring** | Container health checks with `start_period`, readiness probes | No agent health monitoring | Add health check endpoints for agent services |
| **Resource management** | Per-container CPU/memory limits, GPU allocation | No resource limits on agent execution | Add resource budgets per agent task |
| **Environment management** | Profiles (dev/staging/prod), override files, secrets mount | `.env` file + flox environment | Add Docker Compose profiles for multi-environment support |
| **Observability** | LGTM stack (Loki/Grafana/Tempo/Metrics) via OpenTelemetry | Basic execution logs, Rust tracing | Add OpenTelemetry integration + Grafana dashboards |
| **Scaling** | `--scale worker=N`, KEDA queue-based auto-scaling | Single agent instance per task | Add agent pool scaling based on task queue depth |
| **Deployment** | Blue/green with graceful shutdown, Compose Bridge → K8s | Development only | Add containerized deployment pipeline |
| **Init/setup** | Init container pattern for migrations, seeding, pre-warming | Manual setup via flox activate | Add init containers for automated environment setup |
| **LLM provider routing** | Three-file config (base + local-inference + cloud-inference) | Hardcoded provider selection | Add environment-based LLM provider switching |

---

## 10. Actionable Recommendations for ORCHA

### High Priority

1. **Containerized Agent Execution** — Wrap each coding agent execution in a Docker container with: resource limits (memory, CPU), network isolation (internal network only), workspace volume mount (Git worktree), and ephemeral lifecycle (destroy after task completion). This provides the isolation guarantees our OTP-style supervision hierarchy needs — a crashed or misbehaving agent can't affect other agents or the orchestrator. Use the service-per-agent-type pattern: one container image per agent type (Claude, Gemini, etc.), spawned per task.

2. **MCP Gateway Service** — Deploy Docker's MCP Gateway as a shared service that consolidates all MCP server connections. Agent containers connect to the gateway endpoint rather than individual MCP servers. This enables: centralized access control (which agents can use which tools), per-tool network isolation (code sandbox has no internet; documentation tools do), and simplified agent container configuration (one endpoint instead of many).

3. **Sandboxed Code Execution** — Agent-generated code must execute in isolated containers, not in the server process. Use Docker Sandboxes (microVM) or `withNetworkMode("none")` containers for code execution. The agent writes code to its workspace volume; a sandboxed executor container runs it and returns results. This prevents agent-generated code from: accessing the host filesystem, making unauthorized network calls, or consuming unbounded resources.

4. **Docker Compose Service Architecture** — Define the full ORCHA stack as a Docker Compose configuration: orchestrator service, agent worker pool, SQLite/PostgreSQL database, message queue (Redis/RabbitMQ for task distribution), MCP Gateway, and optional observability stack (LGTM). This makes the entire system portable, reproducible, and deployable with a single `docker compose up`.

### Medium Priority

5. **Profile-Based Environment Configuration** — Use Docker Compose profiles to manage different deployment modes: `--profile dev` (local development with hot-reload), `--profile gpu` (local LLM inference), `--profile monitoring` (LGTM observability stack), `--profile prod` (production settings with resource limits and health checks). Three-file LLM provider strategy: `compose.yaml` (base) + `compose.local-llm.yaml` (Docker Model Runner) + `compose.cloud-llm.yaml` (API provider).

6. **Health Check Infrastructure** — Every agent service exposes a health endpoint verifying: LLM API connectivity, database connection, workspace filesystem access, and MCP server availability. The orchestrator only dispatches tasks to healthy agents. Health checks use `start_period` for graceful startup and `depends_on: condition: service_healthy` for dependency ordering.

7. **OpenTelemetry Integration** — Add `tracing-opentelemetry` to the Rust backend. Emit traces, metrics, and logs to an OpenTelemetry Collector running as a Compose service. Route to Grafana (dashboards), Tempo (traces), Loki (logs), and Prometheus (metrics). Key agent metrics: task completion rate, LLM API latency, token usage, cost per task, error rates, queue depth.

8. **Init Container Chain for Setup** — Replace manual `flox activate` + DB migration with an init container chain: `init-migrate → init-seed → init-verify → orchestrator`. Uses `condition: service_completed_successfully` and `restart: "no"`. Failures in any init step prevent the system from starting in a broken state. This makes deployment fully automated and reproducible.

### Lower Priority

9. **Message Queue for Task Distribution** — When agents are containerized, replace direct function calls with a Redis/RabbitMQ task queue. The orchestrator publishes tasks; agent containers consume from the queue. Enables: natural load balancing across multiple agent instances, task persistence across restarts, and dead letter queues for failed tasks. Maps to the blackboard pattern (MAS research) implemented via message broker.

10. **Agent Pool Auto-Scaling** — Scale agent container count based on task queue depth. For Compose: `docker compose up --scale worker=N`. For Kubernetes: KEDA with queue depth triggers. Target: `desired_replicas = queue_depth / tasks_per_agent`. Scale to zero when idle for cost optimization.

11. **Blue/Green Agent Deployments** — When updating agent images (new prompts, tool versions, model versions), deploy the update to inactive containers, health-check them, then switch traffic. Use `docker compose stop -t 90` to allow in-flight agent tasks 90 seconds to complete or checkpoint. This enables zero-downtime agent updates.

12. **Compose Bridge to Kubernetes Migration Path** — Design Compose configurations with K8s migration in mind: use named volumes (→ PVCs), secrets (→ K8s Secrets), health checks (→ readiness/liveness probes), resource limits (→ resource requests/limits). When scale demands exceed single-host capacity, use Docker's Compose Bridge for automated conversion.

---

## Key Takeaways

1. **Container-per-agent is the industry standard** — Docker's official compose-for-agents repository, OpenHands, SWE-agent, and Cerebras all use container isolation per agent. The pattern provides resource limits, filesystem isolation, network segmentation, and clean lifecycle management. It's the "service-per-agent" equivalent of microservices architecture.

2. **MCP Gateway consolidates tool access** — Rather than each agent independently connecting to MCP servers, a gateway service provides unified access control and per-tool network isolation. Code sandboxes get no network; documentation tools get full network. The gateway routes transparently.

3. **Sandboxed execution is non-negotiable for coding agents** — Agent-generated code must run in isolated containers. Docker Sandboxes (microVM) provide the strongest guarantee: private Docker daemon, no host filesystem access, configurable network isolation. The security model is "default deny, explicit allow."

4. **Docker Compose profiles replace environment branching** — Instead of maintaining separate configurations per environment, use profiles: `--profile dev`, `--profile gpu`, `--profile monitoring`. Services without profiles always start; profiled services activate on demand. Override files layer production settings on top.

5. **`service_completed_successfully` enables pipeline orchestration** — Standard `depends_on` only waits for startup. The `condition: service_completed_successfully` flag enables true sequential pipeline execution — critical for init containers and agent workflow stages.

6. **The LGTM stack deploys alongside agents** — Loki, Grafana, Tempo, and Prometheus as Compose services provide full observability without external infrastructure. OpenTelemetry auto-instrumentation means agents emit traces with minimal code changes.

7. **Three-file LLM provider strategy** — Base configuration + local inference override + cloud inference override. Switch providers without changing agent code: `docker compose -f compose.yaml -f compose.local-llm.yaml up`.

8. **Compose is sufficient until you need multi-host** — Docker Compose handles single-host deployments effectively. Graduate to Kubernetes only when you need: multi-host orchestration, automatic failover, RBAC, or dynamic auto-scaling beyond `--scale`. Compose Bridge eases the transition.

9. **Ephemeral containers prevent contamination** — Agent containers created per-task and destroyed after completion ensure that state from one task (including any injected malicious code) can't affect the next task. This is stronger isolation than process-level separation.

10. **Health checks gate task dispatch** — Agent services expose health endpoints; the orchestrator only dispatches to healthy agents. `start_period` prevents false failures during initialization. This is the container-native implementation of the circuit breaker pattern from MAS research.

---

## Sources

### Docker Official
- [Docker Brings Compose to the AI Agent Era](https://www.docker.com/blog/build-ai-agents-with-docker-compose/)
- [Docker Docs: Agentic AI Applications](https://docs.docker.com/guides/agentic-ai/)
- [Docker Sandboxes Documentation](https://docs.docker.com/ai/sandboxes/)
- [Docker Blog: Cerebras + Docker Compose Secure AI Coding Agents](https://www.docker.com/blog/cerebras-docker-compose-secure-ai-coding-agents/)
- [Docker Blog: Docker Sandboxes for Coding Agent Safety](https://www.docker.com/blog/docker-sandboxes-a-new-approach-for-coding-agent-safety/)
- [Docker Blog: Docker MCP for AI Agents](https://www.docker.com/blog/docker-mcp-ai-agent-developer-setup/)
- [Docker Docs: GPU Support](https://docs.docker.com/compose/how-tos/gpu-support/)
- [Docker Docs: Compose Watch](https://docs.docker.com/compose/how-tos/file-watch/)
- [Docker Docs: Secrets in Compose](https://docs.docker.com/compose/how-tos/use-secrets/)
- [GitHub: docker/compose-for-agents](https://github.com/docker/compose-for-agents)

### Multi-Agent Deployments
- [freeCodeCamp: Build and Deploy Multi-Agent AI with Python and Docker](https://www.freecodecamp.org/news/build-and-deploy-multi-agent-ai-with-python-and-docker/)
- [DEV Community: Building Autonomous AI Agents with Docker](https://dev.to/docker/building-autonomous-ai-agents-with-docker-how-to-scale-intelligence-3oi)
- [OpenHands Agent Framework](https://www.emergentmind.com/topics/openhands-agent-framework)
- [GitHub: SWE-agent/SWE-ReX](https://github.com/SWE-agent/SWE-ReX)
- [GitHub: typper-io/ai-code-sandbox](https://github.com/typper-io/ai-code-sandbox)
- [Docker Sandbox for LLM Coding Agents (hartleybrody.com)](https://blog.hartleybrody.com/docker-sandbox/)

### Communication Protocols
- [A2A Protocol (Google)](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)
- [A2A Protocol Official Site](https://a2a-protocol.org/latest/)
- [IBM: Agent2Agent Protocol](https://www.ibm.com/think/topics/agent2agent-protocol)
- [MCP, ACP, A2A, ANP Survey (arxiv)](https://arxiv.org/html/2505.02279v1)

### Infrastructure Patterns
- [Docker Compose Init Container Pattern](https://oneuptime.com/blog/post/2026-02-08-how-to-use-docker-compose-init-containers-pattern/view)
- [Docker Container Communication Architecture](https://oneuptime.com/blog/post/2026-02-08-how-to-plan-docker-container-communication-architecture/view)
- [Docker Compose Observability Stack](https://oneuptime.com/blog/post/2026-02-06-docker-compose-observability-stack/view)
- [Docker Compose Profiles](https://oneuptime.com/blog/post/2025-11-27-manage-docker-compose-profiles/view)
- [Scaling AI Agents with Docker MCP Gateway and Offload](https://dasroot.net/posts/2026/03/scaling-ai-agents-docker-mcp-gateway-docker-offload/)

### Production & Deployment
- [freeCodeCamp: Docker Compose for Production Workloads](https://www.freecodecamp.org/news/how-to-use-docker-compose-for-production-workloads/)
- [Migrating Docker Compose to Kubernetes](https://dasroot.net/posts/2026/03/migrating-docker-compose-kubernetes-helm-kustomize/)
- [Spacelift: Docker Compose vs Kubernetes](https://spacelift.io/blog/docker-compose-vs-kubernetes)
- [Blue-Green Deployment with Docker Compose](https://technicallyshane.com/2025/08/30/blue-green-deployment-of-a-docker-compose-setup.html)
- [Dokploy: Deploy with Docker Compose in 2025](https://dokploy.com/blog/how-to-deploy-apps-with-docker-compose-in-2025)

### Scaling & GPU
- [KEDA: Kubernetes Event-Driven Autoscaling](https://keda.sh/)
- [Sharing GPU Access Across Containers](https://dasroot.net/posts/2026/02/sharing-gpu-access-across-containers/)
- [Docker Compose Secrets Best Practices (Bitdoze)](https://www.bitdoze.com/docker-compose-secrets/)
