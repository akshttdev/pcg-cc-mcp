# Spanish Conversation Workflow Demo — Planning & Research

**Date:** 2026-03-15
**Branch:** `e2e/dogfood-demo-replay-2026-03-14`
**Depends on:** Demo 4 (CRM pipeline) passing end-to-end
**Goal:** Demonstrate multilingual workflow capabilities by extracting CRM data from a Spanish business conversation, showcasing translation + extraction in a single pipeline.

---

## Research Summary

### What the engine supports today

The workflow engine processes nodes via topological sort. Each LLM node (`llm_extract`, `llm_analyze`, `llm_summarize`) accepts a fully customizable `prompt_template` with `{{content}}` and `{{previous_results}}` placeholders. Output nodes (`output_crm_contacts`, `output_crm_companies`, `output_crm_deals`) stage extracted JSON into the CRM review pipeline.

There is no dedicated "Translate" node type — but LLM Extract nodes can combine translation + extraction in a single prompt. The system prompt enforces anti-hallucination rules ("only extract entities explicitly mentioned") which remain active alongside custom prompts.

### Key architectural details

- **Prompt template**: Custom text + auto-appended target schemas (contacts[], companies[], deals[])
- **Output mode**: `structured` (JSON) or `text` (raw); defaults to `auto` (structured when schemas present)
- **Chaining**: Nodes chain via `connections`; downstream nodes receive `{{previous_results}}` from upstream
- **Staging**: Records get confidence scores, email-based dedup (contacts), name-based dedup (companies/deals)
- **Commit fields**: Contacts need `first_name`, `last_name`, `email`; Companies need `name`; Deals need `name`, `amount`

### Design decision: single-pass vs multi-step

| Approach | Pros | Cons |
|----------|------|------|
| **Single-pass** (translate + extract in one prompt) | Simpler workflow, fewer LLM calls, lower cost | Less transparent, harder to debug translation vs extraction |
| **Multi-step** (Node 1: translate → Node 2: extract) | Clear separation of concerns, reusable translation step, better for demo storytelling | More nodes, higher cost, `text` output mode needed for translation node |

**Decision:** Use a **hybrid approach** — a single workflow with 4 LLM nodes that each handle translation + extraction for their entity type (contacts, companies, deals) plus one dedicated translation/summary node. This maximizes demo value by showing both chaining and custom prompts.

---

## Workflow Design

### Pipeline: "Spanish Conversation CRM Extract"

```
┌──────────────┐
│ Data Source   │ (Spanish conversation transcript)
└──────┬───────┘
       │
       ├──────────────────────────────────┐
       │                                  │
┌──────▼──────────┐              ┌────────▼─────────┐
│ Translate &     │              │ Extract Companies │
│ Summarize       │              │ (Spanish→English) │
│ (llm_summarize) │              │ (llm_extract)     │
│ output: text    │              │ schema: companies[]│
└──────┬──────────┘              └────────┬──────────┘
       │                                  │
┌──────▼──────────┐              ┌────────▼──────────┐
│ Extract Contacts│              │ Output: Companies  │
│ (from summary)  │              │ (output_crm_       │
│ (llm_extract)   │              │  companies)        │
│ schema:contacts[]│             └────────────────────┘
└──────┬──────────┘
       │
       ├──────────────────────────┐
       │                          │
┌──────▼──────────┐      ┌───────▼────────┐
│ Output: Contacts│      │ Extract Deals  │
│ (output_crm_    │      │ (from contacts │
│  contacts)      │      │  + source)     │
└─────────────────┘      │ schema: deals[]│
                         └───────┬────────┘
                                 │
                         ┌───────▼────────┐
                         │ Output: Deals  │
                         │ (output_crm_   │
                         │  deals)        │
                         └────────────────┘
```

### Node details

**Node 1: Translate & Summarize** (`llm_summarize`, output_mode: `text`)
```
Prompt: "The following text is a business conversation in Spanish.
Translate the full conversation to English, preserving all names,
emails, phone numbers, company names, monetary amounts, and
deal details exactly as stated. Output the full English translation."
```
Purpose: Creates a clean English transcript that downstream nodes can extract from. This demonstrates chaining — the translation output feeds into extraction nodes.

