

# Graph Databases & Knowledge Graph Technologies — Research Report (Early 2026)

## 1. Neo4j

**Current Capabilities:** Neo4j remains the dominant property graph database. Version 5.x (current as of early 2026) brought significant improvements: composite databases, server-side routing, improved Cypher query planning, and vector search integration (added mid-2024). Neo4j now supports hybrid graph+vector queries natively, positioning it for RAG/AI workloads.

**Rust Drivers:** The official `neo4rs` crate is the primary Rust driver. It supports Bolt protocol, transactions, streaming results, and connection pooling. Maturity is moderate — it works for production use but the ecosystem is smaller than Java/Python/JS. Community maintains it actively.

**Pricing & Hosting:**
- **Community Edition:** Free, open-source (GPLv3), single-instance only, no clustering
- **Enterprise Edition:** Commercial license, clustering, role-based access, hot backups. Pricing is per-core, typically $60k-$150k+/year for production deployments
- **AuraDB (managed):** Free tier (small), Professional (~$65/mo starting), Enterprise (custom pricing). AuraDB is the path of least resistance for hosted
- **Self-hosted:** Runs on Linux/Docker/K8s. Operationally heavier than SQLite/Postgres — requires JVM tuning, memory management, backup strategies

**Performance at Scale:** Excellent for traversal-heavy queries (multi-hop relationships, shortest path, community detection). Handles billions of nodes/edges with proper hardware. Weaker for bulk analytical scans compared to columnar stores. Index-free adjacency gives O(1) relationship traversal regardless of graph size.

## 2. SurrealDB

**Multi-Model:** SurrealDB is the most interesting newcomer for a Rust-native project. It combines document, graph, relational, and time-series models in one database. Written in Rust. Supports graph traversal via `->relation->` syntax in SurrealQL, which is SQL-like.

**Current Maturity (early 2026):** SurrealDB reached 2.x in late 2025. It is production-usable for moderate workloads but still young compared to Neo4j or PostgreSQL. Key concerns:
- Graph query optimizer is less mature than Neo4j's (simpler traversals work well; complex graph algorithms may need external tooling)
- The Rust SDK (`surrealdb` crate) is first-class and well-maintained
- Embedded mode available — can run in-process like SQLite (using RocksDB or in-memory backend)
- Clustering/distributed mode exists but is less battle-tested than single-node

**Fit for ORCHA:** High relevance. SurrealDB's embedded mode means you could potentially replace SQLite with SurrealDB and get graph capabilities without running a separate service. The Rust-native SDK is a strong plus. Risk: maturity. For critical production data, it's still newer than the alternatives.

## 3. EdgeDB

**Graph-Relational Hybrid:** EdgeDB sits on top of PostgreSQL but provides a schema language with first-class links (relationships) and a query language (EdgeQL) that handles deep traversals naturally. It is not a pure graph database — it is a relational database with ergonomic graph-like queries.

**Knowledge Graph Fit:** Good for structured schemas where relationships are known at design time. Less suitable for highly dynamic knowledge graphs where edge types proliferate. EdgeDB shines for the "typed schema with relationships" use case — which maps well to CRM, task dependencies, and org hierarchies.

**Considerations:** EdgeDB runs as a service on top of Postgres. It has a Rust client (`edgedb-tokio`). The operational cost is: you run EdgeDB server + it manages its own Postgres instance underneath. This is heavier than just using Postgres directly.

## 4. Apache AGE (A Graph Extension)

**What It Is:** A PostgreSQL extension that adds OpenCypher query support. You can write Cypher queries against graph data stored in PostgreSQL tables. This is the "extend your existing relational DB" approach.

**Production Readiness:** AGE graduated from Apache Incubator. It works with PostgreSQL 13-16. The extension is stable for basic graph queries. Limitations:
- Performance is not comparable to Neo4j for deep traversals (no index-free adjacency)
- Complex graph algorithms (PageRank, community detection) must be implemented externally
- Works with standard PostgreSQL tooling, backups, replication — no new operational burden

**Fit for ORCHA:** If you move from SQLite to PostgreSQL (or already use it), AGE lets you add graph queries without a new service. This is the lowest-friction path to adding graph capabilities. However, AGE does not work with SQLite.

