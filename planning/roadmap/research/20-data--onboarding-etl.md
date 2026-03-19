# Data Onboarding, ETL, and Data Pipeline Research

**Date**: 2026-03-19
**Scope**: Client data import, ETL/ELT frameworks, data quality, webhook/event processing, file processing, and multi-tenant data migration for ORCHA
**Context**: ORCHA is a Rust/Axum + React AI orchestration SaaS platform with CRM, project management, and workflow automation. Currently SQLite, migrating to PostgreSQL. Existing data sources support file upload and text/JSON input but lack structured ETL pipelines.

---

## Table of Contents

1. [Data Import/ETL Libraries (Rust)](#1-data-importetl-libraries-rust)
2. [Data Integration Connectors](#2-data-integration-connectors)
3. [ETL/ELT Frameworks](#3-etlelt-frameworks)
4. [Data Quality Tools](#4-data-quality-tools)
5. [Webhook/Event Processing](#5-webhookevent-processing)
6. [File Processing](#6-file-processing)
7. [Data Migration for Multi-Tenant](#7-data-migration-for-multi-tenant)
8. [Recommendation Matrix](#8-recommendation-matrix)
9. [Implementation Roadmap](#9-implementation-roadmap)

---

## 1. Data Import/ETL Libraries (Rust)

### 1.1 `csv` crate

- **License**: MIT/Unlicense (dual)
- **What it does**: High-performance CSV reader/writer with Serde integration. Handles quoting, escaping, different delimiters, and streaming row-by-row parsing.
- **Pros**:
  - Extremely mature (100M+ downloads on crates.io), maintained by BurntSushi (ripgrep author)
  - Zero-copy parsing with `ByteRecord` for maximum throughput
  - Serde derive for direct deserialization into Rust structs
  - Streaming API — can process files larger than RAM
  - Handles encoding edge cases (BOM detection, line ending normalization)
- **Cons**:
  - No built-in schema inference (you define the target struct)
  - No data validation beyond type deserialization errors
  - No header normalization (whitespace, case)
- **Roadmap stage fit**: **Stage 0-1** — CSV import is the most requested data onboarding feature for any B2B SaaS. Essential for CRM data migration from spreadsheets.
- **Rationale**: Every pilot client will have CRM data in CSV/Excel. This is table stakes for onboarding.
- **Budget/timeline**: 0 cost, 1-2 days to integrate for basic import, 1 week for schema mapping UI.

### 1.2 `calamine` crate

- **License**: MIT
- **What it does**: Reads Excel files (.xlsx, .xls, .xlsb, .ods). Extracts cell values with type information (string, float, int, bool, datetime, duration, error, empty).
- **Pros**:
  - Pure Rust, no external dependencies (no libxlsx, no Python)
  - Supports all major spreadsheet formats including legacy .xls
  - Memory-efficient — can iterate rows without loading entire sheet
  - Returns typed cell values (not just strings), preserving Excel data types
  - Handles multi-sheet workbooks — can list and select sheets
- **Cons**:
  - Read-only (cannot write Excel files — use `rust_xlsxwriter` or `xlsxwriter` for export)
  - No formula evaluation (reads cached formula results only)
  - Date handling requires careful timezone treatment (Excel epoch differs)
  - Merged cell handling can be tricky — only the top-left cell holds the value
- **Roadmap stage fit**: **Stage 0-1** — Same rationale as CSV. Agencies live in spreadsheets.
- **Rationale**: Paired with `csv`, covers 90%+ of "I have data in a file" onboarding scenarios.
- **Budget/timeline**: 0 cost, 1-2 days to integrate alongside CSV import.

### 1.3 Schema Mapping Patterns

**The Problem**: When importing CSV/Excel, source columns rarely match target schema exactly. "First Name" vs "first_name" vs "fname" — users need a UI to map source fields to ORCHA CRM/project fields.

**Recommended Pattern — Two-Phase Import**:

1. **Upload & Preview**: Parse first N rows (50-100), display in table. Show inferred column types.
2. **Schema Mapping UI**: Side-by-side mapping interface. Left column: source headers. Right column: dropdown of ORCHA target fields. Auto-suggest mappings using:
   - Exact match (case-insensitive)
   - Fuzzy match (Levenshtein distance, Jaro-Winkler similarity)
   - Common synonym tables ("fname" → "first_name", "company" → "company_name")
3. **Validation Preview**: Show sample transformed data before committing. Highlight rows with errors (missing required fields, type mismatches).
4. **Execute Import**: Bulk insert with progress bar, error log for rejected rows.

**UI Reference Implementations**:
- Flatfile.com — best-in-class import UX (commercial, $500+/mo, not recommended to embed)
- Papa Parse (JS) + react-spreadsheet-import — open-source React component for CSV import with column mapping
- OneSchema — similar to Flatfile, embedded import widget

**For ORCHA**: Build a custom `ImportWizard` component using the existing `Dialog` component system. The schema mapping step is the core UX investment. Target fields come from the CRM contact/deal/company schemas and workflow output schemas (already defined in the workflow editor).

**Fuzzy matching in Rust**: `strsim` crate (MIT) provides Jaro-Winkler, Levenshtein, Sorensen-Dice for column name auto-matching.

### 1.4 Bulk Import Performance

**Pattern**: For imports > 1,000 rows, avoid row-by-row INSERT:

```
Approach               | Rows/sec (SQLite) | Rows/sec (PostgreSQL)
-----------------------|-------------------|----------------------
Row-by-row INSERT      | ~500              | ~1,000
Batched INSERT (100)   | ~15,000           | ~30,000
COPY (PostgreSQL)      | N/A               | ~200,000+
Transaction wrapping   | ~50,000 (SQLite)  | ~50,000
```

**SQLite optimization** (current): Wrap bulk inserts in a single transaction. Use prepared statements. `INSERT INTO ... VALUES (...), (...), (...)` with 100-500 row batches.

**PostgreSQL optimization** (future): Use `COPY FROM STDIN` via `sqlx` or `tokio-postgres` for maximum throughput. For SQLx, the `PgCopy` API supports binary COPY protocol.

**Recommendation**: Implement a `BulkImporter` service in `crates/services/` that abstracts the batch insert pattern. Returns a stream of progress events for SSE to frontend.

### 1.5 Incremental Sync vs Full Import

**Full Import**: User uploads file, entire dataset replaces or appends. Simple, no state to track. Best for initial onboarding and one-off imports.

**Incremental Sync**: Ongoing connection to external system (Salesforce, HubSpot). Tracks last sync timestamp or change cursor. Only pulls new/modified/deleted records.

**Patterns**:
- **Timestamp-based**: Query `WHERE updated_at > last_sync_time`. Simple but can miss deletes.
- **Cursor/Offset-based**: APIs return a cursor token for pagination. Salesforce uses this pattern.
- **Webhook-based**: External system pushes changes in real-time. Most efficient but requires webhook infrastructure.
- **Change Data Capture (CDC)**: Database-level change tracking. Best for self-hosted databases. PostgreSQL logical replication can provide this.
- **Hash-based**: Compute hash of each record, compare with stored hashes to detect changes. Works when source has no change tracking.

**Recommendation**: Stage 0-1 = full import only (CSV/Excel upload). Stage 2+ = incremental sync via API connectors with webhook support where available.

---

## 2. Data Integration Connectors

### 2.1 CRM Connectors

#### Salesforce API

- **API Pattern**: REST API v59+ with OAuth 2.0 (JWT bearer or web server flow). Bulk API 2.0 for large datasets (async job-based, returns CSV). SOQL for queries. Change Data Capture via Pub/Sub API (gRPC).
- **Rust approach**: `reqwest` + custom OAuth token management. No mature Rust SDK exists. Build thin wrapper around REST endpoints.
- **Key objects**: Account, Contact, Lead, Opportunity, Task, Event, Custom Objects
- **Rate limits**: 100,000 API calls/24hr (Enterprise), 15,000/24hr (Professional). Bulk API has separate 15,000 batch limit.
- **Pros**: Largest CRM market share, comprehensive API, well-documented
- **Cons**: Complex authentication, aggressive rate limits, expensive licensing for API access
- **Roadmap stage fit**: **Stage 2** — Salesforce orgs are larger enterprises. Stage 1 pilots are agencies who likely use HubSpot/Pipedrive.
- **Budget/timeline**: 2-3 weeks for connector, $0 (API access included in Salesforce licenses)

#### HubSpot API

- **API Pattern**: REST API v3 with OAuth 2.0 or API key. Batch endpoints for bulk operations. Webhook subscriptions for real-time changes. CRM Search API with powerful filtering.
- **Rust approach**: `reqwest` + OAuth. No Rust SDK but API is simpler than Salesforce.
- **Key objects**: Contacts, Companies, Deals, Tickets, Engagements, Custom Objects
- **Rate limits**: 100 requests/10 seconds (OAuth), 200 requests/10 seconds (private apps). Generous for most use cases.
- **Pros**: Free tier available, well-designed REST API, excellent documentation, popular with agencies
- **Cons**: Rate limits tighter per-second than Salesforce, association model can be confusing
- **Roadmap stage fit**: **Stage 1** — High priority. Agencies (ORCHA's initial market) heavily use HubSpot.
- **Budget/timeline**: 1-2 weeks for connector, $0 (free developer account for testing)

#### Pipedrive API

- **API Pattern**: REST API v1 with OAuth 2.0 or API token. Webhook subscriptions. Straightforward CRUD.
- **Key objects**: Persons, Organizations, Deals, Activities, Leads, Products
- **Rate limits**: 80 requests/2 seconds (Essential), 160/2s (Advanced+)
- **Pros**: Simple, clean API. Popular with small agencies. Direct mapping to ORCHA CRM concepts.
- **Cons**: Smaller market share, less customization than Salesforce/HubSpot
- **Roadmap stage fit**: **Stage 1** — Good fit for agency pilots alongside HubSpot.
- **Budget/timeline**: 1 week for connector, $0

### 2.2 Project Management Connectors

#### Jira (Atlassian) API

- **API Pattern**: REST API v3 with OAuth 2.0 (Atlassian Connect or Forge). JQL for queries. Webhooks for events.
- **Key objects**: Issues (tasks), Projects, Boards, Sprints, Comments, Attachments
- **Rate limits**: Rate limiting per user token, typically 100-200 requests/minute
- **Pros**: Largest project management install base, comprehensive API
- **Cons**: Complex API surface, Atlassian auth is notoriously tricky, API versioning has been painful
- **Roadmap stage fit**: **Stage 2** — Useful when onboarding enterprise clients who already have Jira
- **Budget/timeline**: 2-3 weeks, $0 (free developer instance via Atlassian)

#### Linear API

- **API Pattern**: GraphQL API with OAuth 2.0 or API key. Webhooks for events. Clean, modern API design.
- **Key objects**: Issues, Projects, Cycles, Teams, Comments, Labels
- **Rate limits**: 1,500 requests/hour (simple), 250 requests/hour (complex/mutations)
- **Pros**: Modern GraphQL API, excellent developer experience, popular with tech-forward agencies
- **Cons**: Smaller market share than Jira/Asana
- **Roadmap stage fit**: **Stage 1-2** — Good for tech-forward pilot clients
- **Budget/timeline**: 1 week, $0

#### Asana API

- **API Pattern**: REST API with OAuth 2.0 or PAT. Event-based webhooks. Batch endpoints available.
- **Key objects**: Tasks, Projects, Sections, Portfolios, Goals, Custom Fields
- **Rate limits**: 1,500 requests/minute (free), higher on paid plans
- **Pros**: Popular with creative agencies (ORCHA's target), generous rate limits, good webhooks
- **Cons**: API quirks (membership/follower model), nested task hierarchy can be complex
- **Roadmap stage fit**: **Stage 1** — High priority for agency market
- **Budget/timeline**: 1-2 weeks, $0

### 2.3 Communication Connectors

#### Slack API

- **API Pattern**: REST + Events API + Socket Mode. OAuth 2.0 with granular scopes. Rich message format (Block Kit).
- **Import use case**: Channel history, message threads, file attachments. Useful for ingesting client communication context into ORCHA CRM/project timeline.
- **Rate limits**: Tier 1 (1/min) to Tier 4 (100+/min) depending on method. Special rate limits for message posting.
- **Pros**: Ubiquitous in agencies, good webhook support, Socket Mode avoids public URL requirement
- **Cons**: Scope proliferation (hundreds of scopes), message export requires admin approval, complex event subscription model
- **Roadmap stage fit**: **Stage 1-2** — Notification integration (Stage 1), full data import (Stage 2)
- **Budget/timeline**: 1 week for notifications, 2-3 weeks for full history import, $0

#### Discord API

- **Already partially integrated** via `crates/discord-bot` and `crates/discord-bots`.
- **Existing**: Bot framework with message handling. Can be extended for data import.
- **Roadmap stage fit**: **Stage 0** — Already in codebase, extend as needed.

#### Microsoft Teams API (Graph API)

- **API Pattern**: Microsoft Graph REST API with OAuth 2.0 (Azure AD). Webhooks via change notifications.
- **Import use case**: Channel messages, chat history, meeting transcripts, files.
- **Pros**: Required for enterprise clients, comprehensive data access via Graph
- **Cons**: Azure AD auth is complex, Graph API rate limits vary by endpoint and tenant, admin consent often required
- **Roadmap stage fit**: **Stage 2-3** — Enterprise feature, not needed for agency pilots
- **Budget/timeline**: 3-4 weeks (Azure AD auth is the time sink), $0 (free developer tenant)

### 2.4 Email Connectors

#### Gmail API

- **API Pattern**: REST API with OAuth 2.0, Google Cloud project required. Push notifications via Pub/Sub. Batch requests supported.
- **Import use case**: Email threads associated with CRM contacts/deals. Parse sender/recipient for relationship mapping.
- **Rate limits**: 250 quota units/sec/user, varies by method
- **Pros**: Massive user base, well-documented, push notifications for real-time
- **Cons**: Google OAuth scope review process is extensive (can take weeks for sensitive scopes like mail.readonly), privacy concerns
- **Roadmap stage fit**: **Stage 2** — Email CRM enrichment is powerful but not MVP-critical
- **Budget/timeline**: 2 weeks for connector, 2-4 weeks for Google OAuth verification, $0

#### Microsoft Outlook (Graph API)

- **API Pattern**: Same Microsoft Graph as Teams. Mail-specific endpoints with rich filtering, delta queries for incremental sync.
- **Import use case**: Same as Gmail — email threads for CRM context.
- **Pros**: Enterprise standard, delta query for efficient sync
- **Cons**: Same Azure AD complexity as Teams
- **Roadmap stage fit**: **Stage 2-3** — Bundled with Teams connector work
- **Budget/timeline**: 1 week incremental over Teams connector, $0

### 2.5 File Storage Connectors

#### Dropbox

- **Already integrated** — `crates/pcg-sync` handles Dropbox sync, 45K+ files indexed. `data_sources` table tracks Dropbox files.
- **Existing pattern**: One-way sync with file classification/enrichment pipeline.
- **Roadmap stage fit**: **Stage 0** — Already operational.

#### Google Drive API

- **API Pattern**: REST API v3 with OAuth 2.0. Push notifications via webhooks. Supports incremental sync via changes API with start page token.
- **Import use case**: Client deliverables, shared folders, document collaboration. Parse Google Docs/Sheets for content extraction.
- **Pros**: Widely used, good change notification API, native format export (Docs → PDF/DOCX)
- **Cons**: OAuth scope verification (sensitive for drive access), complex permission model
- **Roadmap stage fit**: **Stage 1-2** — Many agency clients share files via Google Drive
- **Budget/timeline**: 2 weeks, $0

#### OneDrive (Graph API)

- **API Pattern**: Microsoft Graph, delta queries for efficient sync, rich permissions model.
- **Import use case**: Same as Google Drive for Microsoft-centric orgs.
- **Roadmap stage fit**: **Stage 2-3** — Bundled with other Graph API work
- **Budget/timeline**: 1 week incremental over existing Graph API work, $0

---

## 3. ETL/ELT Frameworks

### 3.1 Apache Arrow / DataFusion

- **License**: Apache 2.0
- **What it does**: Arrow provides a columnar in-memory format with zero-copy reads. DataFusion is a SQL query engine built on Arrow. Together, they enable SQL-based data transformation in Rust with excellent performance.
- **Rust ecosystem**: `arrow` crate (1.2M+ downloads), `datafusion` crate (mature, used in production by InfluxDB, Coralogix, Databend).
- **Pros**:
  - Native Rust — no FFI, no Python subprocess
  - SQL interface for transformations (DataFusion) — non-engineers can write transform queries
  - Columnar format is 10-100x faster than row-based for analytics/aggregation
  - Parquet read/write support (efficient storage for data exports)
  - Growing ecosystem — DataFusion-based projects (Ballista for distributed, GlueSQL for embedded)
  - Memory-mapped file support for processing datasets larger than RAM
- **Cons**:
  - Learning curve — columnar thinking differs from row-based ORM patterns
  - Overkill for simple row-by-row import (csv crate is simpler)
  - API surface is large and still evolving
  - Adds significant binary size (~10-15MB)
- **Roadmap stage fit**: **Stage 2-3** — When ORCHA needs analytics, cross-tenant reporting, or complex data transformations. Not needed for Stage 0-1 CSV import.
- **Rationale**: DataFusion becomes compelling when you need to process workflow outputs at scale (aggregation, joins, filtering across execution artifacts). Premature before that.
- **Budget/timeline**: 0 cost. 2-4 weeks for initial integration. Ongoing maintenance as API evolves.

### 3.2 Polars

- **License**: MIT
- **What it does**: DataFrame library for Rust (and Python). Lazy evaluation, parallel execution, columnar storage. Think "pandas but fast and in Rust."
- **Pros**:
  - MIT license — most permissive
  - Incredible performance (often 5-10x pandas, competitive with DuckDB)
  - Lazy evaluation — query optimizer before execution
  - Native CSV, Parquet, JSON, Excel reading
  - Expression-based API is intuitive: `df.filter(col("status").eq("won")).group_by("org_id").sum()`
  - Excellent for data profiling, aggregation, deduplication
  - Active development, strong community
- **Cons**:
  - Adds ~15-20MB to binary size
  - API is Rust-idiomatic but has a learning curve for DataFrame operations
  - Not a query engine — for SQL-style queries, DataFusion is more natural
  - Some features are Python-only (plotting, certain I/O connectors)
- **Roadmap stage fit**: **Stage 1-2** — Useful for data profiling during import (detect duplicates, profile column distributions, validate data quality). Could replace custom import validation logic.
- **Rationale**: When import volumes grow beyond simple row-by-row validation, Polars provides efficient batch processing. Also useful for generating import preview statistics.
- **Budget/timeline**: 0 cost. 1-2 weeks for initial integration. The main investment is learning the expression API.

### 3.3 Singer/Meltano

- **License**: MIT (Meltano core), individual taps/targets are MIT or Apache 2.0
- **What it does**: Meltano is a DataOps platform built on the Singer specification. Singer defines a standard JSON-based protocol for data extraction (taps) and loading (targets). Meltano provides orchestration, configuration management, and a plugin ecosystem.
- **Ecosystem**: 600+ Singer taps/targets covering most SaaS APIs (Salesforce, HubSpot, Jira, Slack, Google Sheets, etc.)
- **Pros**:
  - Massive connector ecosystem — most CRM/PM/communication connectors already exist
  - Standard protocol — can mix and match taps and targets
  - Meltano Hub for discovering and managing connectors
  - Declarative configuration (YAML-based)
  - Built-in scheduling and orchestration
  - Active community (GitLab-backed)
- **Cons**:
  - Python-based — requires Python runtime alongside Rust backend
  - Singer connectors vary wildly in quality (some unmaintained, some buggy)
  - JSON-lines protocol is inefficient for large datasets
  - Meltano itself adds operational complexity (another service to manage)
  - Connector versioning and dependency conflicts (Python packaging issues)
  - Some connectors have bugs in incremental replication
- **Roadmap stage fit**: **Stage 2** — When ORCHA needs many connectors fast. The breadth of the ecosystem is the value.
- **Rationale**: Building native Rust connectors for every SaaS API is time-prohibitive. Singer/Meltano provides a bridge: use Singer taps for initial integration, replace with native Rust connectors for high-traffic sources over time.
- **Budget/timeline**: 0 cost. 1-2 weeks for Meltano integration. Individual connector setup is hours to days. Ongoing maintenance for connector updates.
- **Integration pattern**: Run Meltano as a sidecar process, pipe output into ORCHA's import pipeline via stdout JSON-lines or temporary files.

### 3.4 Airbyte

- **License**: Elv2 (Elastic License 2.0) for core platform (NOT MIT/Apache). Individual connectors are MIT.
- **What it does**: Open-source ELT platform with 350+ connectors. UI-based configuration, scheduling, and monitoring. CDC support.
- **Pros**:
  - Largest connector ecosystem with maintained quality
  - Beautiful UI for connector configuration
  - Built-in schema evolution handling
  - Normalization layer
  - CDC support for databases
  - Strong community and commercial backing (Series B, $181M raised)
- **Cons**:
  - **License warning**: Elv2 is NOT permissive — prohibits offering as a managed service. If ORCHA offers data import as a feature to paying customers, this may violate the license. Connectors themselves are MIT.
  - Heavy resource footprint (Java/Python, Docker-based connector runtime)
  - Each connector runs in its own Docker container — high overhead for small datasets
  - Self-hosting requires significant infrastructure (Kubernetes recommended)
  - Complex for embedding into an existing platform
- **Roadmap stage fit**: **Stage 3+** (if at all) — License concerns make this risky for a SaaS platform. Consider individual MIT-licensed connectors only.
- **Rationale**: The Elv2 license is a dealbreaker for embedding into a commercial SaaS product. Use Singer/Meltano (MIT) instead and selectively port Airbyte's MIT connectors if needed.
- **Budget/timeline**: 0 cost for self-hosted. Legal review required for license compliance. High infra cost.

### 3.5 dbt (data build tool)

- **License**: Apache 2.0
- **What it does**: SQL-based transformation framework. Define transforms as SQL SELECT statements, dbt handles dependency ordering, incremental materialization, and testing.
- **Pros**:
  - SQL-first — analysts can write transforms without code
  - Excellent testing framework (schema tests, data tests, freshness checks)
  - Incremental models reduce processing time
  - Documentation generation from model descriptions
  - Massive community and ecosystem (dbt packages, dbt Hub)
  - Works with PostgreSQL (dbt-postgres adapter)
- **Cons**:
  - Python-based — another runtime dependency
  - Designed for warehouse-to-warehouse transforms, not application databases
  - Overhead for simple transforms (better for complex multi-step pipelines)
  - Not a real-time tool — batch-oriented
  - Doesn't extract or load data (the T in ELT only)
- **Roadmap stage fit**: **Stage 3+** — When ORCHA has a data warehouse or needs complex cross-tenant analytics. Not needed for transactional import/export.
- **Rationale**: dbt excels at transforming raw data into analytics-ready models. ORCHA doesn't need this until it has a reporting/analytics tier separate from the operational database.
- **Budget/timeline**: 0 cost. 1-2 weeks for initial setup. Ongoing model development.

---

## 4. Data Quality Tools

### 4.1 Great Expectations

- **License**: Apache 2.0
- **What it does**: Python framework for data validation, profiling, and documentation. Define "expectations" (rules) about your data, run validation, get detailed reports.
- **Pros**:
  - Comprehensive expectation library (150+ built-in expectations)
  - Auto-profiling generates expectations from sample data
  - Data docs — auto-generated documentation of data quality
  - Integrates with Pandas, Spark, SQLAlchemy
  - Strong community
- **Cons**:
  - Python-only — requires Python runtime
  - Heavy dependency tree
  - Learning curve for expectation suites and checkpoints
  - Overkill for row-level import validation
- **Roadmap stage fit**: **Stage 3+** — When data quality monitoring becomes a feature (data observability for clients). For Stage 0-1, simple Rust validation is sufficient.
- **Budget/timeline**: 0 cost. 2-3 weeks for integration. Requires Python sidecar.

### 4.2 Data Profiling (Rust-Native)

**Recommended approach for Stage 0-1**: Build lightweight profiling during import using Rust:

- **Column statistics**: min, max, mean, median, null count, unique count, type distribution
- **Pattern detection**: email regex, phone regex, URL regex, date format detection
- **Completeness score**: percentage of non-null values per column
- **Uniqueness score**: percentage of unique values (helps identify candidate keys)

**Crates**:
- `statrs` (MIT) — statistical distributions and functions
- `regex` (MIT/Apache 2.0) — pattern detection
- `chrono` (MIT/Apache 2.0) — date parsing and format detection
- `strsim` (MIT) — string similarity for fuzzy matching

**Roadmap stage fit**: **Stage 1** — Profiling during import preview helps users understand their data quality before committing.

### 4.3 Deduplication Algorithms

**The Problem**: CRM imports almost always contain duplicates. "John Smith" vs "John D. Smith" vs "Jon Smith". Same person, different records.

**Algorithms** (in increasing sophistication):

1. **Exact match on canonical field**: Email address, phone number. Cheapest, catches 60-70% of dupes.
   - ORCHA already does this: `CrmContact::find_by_email()` in commit dedup.

2. **Normalized match**: Lowercase, trim whitespace, strip punctuation, then exact match. Catches formatting differences.

3. **Fuzzy string matching**:
   - Jaro-Winkler similarity (best for names, weights prefix matches higher)
   - Levenshtein distance (edit distance, good for typos)
   - Soundex/Metaphone (phonetic matching, "Smith" = "Smyth")
   - `strsim` crate (MIT) provides all of these

4. **Record linkage / probabilistic matching**:
   - Fellegi-Sunter model: weight multiple fields (name similarity + address similarity + phone match) → composite match score
   - Blocking: group records by approximate key (first 3 chars of last name + zip code) to reduce O(n^2) comparison space
   - Active learning: use ML to learn from user-confirmed matches

5. **ML-based deduplication**:
   - Dedupe.io (MIT, Python) — active learning deduplication
   - RecordLinkage (MIT, Python) — statistical record linkage

**Recommendation for ORCHA**:
- **Stage 0-1**: Email exact match (already have) + normalized name match + Jaro-Winkler on name fields. Implement in Rust using `strsim`.
- **Stage 2**: Blocking + multi-field composite scoring. Add UI for "potential duplicates" review queue.
- **Stage 3+**: ML-based deduplication or integration with Dedupe.io for large datasets.

**Existing ORCHA code to extend**: `check_contact_duplicate`, `check_intra_batch_duplicate` in workflow staging already implement basic dedup. Enhance these with fuzzy matching.

### 4.4 Data Lineage Tracking

**What it is**: Tracking the origin, transformations, and destinations of every data record. "Where did this CRM contact come from? What workflow processed it? What changed?"

**Patterns**:
- **Event-based lineage**: Every data mutation emits an event with source, operation, and actor. ORCHA's workflow execution already produces execution artifacts that serve as partial lineage.
- **Metadata tagging**: Each record carries `source_type`, `source_id`, `imported_at`, `imported_by` metadata.
- **Graph-based lineage**: DAG of data transformations. Nodes are datasets/records, edges are transforms. Visualize with the existing topology graph UI.

**Rust tools**:
- No mature Rust-native lineage tool exists
- Best approach: model lineage as metadata on existing records + event log

**Recommendation**: Extend the `execution_artifacts` table to track full lineage. Add `source_type` (csv_import, hubspot_sync, workflow_output, manual_entry) and `source_ref` (file ID, external record ID) to CRM records.

**Roadmap stage fit**: **Stage 1-2** — Basic source tracking at Stage 1. Full lineage graph at Stage 2+.

---

## 5. Webhook/Event Processing

### 5.1 Inbound Webhook Handling

**Current state**: ORCHA has `crates/server/src/routes/webhooks.rs` with HMAC validation. Webhook triggers for workflows exist (`workflow_triggers.rs`). A cooldown race condition is noted in architecture gaps.

**Patterns for robust webhook handling**:

1. **Immediate ACK, async processing**: Return 200 immediately, queue the payload for async processing. This prevents webhook timeouts and retries from the sender.
   ```
   POST /api/webhooks/:source → validate HMAC → store payload → return 200
   Background worker → dequeue → process → update state
   ```

2. **Idempotency**: External systems often retry webhooks. Use webhook ID or content hash as idempotency key. Store processed webhook IDs in a `webhook_events` table with TTL.

3. **Signature verification**: HMAC-SHA256 for most services (HubSpot, Stripe, GitHub). Some use asymmetric signatures (Salesforce).

4. **Payload normalization**: Each source has different payload formats. Normalize to internal event format before processing.

**Crates**:
- `hmac` + `sha2` (MIT/Apache 2.0) — already in use
- `ring` (ISC license, permissive) — alternative crypto with better performance

### 5.2 Event Queuing

**Current state**: ORCHA uses NATS for event messaging. SSE for real-time frontend updates.

**Options for reliable event queuing**:

| Queue System | License | Pros | Cons | Stage Fit |
|---|---|---|---|---|
| **NATS JetStream** | Apache 2.0 | Already using NATS, persistence + replay, at-least-once delivery | Operational overhead, less ecosystem than RabbitMQ | Stage 0-1 (extend existing) |
| **PostgreSQL LISTEN/NOTIFY + pgqueue** | N/A (built-in) | Zero new infra post-migration, transactional enqueue | Limited throughput (~1000 msg/sec), no persistence beyond current state | Stage 2 (after PG migration) |
| **Redis Streams** | BSD 3-clause | Fast, sorted, consumer groups, mature | Another service to operate, memory-bound | Stage 2+ |
| **RabbitMQ** | MPL 2.0 | Battle-tested, dead letter queues built-in, AMQP standard | Java-based, operational complexity | Stage 3+ |
| **SQLite/PG as queue** | N/A | No new infra, SKIP LOCKED for PG, simple | Lower throughput, polling-based | Stage 0-1 (simplest) |

**Recommendation**:
- **Stage 0-1**: Use the database as a simple job queue. A `webhook_events` table with `status` column (pending → processing → completed → failed). Background worker polls with `SELECT ... WHERE status = 'pending' ORDER BY created_at LIMIT 10 FOR UPDATE SKIP LOCKED` (PostgreSQL) or equivalent SQLite pattern.
- **Stage 2+**: NATS JetStream for persistent event streaming. Already have NATS infrastructure.

### 5.3 Retry and Dead Letter Queue (DLQ)

**Pattern**:
```
Attempt 1 → process → success → done
         → failure → wait 1s
Attempt 2 → process → success → done
         → failure → wait 5s
Attempt 3 → process → success → done
         → failure → wait 30s
Attempt 4 → process → success → done
         → failure → move to DLQ
```

**Implementation**:
- `retry_count` and `next_retry_at` columns on the job/event table
- Exponential backoff: `delay = base_delay * 2^retry_count` with jitter
- Max retries configurable per event type (3-5 typical)
- DLQ: separate table or status='dead_letter' for manual review
- Admin UI to inspect, replay, or discard DLQ items

**Crates**:
- `backoff` (MIT) — exponential backoff with jitter
- `tokio-retry` (MIT) — retry with configurable strategies (currently unmaintained, consider custom)

**Roadmap stage fit**: **Stage 0-1** — Essential for webhook reliability. Simple implementation with DB-backed queue.

### 5.4 Rate Limiting for External API Calls

**The Problem**: When syncing data from HubSpot/Salesforce/etc., ORCHA must respect their rate limits. Bursting past limits results in 429 errors and potential API key suspension.

**Patterns**:
- **Token bucket**: Shared bucket per API source. Each request consumes a token. Tokens refill at the API's stated rate. Already implemented for Nora chat (20 req/min).
- **Sliding window**: Track timestamps of recent requests, reject if window is full. More precise than token bucket for bursty APIs.
- **Adaptive rate limiting**: Start at documented limit, back off on 429 responses, gradually increase. Handles undocumented or dynamic limits.
- **Per-tenant quotas**: Each org gets a share of the global API budget. Prevents one tenant from exhausting rate limits for all.

**Implementation**: Extend existing `TokenBucket` in ORCHA. Create a `RateLimiter` service keyed by `(api_source, tenant_id)`. Store state in Redis (Stage 2+) or in-memory with periodic persistence (Stage 0-1).

**Crates**:
- `governor` (MIT) — mature rate limiter with GCRA algorithm
- `tower` rate limiting middleware (MIT) — for HTTP-level limiting
- Custom `TokenBucket` already in codebase — extend it

**Roadmap stage fit**: **Stage 1** — Required as soon as external API connectors are built.

---

## 6. File Processing

### 6.1 PDF Extraction

#### `pdf-extract` crate
- **License**: Apache 2.0
- **What it does**: Extracts text content from PDF files. Uses `lopdf` internally for PDF parsing.
- **Pros**: Pure Rust, simple API, handles basic text extraction
- **Cons**: Struggles with complex layouts (multi-column, tables), no OCR, no image extraction, limited font handling
- **Roadmap stage fit**: **Stage 1** — Basic PDF text extraction for document data sources

#### `lopdf` crate
- **License**: MIT
- **What it does**: Low-level PDF manipulation (read, modify, write). Can extract text, images, metadata.
- **Pros**: Pure Rust, fine-grained control, can modify PDFs
- **Cons**: Low-level API, text extraction requires understanding PDF internals (content streams, fonts, encodings)
- **Roadmap stage fit**: **Stage 1** — Foundation for PDF processing, used by `pdf-extract`

#### `pdfium-render` crate
- **License**: Apache 2.0 (bindings), PDFium itself is BSD 3-clause
- **What it does**: Rust bindings to Google's PDFium library (the Chrome PDF renderer). High-quality text extraction, rendering to images, form field extraction.
- **Pros**: Production-quality extraction (Google Chrome's PDF engine), handles complex layouts, form fields, annotations
- **Cons**: Requires PDFium native library (~20MB), C++ dependency via FFI
- **Roadmap stage fit**: **Stage 2** — When extraction quality matters (client proposals, contracts)

**Recommendation**: Start with `pdf-extract` for basic text extraction (Stage 1). Graduate to `pdfium-render` when complex document parsing is needed (Stage 2).

### 6.2 Document Parsing (Apache Tika Alternative)

**Apache Tika** is Java-based and heavy. Rust alternatives:

#### `infer` crate (MIT)
- File type detection by magic bytes. Identifies 100+ file types. Useful for upload validation.

#### `docx-rs` / `docx` crate (MIT)
- Read/write DOCX files. Extract text, paragraphs, tables from Word documents.

#### `quick-xml` crate (MIT)
- Fast XML parser. DOCX/XLSX/PPTX are ZIP archives containing XML — combine with `zip` crate for Office doc parsing.

#### Strategy: Compose from primitives
Instead of one monolithic parser, compose:
- `zip` (MIT) — unpack Office/EPUB formats
- `quick-xml` (MIT) — parse XML content
- `pdf-extract` (Apache 2.0) — PDF text
- `calamine` (MIT) — Excel/ODS data
- `html5ever` (MIT/Apache 2.0) — HTML parsing
- `encoding_rs` (MIT/Apache 2.0) — character encoding detection

**Roadmap stage fit**: **Stage 1** for basic formats (PDF, DOCX, XLSX). **Stage 2** for comprehensive document pipeline.

### 6.3 OCR: Tesseract Bindings

#### `leptess` crate
- **License**: MIT (bindings), Tesseract is Apache 2.0
- **What it does**: Rust bindings to Tesseract OCR engine via Leptonica.
- **Pros**: Tesseract is the standard open-source OCR, 100+ languages, good accuracy for printed text
- **Cons**: Requires Tesseract + Leptonica system libraries, accuracy drops on handwriting/low-quality scans, slow (seconds per page)
- **Alternative**: `rusty-tesseract` (MIT) — higher-level API over `leptess`

#### Cloud OCR alternatives
- **Google Cloud Vision API**: Best accuracy, especially for handwriting. $1.50/1000 pages.
- **AWS Textract**: Good for structured documents (forms, tables). $1.50/1000 pages.
- **Azure Computer Vision**: Competitive accuracy. $1/1000 pages.

**Recommendation**:
- **Stage 1**: Skip OCR entirely. Focus on native-text documents (PDF text extraction, DOCX, spreadsheets). Most agency data is digital-native.
- **Stage 2**: Add Tesseract for scanned PDF detection. If a PDF has no extractable text, offer OCR option.
- **Stage 3**: Cloud OCR for high-accuracy needs (contract parsing, invoice extraction).

### 6.4 Audio Transcription Pipeline

**Current state**: ORCHA has Nora voice integration with Twilio, `crates/nora/src/twilio/audio.rs` for audio handling, and `libopus` in the flox environment.

**Transcription options**:
- **Whisper (OpenAI)**: API at $0.006/minute. Best accuracy. Also available self-hosted (whisper.cpp, MIT).
- **whisper-rs** crate (MIT): Rust bindings to whisper.cpp. CPU inference, ~1x realtime on modern hardware. GPU inference via CUDA/Metal.
- **Deepgram**: API-based, real-time streaming transcription. $0.0043/minute for pre-recorded.
- **AssemblyAI**: API-based, strong diarization (speaker identification). $0.012/minute.

**Recommendation**:
- **Stage 0**: Use Whisper API (already likely used via Nora). $0.006/min is negligible for agency call volumes.
- **Stage 2**: Self-host whisper.cpp via `whisper-rs` for cost reduction at scale (eliminates per-minute API cost).
- **Stage 3+**: Real-time transcription via Deepgram for live meeting capture.

---

## 7. Data Migration for Multi-Tenant

### 7.1 Tenant Data Isolation During Import

**Problem**: When tenant A imports data, it must never leak to tenant B. Imports must be scoped to `organization_id`.

**Current state**: ORCHA CRM is already scoped to `organization_id`. Data sources are scoped via `organization_id` or `project_id`. The scaffolding exists.

**Patterns**:
1. **Row-level scoping** (current): Every record has `organization_id`. All queries filter by org. Risk: forgotten WHERE clause leaks data.
2. **PostgreSQL Row-Level Security (RLS)**: Database-enforced isolation. Even if application code forgets the WHERE clause, RLS blocks cross-tenant access. Set `current_setting('app.current_org_id')` on each connection.
3. **Schema-per-tenant**: Each org gets a PostgreSQL schema. Complete isolation, but migration complexity scales linearly.
4. **Database-per-tenant**: Each org gets a separate database file/instance. Maximum isolation but highest operational cost.

**Recommendation**:
- **Stage 0-1** (SQLite): Row-level scoping with `organization_id` (current approach). Add auditing.
- **Stage 2** (PostgreSQL): Implement RLS as defense-in-depth on top of application-level scoping. RLS is the industry best practice for multi-tenant SaaS.
- **Stage 3+**: Evaluate schema-per-tenant for enterprise customers who require contractual data isolation.

**Import-specific isolation**:
- Every import job is tagged with `organization_id` + `imported_by` user ID
- Import processing runs in a scoped context that cannot access other orgs' data
- Failed imports roll back completely (transaction-based)
- Import audit log: who imported what, when, how many records, which records failed

### 7.2 Data Export/Portability (GDPR)

**GDPR Article 20 — Right to Data Portability**: Data subjects can request their data in a "structured, commonly used and machine-readable format."

**What ORCHA must export per tenant**:
- CRM contacts, companies, deals (CSV + JSON)
- Project data, tasks, comments
- Workflow definitions and execution history
- Uploaded files and documents
- Communication history (if stored)
- User profiles and activity logs

**Implementation pattern**:
1. **Export endpoint**: `POST /api/organizations/:id/export` triggers async export job
2. **Background worker**: Queries all org-scoped tables, generates ZIP archive with CSV files per entity type + a manifest JSON describing the schema
3. **Storage**: Upload ZIP to object storage (S3/R2), generate time-limited download URL
4. **Notification**: Email/SSE notification when export is ready
5. **Retention**: Auto-delete export after 7 days

**Format**: CSV for tabular data (universally readable), JSON for complex/nested data, original files for uploads.

**Crates**:
- `zip` (MIT) — create ZIP archives
- `csv` (MIT) — CSV generation
- `tokio::fs` — async file I/O

**Roadmap stage fit**: **Stage 1** — Required before accepting paying customers. GDPR compliance is a legal requirement for EU customers and a trust signal for all customers.
- **Budget/timeline**: 1-2 weeks for basic export. Ongoing maintenance as data model evolves.

### 7.3 Backup Per Tenant

**Patterns**:
- **Full database backup**: Simple but includes all tenants. Good for disaster recovery, not for single-tenant restore.
- **Logical backup per tenant**: Query all org-scoped data, export to backup format. Slower but allows single-tenant restore.
- **Point-in-time recovery (PITR)**: PostgreSQL WAL archiving allows restoring to any point in time. Best for disaster recovery.
- **Snapshot-based**: For schema-per-tenant or database-per-tenant, take per-tenant snapshots.

**Recommendation**:
- **Stage 0** (SQLite): `litestream` for continuous backup to S3 (already researched in database-migration doc). Covers full database.
- **Stage 2** (PostgreSQL): PostgreSQL PITR via WAL archiving for disaster recovery. Per-tenant logical backup on-demand (export endpoint above doubles as backup).
- **Stage 3+**: Automated per-tenant backup schedule with retention policy. Self-service restore from backup.

### 7.4 Cross-Tenant Data Sharing Controls

**Use cases**:
- Agency manages multiple client orgs, needs to view aggregated data
- White-label: agency's clients see their data only, agency sees all
- Marketplace: workflow templates shared across orgs (not data, just definitions)

**Patterns**:
- **Parent-child org hierarchy**: Agency org is parent, client orgs are children. Parent can access children's data with explicit permission.
- **Sharing links**: Generate time-limited, scoped access tokens for specific records.
- **Data federation**: Queries that span orgs are explicitly requested and logged. Never implicit.
- **Template/definition sharing**: Workflow definitions, output schemas, and pipeline templates can be "published" to a shared catalog without sharing execution data.

**Recommendation**:
- **Stage 1**: No cross-tenant sharing. Each org is fully isolated.
- **Stage 2**: Workflow template marketplace (share definitions, not data).
- **Stage 3**: Parent-child org hierarchy for agency→client relationship.

---

## 8. Recommendation Matrix

| Tool/Pattern | License | Stage | Priority | Effort | Why |
|---|---|---|---|---|---|
| **`csv` crate** | MIT | 0-1 | P1 | 1-2 days | Table stakes for data onboarding |
| **`calamine` crate** | MIT | 0-1 | P1 | 1-2 days | Excel import alongside CSV |
| **Schema mapping UI** | N/A | 1 | P1 | 1-2 weeks | Core UX for import wizard |
| **`strsim` crate** | MIT | 1 | P2 | 2-3 days | Fuzzy matching for dedup and column mapping |
| **HubSpot connector** | N/A | 1 | P2 | 1-2 weeks | Primary CRM for agency market |
| **Pipedrive connector** | N/A | 1 | P3 | 1 week | Secondary CRM for agencies |
| **Asana connector** | N/A | 1 | P3 | 1-2 weeks | PM tool popular with agencies |
| **Bulk import service** | N/A | 1 | P2 | 1 week | Performance for large imports |
| **Webhook reliability** (retry + DLQ) | N/A | 0-1 | P2 | 1 week | Extends existing webhook infra |
| **Data export (GDPR)** | N/A | 1 | P1 | 1-2 weeks | Legal requirement for paying customers |
| **`pdf-extract`** | Apache 2.0 | 1 | P3 | 2-3 days | PDF document ingestion |
| **Rate limiter for external APIs** | MIT (governor) | 1 | P2 | 3-5 days | Required for API connectors |
| **Data lineage metadata** | N/A | 1-2 | P3 | 1 week | Source tracking on CRM records |
| **Polars** | MIT | 2 | P3 | 1-2 weeks | Data profiling and batch processing |
| **Salesforce connector** | N/A | 2 | P2 | 2-3 weeks | Enterprise CRM connector |
| **Linear connector** | N/A | 2 | P3 | 1 week | Tech-forward PM tool |
| **Slack full import** | N/A | 2 | P3 | 2-3 weeks | Communication history import |
| **Google Drive connector** | N/A | 1-2 | P3 | 2 weeks | File storage integration |
| **Singer/Meltano** | MIT | 2 | P2 | 1-2 weeks | Connector ecosystem bridge |
| **PostgreSQL RLS** | N/A | 2 | P1 | 1-2 weeks | Multi-tenant security hardening |
| **DataFusion** | Apache 2.0 | 2-3 | P3 | 2-4 weeks | Analytics and complex transforms |
| **Tesseract OCR** | Apache 2.0 | 2 | P4 | 1 week | Scanned document processing |
| **whisper-rs** (self-hosted) | MIT | 2 | P3 | 1-2 weeks | Cost reduction for transcription |
| **Jira connector** | N/A | 2-3 | P3 | 2-3 weeks | Enterprise PM import |
| **Gmail/Outlook connector** | N/A | 2-3 | P3 | 3-4 weeks | Email CRM enrichment |
| **MS Teams connector** | N/A | 2-3 | P4 | 3-4 weeks | Enterprise communication |
| **dbt** | Apache 2.0 | 3+ | P4 | 1-2 weeks | Data warehouse transforms |
| **Great Expectations** | Apache 2.0 | 3+ | P4 | 2-3 weeks | Data quality monitoring |
| **pdfium-render** | Apache 2.0/BSD | 2 | P3 | 1 week | High-quality PDF extraction |
| **ML deduplication** | MIT | 3+ | P4 | 2-3 weeks | Advanced record linkage |
| **Parent-child org hierarchy** | N/A | 3 | P3 | 2-3 weeks | Agency→client data sharing |
| **Airbyte connectors** | Elv2 (CAUTION) | 3+ | P4 | N/A | License risk — prefer Singer/Meltano |

---

## 9. Implementation Roadmap

### Stage 0 (Q2 2026) — Foundation

**Goal**: Enable basic data import for PCG internal use.

1. Add `csv` and `calamine` as dependencies to `crates/services/`
2. Build `ImportService` with:
   - File upload → parse → preview (first 50 rows) → return column headers + sample data
   - Validate against target schema (CRM contact, deal, company)
   - Batch insert with transaction wrapping
3. Frontend: `ImportDialog` component with file drop + preview table
4. Extend webhook handler with idempotency key and retry (fix cooldown race condition)
5. Add `source_type` and `source_ref` metadata fields to CRM records

**No new infra. No Python. Pure Rust + React.**

### Stage 1 (Q3-Q4 2026) — Pilot Onboarding

**Goal**: External clients can import their existing data and export it.

1. Schema mapping UI (`ImportWizard` with column mapping step)
2. Fuzzy column auto-matching (`strsim`)
3. HubSpot connector (OAuth + REST, import contacts/companies/deals)
4. Pipedrive connector (simpler API, quick win)
5. Asana connector (task import for PM migration)
6. GDPR data export endpoint
7. PDF text extraction (`pdf-extract`)
8. Rate limiter for external API calls (`governor` crate)
9. Import audit log and error reporting
10. Dedup enhancement: fuzzy name matching on CRM import

### Stage 2 (Q1-Q3 2027) — Scale

**Goal**: Comprehensive data integration platform.

1. Salesforce connector
2. Google Drive connector
3. Slack full history import
4. Singer/Meltano integration for long-tail connectors
5. PostgreSQL RLS for tenant isolation
6. Polars for data profiling during import
7. Tesseract OCR for scanned documents
8. Self-hosted Whisper for transcription cost reduction
9. Data lineage graph visualization
10. Workflow template marketplace (cross-org sharing of definitions only)

### Stage 3+ (2028+) — Enterprise

**Goal**: Enterprise-grade data operations.

1. DataFusion for analytics/reporting layer
2. dbt for complex data transformations
3. Great Expectations for data quality monitoring
4. ML-based deduplication
5. Parent-child org hierarchy
6. Automated per-tenant backup and self-service restore
7. Real-time sync (CDC) for connected systems
8. Enterprise connectors (MS Teams, Outlook, Jira Cloud)

---

## Key Decisions Summary

| Decision | Choice | Rationale |
|---|---|---|
| CSV parsing | `csv` crate (Rust-native) | Mature, fast, zero dependencies |
| Excel parsing | `calamine` (Rust-native) | Pure Rust, all formats, read-only is fine |
| Connector ecosystem bridge | Singer/Meltano (Stage 2) over Airbyte | MIT vs Elv2 license, lower resource footprint |
| Event queue (Stage 0-1) | Database-as-queue | No new infrastructure |
| Event queue (Stage 2+) | NATS JetStream | Already in stack |
| Data processing at scale | Polars (Stage 2) | MIT, Rust-native, faster than DataFusion for DataFrame ops |
| Analytics queries | DataFusion (Stage 3) | SQL interface for complex analytics |
| Multi-tenant isolation | RLS on PostgreSQL (Stage 2) | Industry standard, defense-in-depth |
| PDF extraction | `pdf-extract` (Stage 1) → `pdfium-render` (Stage 2) | Graduate as needs grow |
| OCR | Skip (Stage 0-1) → Tesseract (Stage 2) → Cloud (Stage 3) | Most agency data is digital-native |
| Deduplication | Exact email match → fuzzy name → ML-based | Progressive sophistication |
| Data export format | CSV + JSON in ZIP | Universal readability |
| Transcription | Whisper API (Stage 0) → whisper-rs self-hosted (Stage 2) | Cost optimization at scale |