**Node 2: Extract Contacts** (`llm_extract`, schema: `contacts[]`, from: Node 1)
```
Prompt: "Extract all people mentioned as business contacts from this
translated conversation. For each person provide: first_name, last_name,
email, phone, company_name, job_title. Preserve original Spanish names
(do not anglicize). Return JSON array."
```

**Node 3: Extract Companies** (`llm_extract`, schema: `companies[]`, from: Data Source)
```
Prompt: "This text is a Spanish business conversation. Extract all
companies and organizations mentioned. For each: name, industry
(inferred from context), description (one sentence in English about
their business). Return JSON array."
```
Note: This node reads directly from the data source (not the translation), demonstrating that LLM nodes can handle Spanish input directly.

**Node 4: Extract Deals** (`llm_extract`, schema: `deals[]`, from: Node 2)
```
Prompt: "Based on the extracted contacts and the original conversation,
identify all potential deals or business opportunities. For each:
name (descriptive title), amount (numeric value), currency (USD/EUR),
contact_name (associated contact), company_name, stage (discovery/
qualification/proposal). Return JSON array."
```

**Output nodes:** Three output nodes (contacts, companies, deals) connected to their respective extract nodes.

---

## Spanish Conversation Content

A realistic business conversation between a Spanish-speaking client and a bilingual account manager:

```
Transcripci'on de Reuni'on — Expansi'on Latinoam'erica
Fecha: 2026-03-12
Participantes: Ana Morales (Powerclub Global), Carlos Ramirez (TechSoluciones),
               Elena Varga (Grupo Andino)

Ana Morales: Buenos d'ias, Carlos y Elena. Gracias por conectarse hoy.
Quiero hablar sobre la expansi'on de nuestra plataforma en Am'erica Latina.

Carlos Ram'irez: Gracias, Ana. En TechSoluciones tenemos un equipo de
150 ingenieros y estamos buscando una soluci'on de gesti'on de proyectos
m'as robusta. Mi correo es carlos.ramirez@techsoluciones.com y mi
tel'efono es +52-555-0198.

Elena Varga: Desde Grupo Andino, representamos a tres subsidiarias en
Colombia, Per'u y Chile. Nuestro presupuesto para herramientas digitales
es de $350,000 d'olares anuales. Pueden contactarme en
elena.varga@grupoandino.com.

Carlos Ram'irez: Nuestro director de tecnolog'ia, Miguel Santos,
tambi'en estar'ia interesado. Su correo es miguel.santos@techsoluciones.com.
Estamos considerando un contrato inicial de $180,000 d'olares para el
primer a~no.

Elena Varga: Adem'as, tenemos una iniciativa de transformaci'on digital
por $500,000 que incluir'ia capacitaci'on y despliegue en las tres oficinas.

Ana Morales: Excelente. Tambi'en quiero presentarles a nuestro
especialista regional, Diego Herrera — diego.herrera@powerclub.global.
'El coordinar'a la implementaci'on.

Carlos Ram'irez: Perfecto. Nuestra empresa matriz, Innovaci'on Global S.A.,
podr'ia expandir esto a nivel corporativo — potencialmente $1.2 millones.
```

### Expected extraction results

**Contacts (5):**
| first_name | last_name | email | company_name | job_title |
|------------|-----------|-------|--------------|-----------|
| Carlos | Ramirez | carlos.ramirez@techsoluciones.com | TechSoluciones | — |
| Elena | Varga | elena.varga@grupoandino.com | Grupo Andino | — |
| Miguel | Santos | miguel.santos@techsoluciones.com | TechSoluciones | Director de Tecnologia / CTO |
| Diego | Herrera | diego.herrera@powerclub.global | Powerclub Global | Regional Specialist |
| Ana | Morales | — | Powerclub Global | — |

**Companies (4):**
| name | industry |
|------|----------|
| TechSoluciones | Technology / Engineering |
| Grupo Andino | Conglomerate / Holdings |
| Powerclub Global | Project Management / SaaS |
| Innovacion Global S.A. | Technology (parent company) |

