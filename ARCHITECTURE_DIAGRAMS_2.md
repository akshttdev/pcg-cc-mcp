# PCG Architecture — Extended Diagrams
> 2026-03-25 | Supplement to ARCHITECTURE_ANALYSIS.md

---

## DIAG-01: Full API Route Map (Backend Surface)

```mermaid
mindmap
  root((PCG API\n:3000/api))
    AUTH
      POST /auth/login
      POST /auth/register
      GET  /auth/invite-info
      GET  /auth/me
    ORGANIZATIONS
      GET    /organizations
      POST   /organizations
      GET    /organizations/:id
      PATCH  /organizations/:id
      GET    /organizations/:id/brand-profile
      PUT    /organizations/:id/brand-profile
      POST   /organizations/:id/brand-research
      POST   /organizations/:id/seed-brand-project
      GET    /organizations/:id/knowledge
      POST   /organizations/:id/generate-invite
      GET    /organizations/:id/members
      POST   /organizations/:id/members
    COMPANIES
      GET    /companies
      POST   /companies
      GET    /companies/:id
      PATCH  /companies/:id
      GET    /companies/:id/brand-profile
      PUT    /companies/:id/brand-profile
      POST   /companies/:id/brand-research
      GET    /companies/:id/persons
    PERSONS
      GET    /persons
      POST   /persons
      GET    /persons/:id
      PATCH  /persons/:id
      DELETE /persons/:id
      POST   /persons/:id/research
      GET    /persons/:id/intelligence-status
      POST   /persons/:id/provision-org
      GET    /persons/:id/notes
      POST   /persons/:id/notes
      PATCH  /person-notes/:id
      DELETE /person-notes/:id
    CRM
      GET    /crm/pipelines
      POST   /crm/pipelines
      GET    /crm/pipelines/:id
      GET    /crm/pipelines/:id/kanban
      GET    /crm/deals
      POST   /crm/deals
      GET    /crm/deals/:id
      PATCH  /crm/deals/:id
      PATCH  /crm/deals/:id/stage
      GET    /crm/contacts
      POST   /crm/contacts
      PATCH  /crm/contacts/:id
      DELETE /crm/contacts/:id
    PROJECTS
      GET    /projects
      POST   /projects
      GET    /projects/:id
      PATCH  /projects/:id
      GET    /projects/:id/members
      POST   /projects/:id/members
      GET    /projects/:id/deliverables
      POST   /projects/:id/deliverables
      GET    /projects/:id/knowledge
      POST   /projects/:id/knowledge
      GET    /projects/:id/boards
      POST   /projects/:id/boards
      GET    /projects/:id/media
      POST   /projects/:id/media
      GET    /projects/:id/media/search
    TASKS
      GET    /boards/:id/tasks
      POST   /boards/:id/tasks
      PATCH  /tasks/:id
      DELETE /tasks/:id
    PROPOSALS
      GET    /proposals
      POST   /proposals
      GET    /proposals/:id
      PATCH  /proposals/:id
      PATCH  /proposals/:id/status
    INVOICES
      GET    /invoices
      POST   /invoices
      GET    /invoices/:id
      PATCH  /invoices/:id/status
      DELETE /invoices/:id
    DELIVERABLES
      GET    /deliverables/:id
      PATCH  /deliverables/:id
      PATCH  /deliverables/:id/status
      GET    /deliverables/:id/review-link
    INTAKE
      POST   /call-intake/email
      GET    /call-intake
      POST   /call-intake/:id/process
    BUSINESS-REPORTS
      GET    /business-reports
      POST   /business-reports/generate
    MARKETPLACE
      GET    /marketplace/listings
      POST   /marketplace/listings
      POST   /marketplace/gateway
      GET    /marketplace/stats
    SOCIAL
      GET    /social/accounts
      POST   /social/accounts
      GET    /social/accounts/:id/best-times
    COMMAND-CENTER
      GET    /command-center
    AI-AGENTS
      POST   /chat/topsi
      POST   /chat/nora
      POST   /call-intake/email
    VIBE
      POST   /vibe/deposit/verify
      POST   /vibe/faucet
      PATCH  /projects/:id/wallet
    REVIEW-PUBLIC
      GET    /review/:token
      POST   /review/:token/approve
      POST   /review/:token/reject
      POST   /review/:token/revision
    BIO-PUBLIC
      GET    /bio/:username
```

