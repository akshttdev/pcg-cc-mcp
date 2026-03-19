Now I have comprehensive research across all 15 topics. Let me compile the final report.

---

# PostgreSQL as a Graph Database: Comprehensive Research Report

## Executive Summary

PostgreSQL, with its extensions and built-in recursive CTE support, can handle the graph workloads of a multi-tenant SaaS platform (task dependencies, CRM networks, workflow DAGs, agent knowledge graphs, org hierarchies) up to significant scale -- Alibaba demonstrated **millisecond responses on 5 billion edges**. A dedicated graph DB is only necessary at extreme scale (100M+ nodes with deep traversals of 10+ hops) or when you need specialized graph algorithms (community detection, PageRank) running continuously. For your use cases, PostgreSQL alone is sufficient.

---

## 1. Recursive CTEs in PostgreSQL

PostgreSQL 14+ provides built-in `SEARCH` and `CYCLE` clauses for recursive CTEs, making graph traversal first-class.

**Core pattern for DAG traversal with cycle detection:**
```sql
WITH RECURSIVE graph AS (
  -- Base case: starting node
  SELECT id, parent_id, ARRAY[id] AS path, 0 AS depth, false AS is_cycle
  FROM edges WHERE source_id = :start_id
  
  UNION ALL
  
  -- Recursive case
  SELECT e.id, e.parent_id, g.path || e.id, g.depth + 1, e.id = ANY(g.path)
  FROM edges e
  INNER JOIN graph g ON e.source_id = g.target_id
  WHERE NOT g.is_cycle AND g.depth < :max_depth
)
SEARCH DEPTH FIRST BY id SET ordercol
CYCLE id SET is_cycle USING path
SELECT * FROM graph;
```

**Performance at scale (Alibaba benchmarks on 5 billion edges):**
- 3-depth search, 100 records per tier: **2.1ms response, 12,000 TPS**
- Key optimization: `CLUSTER` table by node identifier to reduce random I/O, plus B-tree indexes on edge source/target columns
- Depth control is essential -- unbounded recursion kills performance

**Practical limits:** Works well up to ~5-6 hop depth. Beyond that, the combinatorial explosion of paths degrades performance unless you add strict per-level LIMIT clauses.

