# Founding Team Q&A — Roadmap Input Sessions

**Date**: 2026-03-18
**Participants**: PCG Founding Team + Claude Research Sprint
**Purpose**: Clarifying questions to inform the 5-year product roadmap. All answers directly shaped stage sequencing, priority ordering, revenue projections, and strategic decisions in `5-year-product-roadmap.md`.

---

## Batch 1: Business Model & Revenue Identity

**Q1. Is PCG currently generating agency revenue? If so, roughly how much monthly?**
Yes. Somewhere between $5-20K per month, but it fluctuates.

**Q2. Who are PCG's current/recent clients? Agency type? Deal sizes?**
Client list spans creative, tech consulting, and marketing — PCG is a multi-faceted agency. Deals between $500-$15K.

**Q3. Is ORCHA intended primarily as (a) an internal tool, (b) a product for other agencies, (c) a platform play, or (d) all three in sequence?**
D — all three in sequence.

**Q4. How important is the "sovereign data" positioning vs. just being a great AI-powered agency tool?**
It is a core platform goal, but not as important as speed to market and revenue.

**Q5. For pricing: do you lean toward seat-based, usage-based, or hybrid?**
Hybrid will probably be necessary because of different cost variables.

**Q6. Are there existing competitors you admire or want to position against?**
Render and Akash have come up but they aren't perfect comps.

**Q7. What's your gut on open-source vs. proprietary?**
Explored options in detail (see analysis below). Decision: **Option D→C** — Proprietary now, transition to Source-Available (BSL) at seed round. Rationale: 2-3 person team can't afford OSS community management overhead; moat is integration depth not code; strategic OSS at seed for MCP servers + APN protocol specs.

**Q8. Is there an existing PCG client who would be a day-1 pilot user of ORCHA?**
Yes. Sirak Studios.

**Q9. What's the minimum monthly revenue target that would let the founding team work full-time on ORCHA?**
Something like $10K/month. Assume team is full-time now.

**Q10. Is Sirak Studios a co-development partner, a client, or both?**
Sirak is a client who also invests.

---

## Batch 2: Team & Hiring

**Q11. Current team composition — who does what?**
Full-stack lead with 15 years of experience across software and startups. Another developer with less experience but capable. And another vision/business/sales person who leads the project and has vibe coding experience.

**Q12. Is anyone on the team besides you writing Rust?**
Everyone writes Rust but the senior dev maintains and QAs.

**Q13. First hire priority — if you could add one person tomorrow, what role?**
Deferred to recommendation. Recommendation provided: DevOps/Platform Engineer as hire #1 ($3-6K/mo remote). Response: "That's fine for now, we can review later."

**Q14. Are you using Claude Code as a force multiplier for development? If so, roughly what % of code output comes from AI-assisted development?**
Yes. 90% of code is from Claude across the team. May use multiple agent processes concurrently and dogfooding can further speed up development when achieved.

**Q15. Any advisors or investors besides Sirak?**
We have a network. Assume generic advisors for now and we can add specifics in the future when it matters.

**Q16. Geographic constraints on hiring?**
Generally flexible on hiring. Cost optimize, we have remote team experience.

**Q17. Equity vs cash compensation?**
Potentially but let's start with cash projections.

**Q18. What's your personal tolerance for fundraising?**
Raise fast but based on solid foundations and hitting key metrics.

---

## Batch 3: Technical Priority Trade-offs

**Q19. Agent Flow Orchestration Engine is the P0 blocker. Is this the #1 thing you want to build next?**
We can ignore existing roadmap for priorities. It is just context documentation.

**Q20. PostgreSQL migration — prerequisite for external users or can it wait?**
PostgreSQL is fully optional unless there are functional requirements. We should also consider other DB tech. Probably will need a hybrid strategy.

**Q21. APN mesh — demo feature, near-term product, or long-term differentiator?**
Long term.

**Q22. VIBE token economy — fundraising narrative, near-term product, or long-term vision?**
Long term but much closer than APN. This is part of the revenue model where VIBE tokens can have a pass-through markup on LLM API costs. Users pay in VIBE tokens, we settle in API credits and keep the difference.

**Q23. Workflow triggers (cron/event-driven) — how important for first pilots?**
Workflow triggers seem important for automation, but we should be using ROI-based prioritization to settle it.

**Q24. The 6 missing workflow node types — which do pilots need in the first 3 months?**
This is a research question. Probably too granular for this level of planning.

**Q25. Nora (voice AI assistant) — priority for early revenue or demo feature?**
This is a core feature, part wow and part utility. Could be streamlined.

**Q26. VIBELAND MMO / 3D features — active development or parked?**
This is not currently prioritized but part of the long-term vision.

**Q27. Content pipeline (9 social platforms) — revenue-generating for agency clients?**
Social media management is one of the current revenue generators that could be expanded in the platform.

**Q28. Editron (post-production pipeline) — active revenue feature or parked?**
Editron is one of the biggest current revenue generators.

---

## Batch 4: Product Vision & End State

**Q29. In one sentence, what is ORCHA in 5 years?**
Vision to reality as a service.

**Q30. Who is the primary end user?**
All use in different roles. PCG is an agency so we currently have use cases across those roles (agency owner, PM, developer, creative professional).

**Q31. Editron generates revenue today — what does the workflow look like?**
Partially manual, migrating in-platform.

**Q32. Social media management generates revenue — same question.**
Mostly outside but migrating in.

**Q33. The VIBE token markup model — have you thought about the markup percentage?**
Base model is 2x markup but some benchmarks at different numbers might be helpful.