---

## DIAG-02: CRM Pipeline — 9-Stage State Machine

```mermaid
stateDiagram-v2
    [*] --> Intel : Deal Created

    Intel : 🔍 Intel\nResearch pass via Scout/Astra\nPerson + Company enrichment
    BusinessAnalysis : 📊 Business Analysis\nBusiness report generated\nMarket + competitive research
    Discovery : 🤝 Discovery\nProposal drafted\nBrand guide created
    Proposal : 💰 Proposal (Cash AI)\nProposal sent + signed\nTier confirmed
    Polish : ✨ Polish (Lux AI)\nPresentation deck built\nDeliverables scoped
    Present : 🎤 Present\nPresentation call\nTranscript captured
    FollowUp : 📞 Follow Up\nObjections addressed\nContract negotiated
    Won : 🏆 Won\nDeal closed\nProject scaffolded
    Lost : ❌ Lost\nReason logged\nNurture sequence

    Intel --> BusinessAnalysis : research_complete
    BusinessAnalysis --> Discovery : report_approved
    Discovery --> Proposal : proposal_drafted
    Proposal --> Polish : proposal_signed
    Polish --> Present : deck_ready
    Present --> FollowUp : call_complete
    FollowUp --> Won : contract_signed
    FollowUp --> Lost : deal_lost
    Proposal --> Lost : declined
    Discovery --> Lost : not_qualified

    note right of Intel
        AUTO-TRIGGERS (NOT IMPLEMENTED ⚠️)
        on_enter: POST /persons/:id/research
        on_enter: POST /companies/:id/brand-research
        assigned_agent: "Scout"
    end note

    note right of Proposal
        AUTO-TRIGGERS (NOT IMPLEMENTED ⚠️)
        on_enter: invoke Cash AI
        on_enter: draft proposal from template
        auto_trigger: "cash_ai_proposal"
    end note

    note right of Polish
        AUTO-TRIGGERS (NOT IMPLEMENTED ⚠️)
        on_enter: invoke Lux AI
        on_enter: generate deck
        auto_trigger: "lux_ai_polish"
    end note

    note right of Won
        SCAFFOLD (NOT IMPLEMENTED ⚠️)
        on_enter: create project_board
        on_enter: create deliverables from proposal
        on_enter: create tasks from deliverables
        on_enter: set projects.client_id
        on_enter: update KG with deal owner_type='deal'
    end note
```

---

## DIAG-03: Intake Pipeline — Email to Deal

```mermaid
sequenceDiagram
    participant E as 📧 Email (Nora inbox)
    participant I as call_intake_items
    participant N as Nora AI
    participant P as persons
    participant CO as companies
    participant CL as clients
    participant CC as crm_contacts
    participant CD as crm_deals
    participant KG as project_knowledge_sources
    participant T as tasks

    E->>I: POST /api/call-intake/email<br/>raw_content = email body

    Note over I: status = 'pending'

    I->>N: POST /api/call-intake/:id/process<br/>Nora extracts structured data

    N->>P: Find or create Person<br/>find_person_by_name_and_company()
    N->>CO: Find or create Company<br/>(companies table, not clients)

    Note over CL: ⚠️ clients row NOT created<br/>in current implementation

    N->>CC: Create crm_contact<br/>person_id = persons.id (TEXT)<br/>BUT pipeline_id often NULL

    N->>CD: Create crm_deal<br/>project_id = project BLOB<br/>client_id = NULL ⚠️<br/>stage = 'intel'

    Note over CD: deal_transcripts row linked<br/>intake_item_id → call_intake_items

    CD-->>KG: ⚠️ NOT auto-registered<br/>Must be done manually via<br/>POST /projects/:id/knowledge<br/>with owner_type='project'

    Note over KG: owner_id = project UUID<br/>(not deal UUID — no isolation)

    CD-->>T: ⚠️ NO tasks auto-created<br/>crm_deal_id never set on tasks
```

---

## DIAG-04: Won Deal → Delivery Scaffold (Intended vs. Actual)