Sources:
- [PostgreSQL Documentation: WITH Queries](https://www.postgresql.org/docs/current/queries-with.html)
- [Recursion with PostgreSQL: Cycle Detection and Processing Millions of Nodes](https://github.com/vb-consulting/blog/discussions/5)
- [Cycle detection in PostgreSQL (Mergify)](https://articles.mergify.com/cycle-detection-in-postgresql/)
- [PostgreSQL Graph Search: 10 Billion-Scale with Millisecond Response (Alibaba)](https://www.alibabacloud.com/blog/postgresql-graph-search-practices---10-billion-scale-graph-with-millisecond-response_595039)

---

## 2. Apache AGE Extension

Apache AGE adds **openCypher query language** to PostgreSQL, letting you write graph queries alongside SQL.

**Status:** Apache Top-Level Project since May 2022. Active development continues with an out-of-cycle release in February 2026. Available on Azure Database for PostgreSQL. Some production users have expressed concern about development velocity but noted renewed activity in late 2025.

**How it works:** You create a named graph, then use Cypher syntax wrapped in SQL:
```sql
-- Create graph
SELECT create_graph('task_deps');

-- Add nodes
SELECT * FROM cypher('task_deps', $$
  CREATE (:Task {id: 'TASK-1', title: 'Design API'}),
         (:Task {id: 'TASK-2', title: 'Implement API'})
$$) AS (v agtype);

-- Add dependency edge
SELECT * FROM cypher('task_deps', $$
  MATCH (a:Task {id: 'TASK-1'}), (b:Task {id: 'TASK-2'})
  CREATE (b)-[:DEPENDS_ON]->(a)
$$) AS (e agtype);

-- Find all upstream dependencies (variable-length path)
SELECT * FROM cypher('task_deps', $$
  MATCH (t:Task {id: 'TASK-5'})-[:DEPENDS_ON*1..10]->(dep:Task)
  RETURN dep.id, dep.title
$$) AS (id agtype, title agtype);
```

**Performance:** In tests, raw AGE queries took ~3.7ms vs ~0.8ms for equivalent recursive CTEs. The readability advantage of Cypher is significant, but there's a performance cost. AGE stores graph data in its own catalog tables (`ag_catalog`) alongside your relational tables.

**Verdict:** Excellent for developer ergonomics when graph queries are complex. Not yet as battle-tested as recursive CTEs for production workloads. The hybrid SQL+Cypher capability is its killer feature.

Sources:
- [Apache AGE Official](https://age.apache.org/overview/)
- [Apache AGE GitHub](https://github.com/apache/age)
- [AGE 2026 Roadmap Discussion](http://www.mail-archive.com/dev@age.apache.org/msg07985.html)
- [Azure Database for PostgreSQL: AGE Extension](https://learn.microsoft.com/en-us/azure/postgresql/azure-ai/generative-ai-age-overview)
- [Basics of Querying with Cypher in PostgreSQL using Apache AGE](https://dev.to/xk_woon/the-basics-of-querying-with-cypher-in-postgresql-using-apache-age-43p1)

---

## 3. pg_graphql (Supabase)

**Not what you need.** pg_graphql auto-generates a **GraphQL API** from your PostgreSQL schema -- it exposes tables/views as GraphQL types with filtering, pagination, and relationships. It does NOT provide graph database query capabilities (no traversals, no path finding, no Cypher).

It is useful if you want a GraphQL API layer over your relational data, but it has zero overlap with graph database functionality. The name is misleading for this context.

Sources:
- [pg_graphql: GraphQL for PostgreSQL (Supabase Docs)](https://supabase.com/docs/guides/database/extensions/pg_graphql)
- [pg_graphql GitHub](https://github.com/supabase/pg_graphql)

---

## 4. ltree Extension

**Best for:** Org hierarchies, category trees, file system paths -- any strict tree (not DAG/graph).

```sql
CREATE EXTENSION ltree;

-- Org hierarchy
CREATE TABLE organizations (
  id UUID PRIMARY KEY,
  name TEXT,
  path ltree  -- e.g., 'acme.engineering.frontend'
);

CREATE INDEX idx_org_path_gist ON organizations USING GIST (path);

-- All descendants of engineering
SELECT * FROM organizations WHERE path <@ 'acme.engineering';

-- All ancestors of a node
SELECT * FROM organizations WHERE 'acme.engineering.frontend.team_a' <@ path;

-- Pattern matching: any team directly under any department
SELECT * FROM organizations WHERE path ~ 'acme.*.team_*';
```

**Strengths:**
- GiST index makes ancestor/descendant queries extremely fast
- Built-in lquery and ltxtquery operators for pattern matching
- No recursion needed

**Limitations:**
- Strictly trees, not DAGs or graphs (a node can have only one parent)
- Path length limited to 65,535 labels
- Moving subtrees requires updating all descendant paths (expensive for frequent restructuring)
- GiST index has size limitations for very deep trees

**Recommendation for your platform:** Use ltree for org hierarchies (which are strict trees). Use adjacency list + recursive CTEs for everything else.

Sources:
- [PostgreSQL Documentation: ltree](https://www.postgresql.org/docs/current/ltree.html)
- [PostgreSQL ltree vs WITH RECURSIVE (Cybertec)](https://www.cybertec-postgresql.com/en/postgresql-ltree-vs-with-recursive/)
- [Hierarchical Models in PostgreSQL (Ackee)](https://www.ackee.agency/blog/hierarchical-models-in-postgresql)

---

## 5. Hierarchy Storage Pattern Tradeoffs

| Pattern | Read Perf | Write Perf | Move Subtree | Storage | Best For |
|---------|-----------|------------|-------------|---------|----------|
| **Adjacency List** | Needs recursion | Excellent | Trivial (update 1 row) | Minimal | Frequently changing trees, DAGs |
| **Closure Table** | Excellent (no recursion) | Moderate | Moderate (delete+reinsert paths) | O(n^2) worst case | Read-heavy hierarchies with level queries |
| **Nested Set** | Excellent (range query) | Poor (renumber on insert) | Very expensive | Minimal | Static read-heavy trees |
| **Materialized Path / ltree** | Good (prefix match) | Good (insert=set path) | Moderate (update all descendants) | Moderate | Deep trees with pattern queries |

**Recommendation for your use cases:**
- **Task dependencies (DAG):** Adjacency list (edges table) + recursive CTEs. Tasks can have multiple parents in a DAG, ruling out ltree/nested set.
- **Org hierarchy (tree):** ltree for read-heavy, adjacency list if orgs restructure frequently.
- **Workflow DAGs:** Adjacency list -- matches what Airflow/Temporal do.
- **CRM contact networks (graph):** Adjacency list with typed edges.
- **Knowledge graph (graph):** Triple store (subject, predicate, object) which is effectively an adjacency list with labeled edges.

Sources:
- [Hierarchical Models in PostgreSQL (Ackee)](https://www.ackee.agency/blog/hierarchical-models-in-postgresql)
- [SQL Trees: From Adjacency Lists to Nested Sets and Closure Tables](https://teddysmith.io/sql-trees/)
- [Storing Hierarchical Data in Relational Databases](https://adamdjellouli.com/articles/databases_notes/03_sql/09_hierarchical_data)

---

## 6. JSONB for Graph Edges

**Viable but not recommended as primary pattern.** JSONB can store adjacency lists inline:

```sql
CREATE TABLE nodes (
  id UUID PRIMARY KEY,
  data JSONB,
  edges JSONB  -- e.g., [{"target": "uuid", "type": "depends_on", "weight": 1.0}]
);
```

**When it makes sense:**
- Storing dynamic edge properties alongside a normalized edges table
- Hybrid model: normalized columns for frequently queried attributes, JSONB for flexible metadata

**When it breaks down:**
- Deeply nested structures (5+ levels) are nearly impossible to index effectively
- Traversal requires extracting and joining JSONB arrays -- much slower than a proper edges table with B-tree indexes
- No foreign key enforcement on references inside JSONB

**Better approach:** Use a normalized edges table with a JSONB `metadata` column for flexible edge properties:
```sql
CREATE TABLE edges (
  id UUID PRIMARY KEY,
  source_id UUID REFERENCES nodes(id),
  target_id UUID REFERENCES nodes(id),
  edge_type TEXT NOT NULL,
  metadata JSONB DEFAULT '{}',
  created_at TIMESTAMPTZ DEFAULT now()
);
CREATE INDEX idx_edges_source ON edges(source_id);
CREATE INDEX idx_edges_target ON edges(target_id);
CREATE INDEX idx_edges_type ON edges(edge_type);
```

Sources:
- [PostgreSQL JSONB Performance Guide](https://www.sitepoint.com/postgresql-jsonb-query-performance-indexing/)
- [PostgreSQL JSONB: Powerful Storage for Semi-Structured Data](https://www.architecture-weekly.com/p/postgresql-jsonb-powerful-storage)

---

## 7. Graph Traversal Performance: PostgreSQL vs Neo4j

| Scale | PostgreSQL (recursive CTE) | Neo4j | Winner |
|-------|---------------------------|-------|--------|
| <10K nodes, 2-3 hops | Microseconds | Microseconds | Tie (PG wins on simplicity) |
| 10K-100K nodes, 3-5 hops | Low milliseconds | Sub-millisecond | Neo4j slight edge |
| 1M nodes, 5-10 hops | 10-100ms (with optimization) | Low milliseconds | Neo4j |
| 10M+ nodes, 10+ hops | Seconds to timeouts | Milliseconds | Neo4j decisively |
| 5B edges, 3-depth with LIMIT | 2.1ms (Alibaba benchmark) | N/A | PG viable with optimization |

**Key insight:** The crossover point depends heavily on **traversal depth**, not just node count. PostgreSQL handles millions of nodes fine for shallow traversals (2-4 hops). Deep traversals (10+ hops) on large graphs is where dedicated graph DBs pull ahead due to index-free adjacency.

**For your platform's use cases:**
- Task dependencies: Rarely exceed 5-10 levels deep. PostgreSQL is fine.
- CRM networks: Typically 2-3 hops ("who knows who knows who"). PostgreSQL is fine.
- Org hierarchy: Rarely exceeds 10 levels. PostgreSQL is fine.
- Workflow DAGs: Typically 5-50 steps, linear-ish. PostgreSQL is fine.
- Knowledge graph: This is the one that could push limits at scale with deep reasoning chains.

Sources:
- [Neo4j Performance Experiment](https://neo4j.com/news/how-much-faster-is-a-graph-database-really/)
- [PostgreSQL as a Graph Database (Hacker News discussion)](https://brianlovin.com/hn/35386948)
- [Alibaba 10 Billion-Scale Graph with Millisecond Response](https://www.alibabacloud.com/blog/postgresql-graph-search-practices---10-billion-scale-graph-with-millisecond-response_595039)

---

## 8. pgRouting

**Specialized for geospatial routing**, not general-purpose graph algorithms. Provides Dijkstra, A*, Yen's k-shortest paths, minimum spanning trees, driving distance calculations. Built on PostGIS.

**Relevant to your platform?** Only if you need geographic routing (delivery optimization, logistics). For task dependency shortest paths or workflow critical paths, recursive CTEs are simpler and don't require the PostGIS dependency.

Sources:
- [pgRouting Extension Analysis (CMU)](https://db.cs.cmu.edu/pgexts-vldb2025/pgrouting.html)
- [pgRouting Documentation](https://supabase.com/docs/guides/database/extensions/pgrouting)

---

## 9. Task Dependency DAGs in Practice

**How production tools do it:**

**Linear** uses Cloud SQL for PostgreSQL as its primary database, with real-time sync via WebSockets and GraphQL APIs.

**Airflow** stores workflow DAGs in PostgreSQL with these key tables:
- `dag` -- DAG definitions (id, description, schedule)
- `dag_run` -- execution instances of a DAG (dag_id, execution_date, state)
- `task_instance` -- individual task execution records (dag_id, task_id, execution_date, state, duration)
- Dependencies between tasks are defined in code (Python), not in the database

**Recommended schema for your platform:**
```sql
CREATE TABLE task_dependencies (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  depends_on_task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  dependency_type TEXT NOT NULL DEFAULT 'finish_to_start',  -- FS, SS, FF, SF
  organization_id UUID NOT NULL,
  created_at TIMESTAMPTZ DEFAULT now(),
  UNIQUE(task_id, depends_on_task_id)
);

-- Prevent cycles with a trigger using recursive CTE
CREATE OR REPLACE FUNCTION check_task_dependency_cycle()
RETURNS TRIGGER AS $$
BEGIN
  IF EXISTS (
    WITH RECURSIVE dep_chain AS (
      SELECT depends_on_task_id AS tid FROM task_dependencies
      WHERE task_id = NEW.depends_on_task_id
      UNION ALL
      SELECT td.depends_on_task_id FROM task_dependencies td
      JOIN dep_chain dc ON td.task_id = dc.tid
    )
    SELECT 1 FROM dep_chain WHERE tid = NEW.task_id
  ) THEN
    RAISE EXCEPTION 'Circular dependency detected';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_check_cycle
BEFORE INSERT OR UPDATE ON task_dependencies
FOR EACH ROW EXECUTE FUNCTION check_task_dependency_cycle();
```

Sources:
- [Airflow Metadata Database (Astronomer)](https://www.astronomer.io/docs/learn/airflow-database)
- [Linear uses Google Cloud databases](https://cloud.google.com/blog/products/databases/product-workflow-tool-linear-uses-google-cloud-databases)

---

## 10. Knowledge Graph in PostgreSQL

**Triple store schema:**
```sql
CREATE TABLE kg_entities (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  properties JSONB DEFAULT '{}',
  embedding vector(1536),  -- pgvector
  organization_id UUID NOT NULL,
  created_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE kg_triples (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  subject_id UUID NOT NULL REFERENCES kg_entities(id),
  predicate TEXT NOT NULL,       -- e.g., 'works_at', 'depends_on', 'knows'
  object_id UUID NOT NULL REFERENCES kg_entities(id),
  confidence FLOAT DEFAULT 1.0,
  source_document_id UUID,
  organization_id UUID NOT NULL,
  created_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_triples_subject ON kg_triples(subject_id);
CREATE INDEX idx_triples_object ON kg_triples(object_id);
CREATE INDEX idx_triples_predicate ON kg_triples(predicate);
CREATE INDEX idx_entities_embedding ON kg_entities USING ivfflat (embedding vector_cosine_ops);
CREATE INDEX idx_entities_name_trgm ON kg_entities USING gin (name gin_trgm_ops);

-- Query: Find all entities within 2 hops of "Acme Corp" via any relationship
WITH RECURSIVE neighborhood AS (
  SELECT e.id, e.name, e.entity_type, 0 AS depth, ARRAY[e.id] AS path
  FROM kg_entities e WHERE e.name = 'Acme Corp'
  
  UNION ALL
  
  SELECT e2.id, e2.name, e2.entity_type, n.depth + 1, n.path || e2.id
  FROM neighborhood n
  JOIN kg_triples t ON (t.subject_id = n.id OR t.object_id = n.id)
  JOIN kg_entities e2 ON e2.id = CASE 
    WHEN t.subject_id = n.id THEN t.object_id 
    ELSE t.subject_id END
  WHERE n.depth < 2 AND NOT e2.id = ANY(n.path)
)
SELECT DISTINCT id, name, entity_type, depth FROM neighborhood ORDER BY depth;

-- Hybrid query: semantic similarity + graph neighborhood
WITH semantic_matches AS (
  SELECT id, name, embedding <=> :query_embedding AS distance
  FROM kg_entities
  WHERE organization_id = :org_id
  ORDER BY embedding <=> :query_embedding
  LIMIT 10
),
graph_context AS (
  SELECT t.predicate, e2.name AS related_name, e2.entity_type
  FROM semantic_matches sm
  JOIN kg_triples t ON t.subject_id = sm.id
  JOIN kg_entities e2 ON e2.id = t.object_id
)
SELECT * FROM semantic_matches
UNION ALL
SELECT * FROM graph_context;
```

Sources:
- [Knowledge Graphs with PostgreSQL (ReadyTensor)](https://app.readytensor.ai/publications/knowledge-graphs-with-postgresql-eQyINuo4ojwW)
- [Building a Personal Knowledge Graph with PostgreSQL](https://dev.to/micelclaw/4o-building-a-personal-knowledge-graph-with-just-postgresql-no-neo4j-needed-22b2)

---

## 11. GraphRAG on PostgreSQL

Microsoft's GraphRAG can be implemented on PostgreSQL + pgvector. The key components:

1. **Entity extraction** -- LLM extracts entities and relationships from documents into triples
2. **Community detection** -- Leiden algorithm groups related entities (this is the hardest part to do in pure SQL; typically done in application code with networkx or similar)
3. **Community summarization** -- LLM generates summaries per community
4. **Local search** -- Fan out from matched entities to neighbors (recursive CTE)
5. **Global search** -- Aggregate community summaries (straightforward SQL)

**Production implementation: EdgeQuake** (Rust-based) uses:
- Apache AGE for graph storage and Cypher queries
- pgvector for embeddings
- SQL pre-filtering with GIN + B-tree indexes for metadata

**Schema for GraphRAG on PostgreSQL:**
```sql
-- Chunks with embeddings
CREATE TABLE document_chunks (
  id UUID PRIMARY KEY,
  document_id UUID NOT NULL,
  content TEXT NOT NULL,
  embedding vector(1536),
  organization_id UUID NOT NULL
);

-- Entities extracted by LLM
CREATE TABLE graph_entities (
  id UUID PRIMARY KEY,
  name TEXT NOT NULL,
  entity_type TEXT,
  description TEXT,
  embedding vector(1536),
  community_id INTEGER,  -- Leiden community assignment
  organization_id UUID NOT NULL
);

-- Relationships extracted by LLM
CREATE TABLE graph_relations (
  id UUID PRIMARY KEY,
  source_entity_id UUID REFERENCES graph_entities(id),
  target_entity_id UUID REFERENCES graph_entities(id),
  relation_type TEXT NOT NULL,
  description TEXT,
  weight FLOAT DEFAULT 1.0,
  source_chunk_id UUID REFERENCES document_chunks(id),
  organization_id UUID NOT NULL
);

-- Community summaries (generated by LLM)
CREATE TABLE graph_communities (
  id INTEGER PRIMARY KEY,
  level INTEGER NOT NULL,  -- hierarchy level (0 = finest)
  summary TEXT NOT NULL,
  embedding vector(1536),
  organization_id UUID NOT NULL
);
```

**Key limitation:** Community detection (Leiden algorithm) must be done in application code -- there is no PostgreSQL extension for it. Run it periodically as a batch job.

Sources:
- [GraphRAG on Postgres: A Builder's Guide](https://medium.com/@duckweave/graphrag-on-postgres-a-builders-guide-1c6d2ecf2eed)
- [GraphRAG and PostgreSQL Integration (Microsoft)](https://techcommunity.microsoft.com/blog/azure-ai-foundry-blog/graphrag-and-postgresql-integration-in-docker-with-cypher-query-and-ai-agents/4420623)
- [EdgeQuake: High-performance GraphRAG in Rust](https://github.com/raphaelmansuy/edgequake)
- [Microsoft GraphRAG Dataflow](https://microsoft.github.io/graphrag/index/default_dataflow/)

---

## 12. CRM Contact/Relationship Graphs

**How Salesforce and HubSpot model it:**
- **Salesforce:** Account (company) is the core entity. Contacts belong to Accounts. Opportunities (deals) link to Accounts. Relationships are explicit junction tables.
- **HubSpot:** Four layers -- Objects (tables), Records (rows), Properties (columns), Associations (junction table with type). Associations connect any object to any other object with typed relationships.

**PostgreSQL schema for CRM relationship graph:**
```sql
-- Your existing crm_contacts table serves as the node table
-- Add a relationship/association table:
CREATE TABLE crm_associations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  source_type TEXT NOT NULL,      -- 'contact', 'company', 'deal'
  source_id UUID NOT NULL,
  target_type TEXT NOT NULL,
  target_id UUID NOT NULL,
  association_type TEXT NOT NULL,  -- 'works_at', 'knows', 'referred_by', 'decision_maker_for'
  metadata JSONB DEFAULT '{}',    -- strength, notes, etc.
  organization_id UUID NOT NULL,
  created_at TIMESTAMPTZ DEFAULT now(),
  UNIQUE(source_id, target_id, association_type)
);

-- Find a contact's network (2 hops)
WITH RECURSIVE network AS (
  SELECT target_id AS contact_id, association_type, 1 AS depth
  FROM crm_associations
  WHERE source_id = :contact_id AND source_type = 'contact'
  
  UNION ALL
  
  SELECT a.target_id, a.association_type, n.depth + 1
  FROM crm_associations a
  JOIN network n ON a.source_id = n.contact_id
  WHERE n.depth < 2 AND a.target_type = 'contact'
)
SELECT DISTINCT c.*, n.depth, n.association_type
FROM network n JOIN crm_contacts c ON c.id = n.contact_id;
```

This is fundamentally the same pattern HubSpot uses -- typed associations in a junction table. PostgreSQL handles this perfectly.

Sources:
- [HubSpot Data Model Overview](https://aptitude8.com/blog/introducing-hubspots-data-model-overview-beta)
- [HubSpot vs Salesforce Data Model Comparison](https://www.coastalconsulting.co/blog/hubspot-vs-salesforce-data-model-a-comparison-and-integration-guide)

---

## 13. Workflow DAG Execution

**Airflow's approach (PostgreSQL-backed):**
- `dag` table: workflow definitions
- `dag_run` table: execution instances (one per trigger)
- `task_instance` table: per-task execution state (queued, running, success, failed, skipped)
- Dependencies defined in code, not DB -- the scheduler resolves them at runtime
- Uses SQLAlchemy ORM over PostgreSQL

**Recommended pattern for your workflow engine:**
```sql
CREATE TABLE workflow_steps (
  id UUID PRIMARY KEY,
  workflow_id UUID NOT NULL REFERENCES workflows(id),
  step_type TEXT NOT NULL,
  config JSONB NOT NULL,
  organization_id UUID NOT NULL
);

CREATE TABLE workflow_step_edges (
  id UUID PRIMARY KEY,
  workflow_id UUID NOT NULL,
  from_step_id UUID REFERENCES workflow_steps(id),
  to_step_id UUID REFERENCES workflow_steps(id),
  condition JSONB,  -- optional branching condition
  UNIQUE(from_step_id, to_step_id)
);

CREATE TABLE workflow_executions (
  id UUID PRIMARY KEY,
  workflow_id UUID NOT NULL,
  status TEXT NOT NULL,  -- 'running', 'completed', 'failed'
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ
);

CREATE TABLE step_executions (
  id UUID PRIMARY KEY,
  execution_id UUID NOT NULL REFERENCES workflow_executions(id),
  step_id UUID NOT NULL REFERENCES workflow_steps(id),
  status TEXT NOT NULL,
  input JSONB,
  output JSONB,
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ
);

-- Find next executable steps (all dependencies completed)
SELECT ws.id, ws.step_type, ws.config
FROM workflow_steps ws
JOIN workflow_step_edges wse ON wse.to_step_id = ws.id
WHERE ws.workflow_id = :workflow_id
AND NOT EXISTS (
  -- No incomplete upstream dependencies
  SELECT 1 FROM workflow_step_edges upstream
  JOIN step_executions se ON se.step_id = upstream.from_step_id 
    AND se.execution_id = :execution_id
  WHERE upstream.to_step_id = ws.id
  AND se.status != 'completed'
)
AND NOT EXISTS (
  -- Not already executed
  SELECT 1 FROM step_executions se
  WHERE se.step_id = ws.id AND se.execution_id = :execution_id
);
```

Sources:
- [Airflow Metadata Database (Astronomer)](https://www.astronomer.io/docs/learn/airflow-database)
- [Airflow Architecture Overview](https://airflow.apache.org/docs/apache-airflow/stable/core-concepts/overview.html)

---

## 14. When PostgreSQL Graph Patterns Break Down

**You need a dedicated graph DB when:**

| Signal | Threshold | Why |
|--------|-----------|-----|
| Traversal depth | >10 hops regularly | Recursive CTEs have combinatorial path explosion |
| Graph size + depth | >100M nodes with deep queries | Index-free adjacency in graph DBs wins decisively |
| Real-time graph algorithms | PageRank, community detection, betweenness centrality | No PostgreSQL extension provides these natively |
| Write-heavy graph mutations | Millions of edge updates/sec | Junction table indexes become bottleneck |
| Pattern matching complexity | Variable-length paths with complex filters | Cypher/GQL far more ergonomic than SQL |

**You do NOT need a dedicated graph DB when:**
- Your graphs have <1M nodes (your SaaS platform for years)
- Traversals are shallow (<5 hops) -- task deps, CRM networks, org trees
- Graph queries are a fraction of total queries (most are relational CRUD)
- You value operational simplicity over marginal graph query performance
- Multi-tenancy with RLS is a requirement (much simpler in PostgreSQL)

**For your platform specifically:** All five use cases (task deps, CRM networks, workflow DAGs, knowledge graphs, org hierarchies) are well within PostgreSQL's comfortable range. The knowledge graph / GraphRAG use case is the most likely to eventually push boundaries, but only at significant scale with deep reasoning chains. At that point, Apache AGE gives you Cypher syntax without leaving PostgreSQL.

Sources:
- [Understanding Scale Limitations of Graph Databases](https://www.thatdot.com/blog/understanding-the-scale-limitations-of-graph-databases/)
- [PostgreSQL Graph Database: Everything You Need To Know](https://www.puppygraph.com/blog/postgresql-graph-database)

---

## 15. SQLite Graph Capabilities

**SQLite supports recursive CTEs** (since 3.8.3, 2014), so the core graph traversal pattern works:

```sql
-- This works in SQLite
WITH RECURSIVE deps(id, depth) AS (
  SELECT depends_on_task_id, 1 FROM task_dependencies WHERE task_id = ?
  UNION ALL
  SELECT td.depends_on_task_id, d.depth + 1
  FROM task_dependencies td JOIN deps d ON td.task_id = d.id
  WHERE d.depth < 10
)
SELECT * FROM deps;
```

**What SQLite CANNOT do:**
- No `SEARCH` or `CYCLE` clauses (PostgreSQL 14+ only) -- must implement cycle detection manually with path arrays
- No ltree extension
- No Apache AGE / Cypher
- No pgvector (no vector similarity search)
- No GiST/GIN indexes for advanced pattern matching
- No Row Level Security for multi-tenancy
- Single-writer concurrency model limits throughput

**Practical assessment:** SQLite can handle basic task dependencies and simple hierarchies with recursive CTEs for a single-user or embedded scenario. For a multi-tenant SaaS platform, PostgreSQL is required -- you need RLS, concurrent writes, pgvector, and the extension ecosystem.

**Migration path for your project:** Since you're currently on SQLite (`dev_assets/db.sqlite`), the recursive CTE patterns shown above will work identically when you migrate to PostgreSQL. The adjacency list + edges table pattern is fully portable. You gain ltree, AGE, pgvector, and RLS on migration.

Sources:
- [Querying Tree Structures in SQLite (Charles Leifer)](https://charlesleifer.com/blog/querying-tree-structures-in-sqlite-using-python-and-the-transitive-closure-extension/)
- [SQLite Forum: Breadth-first graph traversal](https://sqlite.org/forum/info/3b309a9765636b79)

---

## Recommendation for Your Platform

**Phase 1 (Now, on SQLite):** Use adjacency list + recursive CTEs for task dependencies and workflow DAGs. This works today with zero new dependencies.

**Phase 2 (After PostgreSQL migration):** Add ltree for org hierarchies, pgvector for knowledge graph embeddings, and RLS for multi-tenancy. Implement the triple store schema for knowledge graphs.

**Phase 3 (If needed):** Add Apache AGE for complex graph queries where Cypher improves developer ergonomics. Implement GraphRAG with community detection in application code + PostgreSQL storage.

**Likely never needed:** A separate graph database. Your use cases are well within PostgreSQL's proven capabilities. The Alibaba benchmark (millisecond responses on 5 billion edges) provides substantial headroom.

---

## Bonus: SQL/PGQ (Future)

ISO SQL:2023 includes SQL/PGQ -- native graph pattern matching in SQL. PostgreSQL implementation is in progress. When it lands, you'll get declarative graph queries without any extension:
```sql
-- Future SQL/PGQ syntax (not yet in PostgreSQL)
SELECT * FROM GRAPH_TABLE (task_graph
  MATCH (t:Task)-[:DEPENDS_ON]->{1,5}(dep:Task)
  WHERE t.id = 'TASK-5'
  COLUMNS (dep.id, dep.title)
);
```

This would eliminate the need for Apache AGE entirely, as graph pattern matching becomes part of core SQL.

Sources:
- [Representing Graphs in PostgreSQL with SQL/PGQ (EDB)](https://www.enterprisedb.com/blog/representing-graphs-postgresql-sqlpgq)
- [ISO/IEC 9075-16:2023 SQL/PGQ Standard](https://www.iso.org/standard/79473.html)