**SQLite Alternative:** There is no mature SQLite graph extension equivalent to AGE. The closest is using recursive CTEs in SQLite for graph traversal (which works but is verbose and limited).

## 5. Dgraph

**Current State:** Dgraph has had a turbulent history. After the company (Dgraph Labs) pivoted and had leadership changes, the open-source project has slowed. The database itself works — it is distributed, supports GraphQL natively, and handles large graphs. But community momentum has decreased significantly compared to 2021-2022.

**Verdict:** Not recommended for new projects in 2026 unless you have specific distributed-graph-with-native-GraphQL requirements. The uncertainty around long-term maintenance is a concern.

## 6. TerminusDB

**Version-Controlled Graph:** TerminusDB's unique feature is Git-like version control for graph data — branching, merging, diffing datasets. Written in Prolog/Rust. Interesting for collaboration-heavy knowledge management.

**Practical Fit:** Niche. The version control concept is powerful for data provenance but adds complexity. The ecosystem is small. Better suited for data science/knowledge management teams than for application backends.

## 7. Knowledge Graphs for AI Agents

This is the highest-growth area in graph databases as of early 2026. Key patterns:

**Agent Memory:** Graph databases store agent conversation history, learned facts, and entity relationships as connected nodes. This allows agents to retrieve relevant context through graph traversal rather than flat vector search alone.

**Reasoning:** Multi-hop reasoning ("Alice works at CompanyX, CompanyX is in IndustryY, IndustryY trends are Z") is natural in graph form. Agents can traverse relationship chains that would require multiple SQL JOINs or embedding-based retrieval wouldn't capture at all.

**GraphRAG (Microsoft Research pattern, popularized 2024-2025):**
1. Extract entities and relationships from documents into a knowledge graph
2. Build community summaries at different granularity levels
3. At query time, combine graph traversal with vector search for context retrieval
4. Significantly outperforms naive RAG for questions requiring synthesis across documents

**Practical Implementation:** Most production systems use a hybrid: vector store for semantic search + graph for structured relationships. The graph doesn't need to be a dedicated graph DB — a relational database with proper relationship tables often suffices for moderate-scale agent memory.

## 8. Graph Databases for Dependency Tracking

**Task Dependencies (DAGs):** This is a classic graph use case. Key operations:
- Topological sort (execution order)
- Critical path analysis
- Cycle detection
- Impact analysis ("what breaks if this task is delayed?")

**Implementation Options:**
- **SQLite/Postgres adjacency table:** A `task_dependencies(task_id, depends_on_id)` table + recursive CTEs handles most DAG operations. This is sufficient for projects with hundreds to low thousands of tasks.
- **Dedicated graph DB:** Only justified if you need real-time traversal of very large dependency graphs (10k+ nodes) or complex graph algorithms beyond basic traversal.

**Org Hierarchies:** Tree structures (org charts, project hierarchies) are easily modeled with adjacency lists or nested sets in relational DBs. Graph DBs add value only when hierarchies become polyhierarchies (nodes with multiple parents) or when you need to traverse cross-hierarchy relationships.

## 9. Graph + Vector Search (Hybrid Approaches)

**GraphRAG:** As mentioned, this combines knowledge graph structure with embedding-based retrieval. The pattern is now well-established:
- Neo4j added native vector index support (2024)
- LangChain/LlamaIndex both have GraphRAG integrations
- Microsoft open-sourced their GraphRAG implementation

**Practical Hybrid Architecture:**
```
Query → Vector search (find semantically similar content)
      → Graph traversal (find structurally related entities)
      → Merge & re-rank results
      → Feed to LLM as context
```

**For ORCHA:** If agents need to reason about relationships between contacts, tasks, deals, and workflows, a graph layer (even a simple one) combined with vector search will outperform either approach alone.

## 10. Graph Databases for CRM

**Relationship Mapping:** CRM is inherently a graph problem — contacts know other contacts, belong to companies, are involved in deals, have communication history. Graph databases excel at:
- "Show me all contacts 2 hops from this deal"
- "Find the shortest connection path between me and this prospect"
- Network analysis (who are the most connected contacts?)
- Influence mapping