```mermaid
flowchart LR
    subgraph TRIGGER["Deal Stage: Won"]
        W[PATCH /crm/deals/:id/stage\nstage = 'won'\nwon_at = now\nactual_close_date = now]
    end

    subgraph INTENDED_SCAFFOLD["✅ Should Auto-Scaffold"]
        direction TB
        S1[Read proposal linked to deal\nvia project_id] --> S2
        S2[For each proposal line item\nCreate deliverable row\ndeliverables.proposal_id = proposal.id\ndeliverables.project_id = project.id]
        S2 --> S3[Create project_board\nboard_type = 'delivery'\nname = deal.title]
        S3 --> S4[For each deliverable\nCreate task row\ntasks.board_id = board.id\ntasks.crm_deal_id = deal.id\ntasks.title = deliverable.title]
        S4 --> S5[Set projects.client_id\nfrom clients table\nvia crm_contact chain]
        S5 --> S6[Register deal KG sources\nowner_type = 'deal'\nowner_id = deal.id\nall transcripts + proposal artifacts]
        S6 --> S7[Assign account manager\ntasks.assigned_to = deal's AM\nproject.account_manager_id]
    end

    subgraph ACTUAL_SCAFFOLD["❌ Actually Happens"]
        direction TB
        A1[status column updated\nwon_at timestamp set] --> A2
        A2[⛔ Nothing else fires\nNo proposal lookup\nNo deliverable creation\nNo board creation\nNo task creation\nNo KG registration\nNo client_id set]
    end

    W --> INTENDED_SCAFFOLD
    W --> ACTUAL_SCAFFOLD

    style INTENDED_SCAFFOLD fill:#1a2e1a,stroke:#2d5a2d,color:#90ee90
    style ACTUAL_SCAFFOLD fill:#2e1a1a,stroke:#5a2d2d,color:#ff9999
    style TRIGGER fill:#1a1a2e,stroke:#2d2d5a,color:#9999ff
```

---

## DIAG-05: Organization Hierarchy & Data Ownership

```mermaid
flowchart TD
    subgraph PLATFORM["🌐 Platform Level"]
        U[users\nauth accounts\nJWT tokens]
        ORG[organizations\nworkspaces\ninvite_token\ncreated_by_org_id]
        CO[companies\nglobal knowledge graph\npublic brand entities]
        U -->|"org membership via\nproject_members / org_members"| ORG
        ORG -->|"company_id → companies\nthe agency's brand entity"| CO
    end

    subgraph ORG_SCOPE["🏢 Organization Scope"]
        P[projects\nwork containers\norg_id FK\nclient_id FK ⚠️ NULL]
        CL[clients\nper-org client records\norg_id FK\ncrm_contact_id FK]
        PIPE[crm_pipelines\nsales pipelines\norg_id OR project_id\nnot both simultaneously]
        ORG_BP[organization_brand_profiles\nthe agency's own brand\n1:1 with organizations]
        ORG --> P
        ORG --> CL
        ORG --> PIPE
        ORG --> ORG_BP
    end

    subgraph PROJECT_SCOPE["📁 Project Scope"]
        BOARD[project_boards\nkanban boards\nproject_id FK]
        TASK[tasks\nwork items\nboard_id FK\ncrm_deal_id FK ⚠️ NULL]
        DEL[deliverables\nclient artifacts\nproject_id FK\nproposal_id FK ⚠️ NULL]
        MEDIA[media assets\nDAM\nproject_id FK]
        KGP[project_knowledge_sources\nowner_type='project'\nowner_id=project_id]
        P --> BOARD
        P --> TASK
        P --> DEL
        P --> MEDIA
        P --> KGP
    end

    subgraph DEAL_SCOPE["💼 Deal Scope"]
        DEAL[crm_deals\npipeline cards\nstage_id FK\nproject_id FK\nclient_id FK ⚠️ NULL]
        PROP[proposals\nscope + pricing\nproject_id FK\norg_id ⚠️ NULL]
        TRANS[deal_transcripts\ncall records\ndeal_id FK\nintake_item_id FK]
        KGD["project_knowledge_sources\nowner_type='deal' ⚠️ unused\nowner_id=deal_id"]
        DEAL --> PROP
        DEAL --> TRANS
        DEAL -.->|"⚠️ SHOULD exist\nbut nothing writes here"| KGD
    end

    subgraph COMPANY_SCOPE["🏷️ Company Scope"]
        CO_BP[company_brand_profiles\nclient brand identity\n1:1 with companies]
        PER[persons\nuniversal identity\ncompany_id → companies]
        KGC[project_knowledge_sources\nowner_type='company'\nowner_id=company_id]
        CO --> CO_BP
        CO --> PER
        CO --> KGC
    end

    PIPE -->|"contains"| DEAL
    P -->|"client_id → clients\n⚠️ never set"| CL
    CL -->|"crm_contact_id → crm_contacts\n→ persons"| PER
    DEAL -->|"project_id → projects"| P

    style PLATFORM fill:#0d0d1a,stroke:#2d2d5a,color:#aaaaff
    style ORG_SCOPE fill:#0d1a0d,stroke:#2d5a2d,color:#aaffaa
    style PROJECT_SCOPE fill:#1a1a0d,stroke:#5a5a2d,color:#ffffaa
    style DEAL_SCOPE fill:#1a0d0d,stroke:#5a2d2d,color:#ffaaaa
    style COMPANY_SCOPE fill:#0d1a1a,stroke:#2d5a5a,color:#aaffff
```