**Q34. Do you envision ORCHA as a single product or a suite?**
ORCHA is mostly a dashboard tool. There is a suite of additional products in the ecosystem around it (i.e., VIBE Land, hardware products, and self-sovereign data infra). Those are all parts of PCG but more layered on top of ORCHA.

---

## Batch 5: Revenue-Critical Details

**Q35. Desktop app (Tauri) — important for users or web-first sufficient?**
We would like to have desktop app in the roadmap. It doesn't have to be an initial priority.

**Q36. The 2x VIBE markup — for all API usage or only external users?**
External only.

**Q37. Editron revenue — roughly what range monthly? Would clients pay for tool access or deliverables?**
Not sure on monthly numbers, in thousands to teens. Clients pay for deliverables from internal PCG operators.

**Q38. Social media revenue — same question.**
Same. The plan is to offer as a service when platform stabilizes. We already support agency/client model.

**Q39. Of your current $5-20K/month, roughly what % comes from Editron, social media, and other services?**
Not sure. There are also website creation and hosting services in the mix.

**Q40. The PCG ecosystem (VIBE Land, hardware, sovereign data infra) — generating revenue now?**
Some hardware generates revenue (PCG/Omega Wireless). The rest are future vision.

**Q41. Hardware products — what kind?**
Compute nodes and mesh networking. It is intended for running APN nodes.

**Q42. When you say "vision to reality as a service" — is the core value proposition: "Tell us what you want, and our AI-powered platform + human team makes it happen"?**
That sounds about right.

---

## Batch 6: Constraints & Risk Tolerance

**Q43. What's your biggest fear about the next 12 months?**
All of the above (running out of money, building wrong thing, competitor launching first, team burnout).

**Q44. What's the one thing that, if it goes wrong, kills the project?**
Not launching product that works as soon as possible.

**Q45. How much technical debt are you willing to carry for speed?**
Cleanup where it facilitates speed. Otherwise some tech debt is acceptable.

**Q46. Regulatory concerns?**
Not at the moment, would not prioritize that.

**Q47. IP protection — patents filed or planned?**
We should pursue patent strategy as part of the plan if possible.

**Q48. Timeline pressure — any external deadline?**
No specific deadline, just move with speed as a principle. Referenced "Speed as a Habit" article (Dave Girouard, First Round Review).

---

## Batch 7: Final Clarifications

**Q49. Omega Wireless / hardware — shipping now? How many units?**
There is revenue here but treat this as out of scope, a separate complementary project, unless it is critical as part of the roadmap stage.

**Q50. Website creation and hosting services — standard web dev or platform component?**
We want to offer services like Replit/Lovable/Render but with a better interface for specialists and better implementations of AI.

**Q51. VIBE token float mechanism — pre-paid credits, floating-rate, or actual crypto?**
C — they are actual crypto tokens. Include concerns in the roadmap or ask additional questions if critical.

**Q52. Patent strategy — most patentable elements?**
We can explore this as we go, can be generic for now.

**Q53. Nora voice assistant — primary interface for non-technical users?**
Voice-first should be an important principle, but we are still working through core functionality first.

**Q54. How important is "self-sovereign data" for target customers (agencies)?**
Will need to gather more data on this. Should be part of the roadmap.

**Q55. The 9-agent roster — all staying or consolidate? Do users need to know agent names?**
Names are useful as a shorthand for the task domain, but this doesn't need to be a literal mapping to specific agents (or even to different agents fundamentally). We want to make orchestration both invisible and familiar.

**Q56. Is there anything critical for the 5-year roadmap that the questions haven't surfaced?**
Possibly but we can always revise the roadmap if we missed something.

---

## Key Decisions Derived from Q&A

| Decision | Derived From | Applied In Roadmap |
|----------|-------------|-------------------|
| Hybrid agency→SaaS revenue model | Q1, Q3, Q37-39 | Stage sequencing (agency first, SaaS at Stage 2) |
| VIBE 2x token markup (external only) | Q22, Q33, Q36, Q51 | Revenue projections, Stage 2-3 priorities |
| Proprietary→Source-Available licensing | Q7 (detailed analysis) | Stage 3 licensing transition |
| Editron + social media capture first | Q27, Q28, Q31, Q32 | Stage 0 Month 1-2 priorities |
| Voice-first design principle | Q25, Q53 | Cross-cutting principle, Stage 1 Nora MVP |
| Replit/Lovable/Render-class hosting | Q50 | Stage 1-2 hosting service |
| DevOps as first hire | Q13 (recommendation) | Stage 1 team plan |
| Speed as a habit | Q18, Q44, Q48 | Guiding principle #1 |
| Desktop app in roadmap but not initial | Q35 | Stage 3 priority |
| APN long-term, VIBE closer | Q21, Q22 | Stage 5 vs Stage 2-3 |
| Actual crypto tokens on Aptos | Q51 | VIBE concerns section, regulatory flags |
| Patent strategy generic for now | Q47, Q52 | Stage 3 patent timeline |
| Data sovereignty research needed | Q4, Q54 | Stage 3 research priority |
| Agent names as domain shorthand | Q55 | Agent identity strategy |
| $10K/mo minimum viable revenue | Q9 | Stage 0→1 gate |
| Sirak Studios as first pilot | Q8, Q10 | Stage 1 pilot plan |
| 90% AI-generated code | Q14 | Team size assumptions |
| All roles use ORCHA | Q30 | UX design implications |
| Hardware out of scope unless critical | Q49 | APN references Omega but doesn't depend on it |

---

*This document is an appendix to the 5-year product roadmap. It preserves the reasoning behind key decisions for future reference when the roadmap is revised.*