**Reality Check:** Most CRMs (including Salesforce, HubSpot) use relational databases with JOIN-heavy queries. Graph databases add value when relationship discovery and traversal are primary use cases, not just data storage. For ORCHA's CRM module, the existing relational model with proper relationship tables is likely sufficient unless you plan to build LinkedIn-style relationship intelligence features.

## 11. Embedding Graph Structure in Existing Databases

**Adjacency Lists in SQLite/PostgreSQL:**

```sql
-- Task dependencies
CREATE TABLE task_dependencies (
    task_id TEXT NOT NULL,
    depends_on_id TEXT NOT NULL,
    dependency_type TEXT DEFAULT 'blocks', -- blocks, related, subtask
    PRIMARY KEY (task_id, depends_on_id)
);

-- Recursive CTE for transitive dependencies
WITH RECURSIVE deps AS (
    SELECT depends_on_id, 1 as depth
    FROM task_dependencies WHERE task_id = ?
    UNION ALL
    SELECT td.depends_on_id, d.depth + 1
    FROM task_dependencies td
    JOIN deps d ON td.task_id = d.depends_on_id
    WHERE d.depth < 10  -- prevent infinite recursion
)
SELECT * FROM deps;
```

**Performance:** SQLite handles recursive CTEs well for graphs up to ~10k nodes. PostgreSQL handles larger graphs and has better query planning for complex traversals. Both support cycle detection via the `CYCLE` clause (Postgres 14+) or tracking visited nodes.

**Limitations:**
- No index-free adjacency — each hop requires an index lookup
- Complex graph algorithms (shortest path, PageRank, community detection) are painful in SQL
- No graph visualization query language (Cypher/Gremlin patterns)

**Verdict for ORCHA:** Start here. Add relationship tables to your existing SQLite schema. Move to a dedicated graph DB only when you hit specific limitations.

## 12. Property Graphs vs RDF

**Property Graphs** (Neo4j, SurrealDB, AGE model):
- Nodes and edges have properties (key-value pairs)
- Intuitive for developers — feels like objects with relationships
- Query languages: Cypher, Gremlin, SurrealQL
- Best for: application data, CRM, PM, workflows — anything where you think in "entities and relationships"

**RDF** (triple stores — Apache Jena, Stardog, Blazegraph):
- Subject-Predicate-Object triples
- Built for semantic web, ontologies, linked open data
- Query language: SPARQL
- Best for: knowledge representation, ontologies, data integration across heterogeneous sources

**For Business Applications (ORCHA):** Property graphs win decisively. RDF's semantic web origins make it overly complex for CRM/PM/workflow data. Property graphs map naturally to your domain model.

## 13. Graph Visualization Libraries

**D3.js Force-Directed Graphs:** Most flexible, lowest level. You control everything. Steep learning curve. Best for custom, highly interactive visualizations. Not React-friendly out of the box.

**Cytoscape.js:** Purpose-built for graph/network visualization. Rich layout algorithms (force-directed, hierarchical, circular). Good performance up to ~5k nodes. Has a React wrapper (`react-cytoscapejs`). Good fit for dependency graphs and network diagrams.

**React Flow:** Designed for node-based editors (workflow builders, flowcharts). Excellent React integration. Built-in support for drag-and-drop, minimap, controls. **Already well-suited for ORCHA's workflow editor.** Less suitable for large network visualizations.

**vis.js / vis-network:** Good middle ground. Handles large networks (10k+ nodes) with WebGL rendering. Less React-native than React Flow.

**Sigma.js:** WebGL-based, handles very large graphs (100k+ nodes). Good for CRM network visualization where you need to render many connections.

**Recommendation for ORCHA:**
- Workflow DAGs: React Flow (you may already use this)
- Task dependency graphs: Cytoscape.js (good layout algorithms for DAGs)
- CRM contact networks: Cytoscape.js or Sigma.js depending on scale
- Org hierarchies: React Flow or a simple tree component

## 14. TypeDB / Vaticle