---

## DIAG-06: Brand Guide System — Two Tracks

```mermaid
flowchart TD
    subgraph ORG_TRACK["🏢 Organization Brand Guide Track"]
        direction LR
        OBG[BrandGuidePage\n/organizations/:orgId/brand-guide]
        OBW[BrandSetupWizard\nno apiOverride]
        OBA[organizationsApi.getBrandProfile\nGET /api/organizations/:id/brand-profile]
        OBP[organization_brand_profiles\ntable]
        OBR[POST /api/organizations/:id/brand-research\n14 Exa searches → 8 scrapes → Claude]
        OBG --> OBA --> OBP
        OBG --> OBW --> OBR --> OBP
    end

    subgraph CO_TRACK["🏷️ Company Brand Guide Track"]
        direction LR
        CBG[CompanyBrandGuidePage\n/companies/:companyId/brand-guide]
        CBW[BrandSetupWizard\napiOverride = companiesApi]
        CBA[companiesApi.getBrandProfile\nGET /api/companies/:id/brand-profile]
        CBP[company_brand_profiles\ntable]
        CBR[POST /api/companies/:id/brand-research\n⚠️ DOES THIS EXIST?]
        CBG --> CBA --> CBP
        CBG --> CBW --> CBR
    end

    subgraph SHARED["🔗 Shared Components"]
        direction TB
        SECTIONS[Brand Guide Sections\nCoverPage\nFoundationPage\nVisualIdentityPage\nLogoSystemPage\nVoicePersonalityPage\nAudienceMarketPage\nApplicationsPage\nDigitalPresencePage\nBackCover]
        TYPES[BrandPageProps\nOrgBrandProfile interface\nBrandPageProps type]
        WIZARD[BrandSetupWizard\norgId + orgName\napiOverride?: BrandApiOverride]
    end

    OBG -->|"pageProps: BrandPageProps"| SECTIONS
    CBG -->|"pageProps: BrandPageProps"| SECTIONS
    OBW --> WIZARD
    CBW --> WIZARD

    subgraph GAPS_BRAND["⚠️ Brand Guide Gaps"]
        BG1["Company track missing:\nMoodBoardPage\nMerchandisePage\n(org track has them)"]
        BG2["company_brand_profiles.company_id\nstored as TEXT UUID\nbut companies.id is BLOB\n→ FK JOIN fails"]
        BG3["PUT /api/companies/:id/brand-profile\nFK constraint fails\ncompany_id BLOB vs TEXT mismatch"]
        BG4["No brand-research endpoint\nfor company track\n(org track has POST /organizations/:id/brand-research)"]
    end

    style ORG_TRACK fill:#1a1a2e,stroke:#2d2d5a,color:#aaaaff
    style CO_TRACK fill:#1a2e2e,stroke:#2d5a5a,color:#aaffff
    style SHARED fill:#1a2e1a,stroke:#2d5a2d,color:#aaffaa
    style GAPS_BRAND fill:#2e1a1a,stroke:#5a2d2d,color:#ffaaaa
```

---

## DIAG-07: Knowledge Graph — What Feeds It