**Deals (3):**
| name | amount | currency | contact |
|------|--------|----------|---------|
| TechSoluciones Initial Contract | 180,000 | USD | Carlos Ramirez |
| Grupo Andino Digital Transformation | 500,000 | USD | Elena Varga |
| Innovacion Global Corporate Expansion | 1,200,000 | USD | Carlos Ramirez |

---

## Demo Script Structure

**File:** `e2e/demos/workflow-spanish-pipeline.spec.ts`

### Parts

1. **Part 1: Build Spanish CRM extraction workflow** (~120s timeout)
   - Navigate to /workflows → click "New Workflow"
   - Create workflow with 4 LLM nodes + 3 output nodes
   - Demonstrate the translation → extraction chain
   - Save workflow

2. **Part 2: Create Spanish conversation data source**
   - Navigate to org intelligence page
   - Add data source with Spanish conversation content
   - Verify source appears in list

3. **Part 3: Run workflow against Spanish data source** (~120s timeout)
   - Navigate to data sources, select the Spanish conversation
   - Run the CRM extract workflow
   - Wait for completion, verify staged records > 0

4. **Part 4: Review staged records**
   - Navigate to Staging tab
   - Verify records show translated/extracted data
   - Approve records (batch approve via API fallback if needed)
   - Commit to CRM

5. **Part 5: Verify CRM contacts**
   - Navigate to org CRM contacts page
   - Verify Carlos Ramirez, Elena Varga, Miguel Santos appear
   - Open a contact detail modal

6. **Part 6: Verify CRM pipeline deals**
   - Navigate to org CRM pipeline
   - Verify deals with correct amounts appear
   - Verify contact associations

### Cleanup
- Delete workflow definition via API
- Delete data source via API
- Delete staged/committed CRM records via API

---

## Implementation Notes

### Differences from Demo 4 (English CRM pipeline)

| Aspect | Demo 4 (English) | Demo 5 (Spanish) |
|--------|-------------------|-------------------|
| Language | English | Spanish |
| Translation node | None | Yes — translate & summarize |
| Node chaining | Flat (all from Data Source) | Chained (translate → extract) |
| Node count | 3 extract + 3 output = 6 | 4 LLM + 3 output = 7 |
| Prompt complexity | Simple extraction | Translation + extraction |
| Companies node | From Data Source | Direct from Data Source (Spanish) |
| Verification | Name matching | Name matching (Spanish names preserved) |

### Risk areas

1. **LLM translation quality**: The system prompt says "only extract entities explicitly mentioned" which works well, but translation quality depends on the model. Need to use prompts that produce consistent output.
2. **Accent/encoding**: The conversation uses ASCII-safe approximations (e.g., `'e` instead of `é`). If we use actual Unicode (`é`, `ñ`, `í`), need to ensure the data source content is handled correctly through the pipeline.
3. **Node builder UI**: Adding 7 nodes via UI is time-intensive. May need increased timeout (180s) for Part 1.
4. **Chaining reliability**: Node 2 depends on Node 1's text output. If Node 1's translation is poor, extraction will fail. The `{{previous_results}}` placeholder must correctly pass the translated text.

### Decision: Use proper Unicode

Use proper Spanish characters (`é`, `ñ`, `í`, `ó`, `ú`, `ü`) in the conversation content. The workflow engine stores content as UTF-8 strings in SQLite, and the frontend handles Unicode correctly. This makes the demo more realistic and tests the full character pipeline.

---

## Implementation Order

1. [ ] Ensure Demo 4 passes end-to-end (FK fix validation)
2. [ ] Create `e2e/demos/workflow-spanish-pipeline.spec.ts`
3. [ ] Add Spanish conversation constant with proper Unicode
4. [ ] Implement Part 1: Build workflow with translation chain
5. [ ] Implement Parts 2-3: Data source creation + workflow run
6. [ ] Implement Parts 4-6: Staging review + CRM verification
7. [ ] Add cleanup logic
8. [ ] Run full demo suite: all 5 demos passing

---

## Open Questions

1. **Separate demo or extend Demo 4?** → Separate demo script (cleaner, independent, can run in isolation)
2. **CRM pipeline for deals?** → Reuse existing default pipeline or create a "LATAM Pipeline" for the demo?
3. **Translation node output_mode**: Should be `text` (not `structured`) since translation produces prose, not JSON. Verify the UI supports selecting output mode per node.