**Type-Theoretic Knowledge Graph:** TypeDB uses a type system based on a conceptual data model (Entity-Relationship extended with type hierarchies, roles, and rules). It has its own query language, TypeQL.

**Categorical/Topos Theory Connection:** TypeDB's type system has conceptual overlap with category theory — types form categories, relationships are morphisms, inheritance is functorial. However, TypeDB does not explicitly implement categorical or topos-theoretic foundations. The connection is more philosophical than practical. If you want actual categorical database theory, look at CQL (Categorical Query Language) from Conexus AI — but that is research-grade, not production-ready.

**Practical Assessment:** TypeDB is powerful for complex domain modeling (drug discovery, fraud detection, knowledge management). For ORCHA's use cases (CRM, PM, workflows), it is overengineered. The operational overhead and smaller ecosystem do not justify the modeling power for typical business application graphs.

## 15. Cost of Adding a Graph Layer

**Operational Overhead Assessment:**

| Approach | New Service? | Ops Burden | Migration Effort | Best For |
|---|---|---|---|---|
| Adjacency tables in SQLite | No | Zero | Low | ORCHA today |
| Recursive CTEs in PostgreSQL | Only if migrating from SQLite | Low | Medium | Moderate graph needs |
| Apache AGE on PostgreSQL | No (extension) | Low | Medium | Cypher queries on Postgres |
| SurrealDB (embedded) | No | Low-Medium | High | Rust-native, multi-model |
| SurrealDB (server) | Yes | Medium | High | Multi-model with scaling |
| Neo4j | Yes | High | High | Graph-first workloads |

**Hidden Costs:**
- Data synchronization: If graph DB is alongside relational DB, you need to keep them in sync (dual writes, CDC, or ETL)
- Schema evolution: Graph schemas evolve differently than relational schemas — need separate migration strategies
- Query complexity: Developers need to learn Cypher/GraphQL/SurrealQL
- Monitoring: Another service to monitor, alert on, back up

---

## Recommendation for ORCHA

### Phase 1 (Now): Adjacency Tables in SQLite

Add graph-like relationship tables to your existing SQLite database:

- `task_dependencies(task_id, depends_on_id, type)` — for DAG task dependencies
- `contact_relationships(contact_id_a, contact_id_b, relationship_type)` — for CRM contact networks
- `workflow_edges(workflow_id, from_node_id, to_node_id, condition)` — if not already present

Use recursive CTEs for traversal. This covers 80% of graph needs with zero operational overhead.

### Phase 2 (When Needed): PostgreSQL + Apache AGE

If/when ORCHA outgrows SQLite (concurrent writes, larger datasets, need for full-text search + graph), migrate to PostgreSQL and add the AGE extension. This gives you Cypher queries without a separate service.

### Phase 3 (If Graph Becomes Core): SurrealDB or Neo4j

Only if graph traversal, agent knowledge graphs, or relationship intelligence become a primary product feature (not just a supporting capability) should you consider a dedicated graph database. SurrealDB is the natural choice for a Rust project (embedded mode, native SDK). Neo4j if you need the most mature graph ecosystem.

### For Agent Knowledge Graphs Specifically

The GraphRAG pattern does not require a graph database. You can implement it with:
1. Entity/relationship extraction into relational tables
2. Community detection as a batch job (output stored in tables)
3. Vector search for semantic retrieval (can use SQLite with `sqlite-vec` or similar)
4. Graph traversal via recursive CTEs for structural context

This keeps the architecture simple while getting most of the GraphRAG benefits.

### For Visualization

Add Cytoscape.js for dependency graph and CRM network visualization. Continue using React Flow (or equivalent) for the workflow editor. These are frontend-only additions with no backend infrastructure changes.

---

**Bottom Line:** ORCHA should not add a dedicated graph database today. The combination of adjacency tables in SQLite + recursive CTEs + a good visualization library (Cytoscape.js) covers task dependencies, CRM networks, workflow DAGs, org hierarchies, and basic agent knowledge graphs. The threshold for adding a dedicated graph DB is when graph traversal performance on SQLite becomes a measurable bottleneck, or when relationship intelligence becomes a flagship feature requiring complex graph algorithms (community detection, influence scoring, path analysis) that are impractical in SQL.