```mermaid
flowchart LR
    subgraph SOURCES["📥 Data Sources → KG"]
        direction TB
        S1[Deliverable marked 'done'\n→ source_type='artifact'\nowner_type='project']
        S2[Brand research pass\n→ source_type='entity'\nowner_type='company' or 'organization']
        S3[Email/call intake\n→ source_type='conversation'\nowner_type='project' ⚠️ not 'deal']
        S4[Business report generated\n→ source_type='artifact'\nowner_type='project']
        S5[Manual POST /projects/:id/knowledge\n→ any source_type\nowner_type='project']
    end

    subgraph KG_TABLE["🧠 project_knowledge_sources"]
        direction TB
        KG1[owner_type: project]
        KG2[owner_type: company]
        KG3[owner_type: organization]
        KG4[owner_type: user]
        KG5["owner_type: deal ⚠️ NEVER WRITTEN"]
    end

    subgraph CONSUMERS["📤 KG Consumers"]
        direction TB
        C1[GET /projects/:id/knowledge\nReturns sources_by_type\nUsed in BrandGuidePage\nDigitalPresencePage]
        C2[GET /organizations/:id/knowledge\nOrg-level context]
        C3[Nora AI context injection\nwhen responding in project scope]
        C4["Deal card sidebar ⚠️ NOT IMPLEMENTED\nNo GET /crm/deals/:id/knowledge endpoint"]
        C5["Pipeline stage context ⚠️ NOT IMPLEMENTED\nNo KG fetch during auto-triggers"]
    end

    S1 --> KG1
    S2 --> KG2
    S2 --> KG3
    S3 --> KG1
    S4 --> KG1
    S5 --> KG1

    KG1 --> C1
    KG2 --> C2
    KG3 --> C2
    KG4 --> C3
    KG1 --> C3

    KG5 -.->|"should feed"| C4
    KG5 -.->|"should feed"| C5

    style SOURCES fill:#1a1a2e,stroke:#2d2d5a,color:#aaaaff
    style KG_TABLE fill:#1a2e1a,stroke:#2d5a2d,color:#aaffaa
    style CONSUMERS fill:#2e2e1a,stroke:#5a5a2d,color:#ffffaa
```

---

## DIAG-08: Automation System — What Runs vs. What Should

```mermaid
flowchart TD
    subgraph CRON["⏰ Hourly Cron (main.rs)"]
        direction TB
        CR1[automation_check_stale_knowledge\nmarks is_stale=1 if > 7 days old]
        CR2[automation_generate_weekly_reports\nif day_of_week = Monday]
        CR3[automation_sync_social_analytics\n⚠️ no connected accounts yet]
        CR4[automation_publish_scheduled_posts\n⚠️ no connected accounts yet]
        CR5[automation_update_pipeline_scores\nupdates deal health scores]
    end

    subgraph MISSING_TRIGGERS["❌ Missing Event-Driven Triggers"]
        direction TB
        MT1[Stage Transition Handler\nON PATCH /crm/deals/:id/stage\nread stage_config.on_enter_actions\nfire auto_trigger]
        MT2[Won Deal Scaffold\nON stage = 'won'\nauto-create board + tasks + deliverables]
        MT3[Proposal Signed Webhook\nON proposal.status = 'contract_signed'\nauto-advance deal to Polish]
        MT4[Deliverable Done → KG\nON deliverable.status = 'done'\nregister as KG artifact]
        MT5[New Intake → Research Queue\nON call_intake_items INSERT\nauto-POST /persons/:id/research]
        MT6[Intel Stage Enter → Scout\nON deal enters Intel stage\nqueue research pass]
    end

    subgraph MANUAL_WORKAROUNDS["🔧 Currently Done Manually"]
        direction TB
        MW1[Move deal stage via PATCH]
        MW2[Create boards manually in UI]
        MW3[Create tasks manually in UI]
        MW4[Run brand research via UI wizard]
        MW5[Register KG sources via direct SQL]
        MW6[Create deliverables manually]
    end

    CRON -->|"runs but has limited impact\nno deal-level events"| MW1
    MT1 -.->|"⚠️ not implemented"| MW1
    MT2 -.->|"⚠️ not implemented"| MW2
    MT2 -.->|"⚠️ not implemented"| MW3
    MT4 -.->|"partially implemented\nfor org deliverables only"| MW5
    MT6 -.->|"⚠️ not implemented"| MW4

    style CRON fill:#1a2e1a,stroke:#2d5a2d,color:#aaffaa
    style MISSING_TRIGGERS fill:#2e1a1a,stroke:#5a2d2d,color:#ffaaaa
    style MANUAL_WORKAROUNDS fill:#2e2e1a,stroke:#5a5a2d,color:#ffffaa
```

