# Editron Production Readiness Roadmap
_Last updated: 2025-12-23_

This roadmap captures the remaining work to ship Editron as a production-ready autonomous post-production lane. Items are grouped by functional area with explicit owners, dependencies, and acceptance criteria.

## 1. Dropbox Integration
| Item | Status | Owner | Notes |
| --- | --- | --- | --- |
| Register webhook + verify requests | IN PROGRESS | Infra | HMAC verification + secret wiring shipped; remaining work is Dropbox app registration |
| Dropbox source registry (account → ingest mapping) | DONE | Backend | `dropbox_sources` table powers auto-ingest without manual payload hints |
| Content change polling fallback | DONE | Infra | Dropbox background monitor re-queues ingest for stale accounts every few minutes |
| Auto-mapping of inbound folders → project metadata | TODO | Ops | Config file or DB mapping from Dropbox paths to project IDs, feed into webhook hints |

## 2. Media Pipeline Enhancements
| Item | Status | Owner | Notes |
| Download service resiliency | TODO | Backend | Retry with exponential backoff, resume partial downloads, track throughput metrics |
| Actual analysis stack (Whisper/CLIP) | TODO | ML | Replace placeholder hero-moment generation with transcripts + embeddings |
| Session templating + Premiere/iMovie automation | TODO | Video | Formalize blueprint schema, call ExtendScript/AppleScript |

## 3. Workflow + Tasking
| Item | Status | Owner | Notes |
| Workflow orchestrator script for Editron | TODO | Backend | Define stages (ingest → analyze → edit → render) and map to Nora workflows |
| Project board automation | TODO | Product | Auto-create tasks per stage w/ SSE updates |

## 4. Frontend & Operator Experience
| Item | Status | Owner | Notes |
| Media batch dashboard | TODO | Frontend | Table view with status, updated timestamps, CTA to open analysis |
| Analysis report viewer | TODO | Frontend | Render hero moments, transcripts, recommended deliverables |
| Render job monitor | TODO | Frontend | Progress bars, error surfaces, manual retry |

## 5. Observability & QA
| Item | Status | Owner | Notes |
| Structured logging for pipeline events | TODO | Backend | Emit tracing spans per batch/session/job |
| Integration tests | TODO | QA | Add `cargo test` suites for pipeline DB persistence and webhook flows |
| Load testing | TODO | Infra | Validate ingest of large >50GB batches |

## 6. Deployment & Ops
| Item | Status | Owner | Notes |
| Secrets management | TODO | DevOps | Move Dropbox/API keys to vault/1Password CLI |
| Asset cleanup policies | TODO | Infra | TTL for `media_pipeline` directories + DB rows |
| Release checklist | TODO | Product | Document runbooks, alarms, rollback steps |

---

### Near-Term Priority Stack
1. **Dropbox security + registration** – unstoppable to get real events flowing.
2. **Analysis engine** – provides tangible value; start with Whisper transcripts + CLIP highlight scoring.
3. **Workflow automation** – ensures Nora orchestrates stages without manual intervention.
4. **Operator dashboard** – surfaces progress/errors to humans.

We will tackle these sequentially while keeping PRs scoped and testable.