---

## DIAG-09: Invoice & Financial Flow

```mermaid
flowchart TD
    subgraph PROPOSAL_FINANCIAL["💰 Proposal (Contract Value)"]
        P[proposals\namount REAL\ncurrency TEXT\ntier TEXT\nstatus = contract_signed\nsigned_at TIMESTAMP]
    end

    subgraph INVOICE_FINANCIAL["🧾 Invoice (Billing)"]
        I[invoices\nperson_id FK\nproject_id FK\namount_usd REAL\nvibe_amount REAL\nstatus: pending|paid|overdue\npaid_at TIMESTAMP]
    end

    subgraph VIBE_FINANCIAL["🔮 VIBE (Internal Currency)"]
        V[vibe_transactions\nproject_id FK\nuser_id FK\ntransaction_type\namount REAL\ndescription]
        VB[users.vibe_balance\nREAL]
        VBP[projects.client_budget_vibe\nREAL]
    end

    subgraph MARKETPLACE_FINANCIAL["🛒 Marketplace Revenue"]
        M[marketplace_subscriptions\nlisting_id FK\nsubscriber_org_id FK\nvibe_per_call REAL]
        MR[gateway_requests\nsubscription_id FK\nvibe_charged REAL\nprovider_vibe REAL]
        PR[PeerReward tasks\n85% to provider\n15% platform]
    end

    subgraph GAPS_FINANCIAL["⚠️ Financial Gaps"]
        FG1["proposals ↔ invoices\nNO FK link\nCannot auto-generate invoice from signed proposal"]
        FG2["invoices.project_id\nbut NO invoices.deal_id\nCannot attribute invoice to CRM deal"]
        FG3["proposals.amount\nvs\ninvoices.amount_usd\nNo reconciliation path"]
        FG4["VIBE_PER_USD = 100\nbut vibe_transactions not\nlinked to invoices or proposals"]
    end

    P -.->|"⚠️ no auto-creation"| I
    P -->|"amount informs"| VBP
    I --> V
    V --> VB
    M --> MR --> PR

    style PROPOSAL_FINANCIAL fill:#1a2e1a,stroke:#2d5a2d,color:#aaffaa
    style INVOICE_FINANCIAL fill:#2e2e1a,stroke:#5a5a2d,color:#ffffaa
    style VIBE_FINANCIAL fill:#1a1a2e,stroke:#2d2d5a,color:#aaaaff
    style MARKETPLACE_FINANCIAL fill:#2e1a2e,stroke:#5a2d5a,color:#ffaaff
    style GAPS_FINANCIAL fill:#2e1a1a,stroke:#5a2d2d,color:#ffaaaa
```

---

## DIAG-10: CRM Contact → Person → User Merge Paths

```mermaid
flowchart LR
    subgraph EXTERNAL["🌐 External Entity"]
        E1[Lead arrives via email]
        E2[Lead from Nora call]
        E3[Manual CRM entry]
    end

    subgraph PERSON_LAYER["👤 persons table\nUniversal Identity"]
        P[persons\nfull_name\nemail\nphones JSON\ncompany_id → companies\nassigned_to → users\nperson_type\nintelligence_summary\nresearch_pass_count\nfinancial_role]
    end

    subgraph CONTACT_LAYER["📋 crm_contacts table\nPipeline Card Identity"]
        C[crm_contacts\nperson_id TEXT → persons\ncrm_pipeline_id\ncontact_name ⚠️ duplicated\ncontact_email ⚠️ duplicated\ncompany ⚠️ duplicated\nstage\nnotes\ntags]
    end

    subgraph USER_LAYER["🔐 users table\nPlatform Auth Identity"]
        U[users\nemail ⚠️ duplicated\nfull_name ⚠️ duplicated\nuser_role\nvibe_balance\npassword_hash]
    end

    subgraph CLIENT_LAYER["🏢 clients table\nOrg-Billing Identity"]
        CL[clients\nname ⚠️ duplicated\nemail ⚠️ duplicated\nphone ⚠️ duplicated\norganization_id\ncrm_contact_id → crm_contacts]
    end

    E1 --> P
    E2 --> P
    E3 --> C

    P -.->|"person_id TEXT\nno FK\nno cascade"| C
    P -.->|"user_id TEXT\nno FK\nno cascade"| U

    C -->|"crm_contact_id BLOB\n✅ declared FK"| CL

    NOTE1["⚠️ 4 tables all store name+email\nChanging email in users\ndoes NOT update persons\nor crm_contacts or clients"]

    NOTE2["⚠️ 'Sign up via invite'\ncreates a users row\nbut does NOT link to\nexisting persons row\nfor the same person"]

    NOTE3["⚠️ Proposals reference\ncontact_ids as JSON array\nof crm_contact IDs\nbut the UI shows person intelligence\nrequiring a JOIN through\ncrm_contacts.person_id"]

    style PERSON_LAYER fill:#1a2e2e,stroke:#2d5a5a,color:#aaffff
    style CONTACT_LAYER fill:#2e2e1a,stroke:#5a5a2d,color:#ffffaa
    style USER_LAYER fill:#1a1a2e,stroke:#2d2d5a,color:#aaaaff
    style CLIENT_LAYER fill:#2e1a2e,stroke:#5a2d5a,color:#ffaaff
    style EXTERNAL fill:#0d0d0d,stroke:#333333,color:#aaaaaa
```

---

## DIAG-11: Dealflow — Bad Girl Strength Club Actual State

```mermaid
flowchart TD
    subgraph BGSC["Bad Girl Strength Club — Live Data State 2026-03-25"]
        direction TB

        COMPANY["🏷️ companies\nBad Girl Strength Club\nid: c09a44b2-2d34-4c92-bfbe-4dfc70515b5a\nwebsite: badgirlstrength.club\nheadquarters: Houston TX\n✅ intelligence_summary populated"]

        PERSON["👤 persons\nStephanie Bruno\n✅ email populated\n✅ intelligence_summary populated\n✅ company_id → Bad Girl SC\n✅ research_pass_count > 0"]

        BRAND["🎨 company_brand_profiles\nprimary: #BB2423\nsecondary: #000000\naccent: #ED1C24\nheading: Syne\nbody: Questrial\ntagline: Get Badass Fast.\n✅ logo_url populated\n⚠️ company_id stored as TEXT UUID\n⚠️ PUT API broken (FK BLOB mismatch)"]

        DEAL["💼 crm_deals\nBad Girl Strength Club - Tier 2\namount: $14,500\nstage: won\nwon_at: 2026-03-14\nactual_close_date: 2026-03-14 00:00:00.000\n⚠️ client_id = NULL\n✅ project_id populated\n✅ crm_stage_id → Won stage"]

        PROJECT["📁 projects\nBad Girl Strength Club\n⚠️ client_id = NULL\n✅ organization_id → Sirak Studios"]

        PROPOSAL["📄 proposals\nTier 2 Brand Build Package\namount: 14500\nstatus: contract_signed\n⚠️ organization_id = NULL\n✅ project_id → project"]

        DELIVERABLES["📦 deliverables (5)\n1. Brand Kit (graphic, pending)\n2. VSL + Landing Page (document, pending)\n3. App Content Creation (copy, pending)\n4. Sales Funnel Build (document, pending)\n5. Social Strategy (copy, pending)\n⚠️ ALL proposal_id = NULL\n✅ project_id → project"]

        BOARD["🗂️ project_boards\n'Bad Girl SC — Delivery'\nboard_type: delivery\n✅ project_id → project"]

        TASKS["✅ tasks (5)\n1. Brand Kit Design\n2. VSL + Landing Page Build\n3. App Content Creation\n4. Sales Funnel Build\n5. Social Strategy\n⚠️ ALL crm_deal_id = NULL\n⚠️ board_id → board? VERIFY"]

        TRANSCRIPTS["📞 deal_transcripts (3)\n1. Initial discovery call\n2. Presentation call 2026-03-14\n3. Final contract email\n✅ deal_id → crm_deal\n✅ intake_item_id → call_intake_items"]

        KG["🧠 project_knowledge_sources (9)\nowner_type: project\nowner_id: project UUID\nsources: logo, photos, website,\ndecks, proposals, transcripts\n⚠️ NOT queryable from deal card\n⚠️ owner_type should also be 'deal'"]

        COMPANY --> BRAND
        COMPANY --> PERSON
        PERSON --> DEAL
        DEAL --> PROPOSAL
        DEAL --> TRANSCRIPTS
        DEAL --> PROJECT
        PROJECT --> DELIVERABLES
        PROJECT --> BOARD
        BOARD --> TASKS
        PROJECT --> KG
    end

    subgraph STATUS["Status Legend"]
        OK["✅ Correctly populated"]
        WARN["⚠️ Present but misconfigured"]
        MISS["❌ Missing entirely"]
    end

    MISS_LIST["❌ Missing:\n• clients row for Bad Girl SC\n• crm_contacts.person_id → Stephanie\n• deliverables.proposal_id\n• tasks.crm_deal_id\n• KG owner_type='deal' registration\n• proposals.organization_id"]

    style COMPANY fill:#1a2e2e,stroke:#2d5a5a,color:#aaffff
    style PERSON fill:#1a2e1a,stroke:#2d5a2d,color:#aaffaa
    style BRAND fill:#2e2e1a,stroke:#5a5a2d,color:#ffffaa
    style DEAL fill:#1a1a2e,stroke:#2d2d5a,color:#aaaaff
    style PROJECT fill:#2e1a2e,stroke:#5a2d5a,color:#ffaaff
    style PROPOSAL fill:#1a2e1a,stroke:#2d5a2d,color:#aaffaa
    style DELIVERABLES fill:#2e2e1a,stroke:#5a5a2d,color:#ffffaa
    style BOARD fill:#1a2e2e,stroke:#2d5a5a,color:#aaffff
    style TASKS fill:#2e1a1a,stroke:#5a2d2d,color:#ffaaaa
    style TRANSCRIPTS fill:#1a1a2e,stroke:#2d2d5a,color:#aaaaff
    style KG fill:#2e1a2e,stroke:#5a2d5a,color:#ffaaff
    style MISS_LIST fill:#3a1a1a,stroke:#aa3333,color:#ff9999
```

---

## DIAG-12: Proposed Target Architecture (Post-Fix)

```mermaid
erDiagram
    users ||--o| persons : "user_id BLOB FK (enforced)"
    persons ||--o{ crm_contacts : "person_id BLOB FK (enforced)"
    crm_contacts ||--o| clients : "crm_contact_id BLOB FK"
    clients }|--|| organizations : "organization_id"
    organizations ||--o{ projects : "organization_id"
    projects }|--o| clients : "client_id (auto-set on deal create)"
    crm_pipelines }|--|| organizations : "organization_id"
    crm_pipeline_stages }|--|| crm_pipelines : "pipeline_id"
    crm_deals }|--|| crm_pipeline_stages : "crm_stage_id"
    crm_deals }|--|| projects : "project_id"
    crm_deals }|--|| clients : "client_id (auto-set)"
    proposals }|--|| projects : "project_id"
    proposals }|--|| organizations : "organization_id (auto-set)"
    deliverables }|--|| projects : "project_id"
    deliverables }|--o| proposals : "proposal_id (auto-set on Won)"
    project_boards }|--|| projects : "project_id"
    tasks }|--|| project_boards : "board_id"
    tasks }|--o| crm_deals : "crm_deal_id (auto-set on Won)"
    tasks }|--o| deliverables : "deliverable_id (NEW FK)"
    project_knowledge_sources }|--|| crm_deals : "owner_type='deal' (NEW)"
    project_knowledge_sources }|--|| projects : "owner_type='project'"
    project_knowledge_sources }|--|| companies : "owner_type='company'"
    deal_stage_transitions }|--|| crm_deals : "deal_id (NEW TABLE)"
    companies ||--o| company_brand_profiles : "company_id (fix to BLOB)"
    organizations ||--o| organization_brand_profiles : "organization_id"
```

---

*End of Extended Diagrams — PCG Architecture v0.2*
