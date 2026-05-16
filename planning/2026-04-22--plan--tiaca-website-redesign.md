# TIACA.org — Complete Website Redesign
**Type**: plan
**Date**: 2026-04-22 (revised)
**Client**: The International Air Cargo Association (TIACA)
**Project**: TIACA.org — Full Custom Next.js Build + Headless CMS + Admin Panel
**Contract scope**: Phase 1 fixed deliverable — $9,800
**Strategic mandate**: Over-deliver. This is a gateway project. Build something so exceptional they commission two more immediately and it anchors the international roadshow.
**Dashboard client record**: `5303d1cd-2845-43c7-bbbf-e39cf8ca08bb`
**Primary contact**: Glyn Hughes `ghughes@tiaca.org` · Web/content partner: Rachel Negron `rnegron@tiaca.org`

---

## 1. Strategic Context

TIACA.org is the institutional foundation of the entire engagement. It is the persistent destination that all other work — the Warsaw landing page, ACF event site, TNN, LinkedIn campaigns, sponsorship decks — ultimately routes traffic to. If the foundation is weak, everything built on top of it underperforms.

**The current site's failure**: It is informational but not compelling. It lists things rather than telling a story. It has no clear conversion path. It looks 5-6 years stale by Glyn's own description. The nav is complex, the copy is institutional boilerplate, and there is no coherent "why TIACA" narrative visible above the fold.

**What we are building instead**: A premium, narrative-led institutional website that:
- Leads with TIACA's *why* — the industry-wide mission and the irreplaceable role TIACA plays
- Positions every product (ACF, Executive Summit, BlueSky Accreditation, Membership, Training) as essential, not optional
- Converts the right visitors into the right actions: attend ACF, join as a member, apply for BlueSky, sponsor
- Functions as the persistent media hub: newsroom, Cargo Pulse, press releases, TNN (when live)
- Elevates copy to executive-level — nothing generic, nothing passive

**This is not a 10-page brochure site.** The contract scopes 10 core pages, but the real deliverable is a complete CMS-powered platform with a custom admin panel, full content migration from WordPress, and a content model that scales to everything TIACA does for years. We are building the long-term infrastructure. The contract price is an investment in a long-term partner.

**BlueSky evolution**: BlueSky is being repositioned from a sustainability self-assessment *program* into a formal industry *accreditation*. Companies apply, are assessed against 8 sustainability objectives, and receive official TIACA BlueSky Accreditation status valid for two years. This is a significant product evolution — it becomes a revenue driver and a differentiator. The website must reflect this new positioning throughout.

---

## 2. Final Tech Stack

| Layer | Decision | Rationale |
|---|---|---|
| **Framework** | Next.js 15 (App Router) | Full-stack, SSR + static, API routes collocated, Vercel-native |
| **Language** | TypeScript throughout | Type safety critical for admin CMS operations |
| **Database** | Vercel Postgres (Neon) | Managed, scalable, collocated with Vercel deployment |
| **ORM** | Prisma | Best-in-class schema management, migrations, type-safe queries |
| **Auth** | NextAuth.js v5 | Admin panel auth, credentials provider for Rachel/Riley login |
| **Styling** | Tailwind CSS + shadcn/ui | Fast build, consistent component library, matches admin panel standards |
| **Animation** | Framer Motion | Scroll animations, page transitions — premium feel |
| **Media storage** | Vercel Blob | Collocated storage for uploaded images/files via admin panel |
| **Email** | Resend | Transactional emails from forms, notifications |
| **CMS** | Custom (built into Next.js) | Admin panel at `/admin` — no third-party CMS dependency |
| **Deployment** | Vercel (Sirak account → transfer to TIACA at handoff) | Auto-CDN, preview deploys, domain management |
| **Initial URL** | `tiaca.vercel.app` | Working preview; DNS cutover to `tiaca.org` at handoff |
| **Analytics** | Vercel Analytics + GA4 | Both: Vercel for Core Web Vitals, GA4 for campaign tracking |
| **SEO** | Next.js Metadata API + `next-sitemap` | Auto-generated sitemap, OG tags, structured data |

**API architecture**: Next.js Route Handlers only — no separate API subdomain. All CMS operations, form submissions, and admin actions go through `/api/*` routes within the same Next.js app. Vercel Postgres connection is collocated. This is simpler, faster to build, and easier to secure than a separate Rust API for this use case.

---

## 3. Page Architecture

Consolidating from 170+ WordPress pages to a focused, narrative-driven structure. Core pages are built and designed. Secondary/archive pages are CMS-generated (template-driven, not individually designed).

### Core Pages (designed + built individually)

| # | Route | Title | Primary CTA | Notes |
|---|---|---|---|---|
| 1 | `/` | Home | Register ACF · Join TIACA | Full narrative hero, TIACA why, initiative showcase, news feed, member logos |
| 2 | `/about` | About TIACA | Explore Membership | Mission/vision, 70-yr heritage, pillars, team, board (consolidates Who We Are + What We Do) |
| 3 | `/membership` | Membership | Book a Call with Kenneth | 4 tiers, value props per tier, member logos, Kenneth CTA |
| 4 | `/events` | Events | Register ACF · Learn About Summit | Hub page: ACF 2026, Executive Summit 2026, regional events, past events archive |
| 5 | `/air-cargo-forum` | Air Cargo Forum 2026 | Register · Sponsor · Exhibit | Conversion bridge page → links out to dedicated ACF site |
| 6 | `/executive-summit` | Executive Summit 2026 | Learn More · Sponsor | Conversion bridge page → links out to dedicated ES site |
| 7 | `/bluesky` | BlueSky Accreditation | Apply for Accreditation | Repositioned as formal accreditation — 8 criteria, accredited member directory, application CTA |
| 8 | `/newsroom` | Newsroom | Subscribe | Press Releases, Cargo Pulse, TIACA In the News, Mission Sustainability, Member News, TNN placeholder |
| 9 | `/training` | Training Library | Browse Courses | Native catalog — 46 courses by category, external provider links, no checkout this phase |
| 10 | `/awards` | Awards & Hall of Fame | Nominate · View Hall of Fame | Leadership Award, Rising Star Award, HoF directory |
| 11 | `/contact` | Contact | Send Message | Team contacts, HQ address, social links, media inquiry form |
| +1 | `/hello` | TIACA 101 — VSL Landing | Join · Register ACF | Conversion-only page, no global nav, TIACA 101 video embed |

### CMS-Generated Pages (template-driven, no individual design)

| Route Pattern | Content Type | Source |
|---|---|---|
| `/newsroom/[slug]` | All news articles (900+ migrated) | WP export |
| `/hall-of-fame/[slug]` | Individual HoF profiles (50+) | WP export |
| `/board-of-directors/[slug]` | Board member profiles (~15) | WP export |
| `/training/[slug]` | Individual course pages (46) | WP product export |
| `/awards/[slug]` | Award pages (Leadership, Rising Star) | CMS |
| `/legal` | Legal | CMS |
| `/privacy` | Privacy Policy | CMS |
| `/terms` | Terms of Service | CMS |

### Removed / Consolidated (do not rebuild)

- `/job-postings/` — placeholder from 2018, no active jobs. Remove.
- `/clusters/` — stale 2018 pages. Fold into About if relevant.
- `/cargo-service-quality-home-page/`, `/how-does-csq-work/` — legacy tool. Fold into Training or remove.
- `/vacscene/`, `/sunrays/` — stale programs. Archive-redirect to About.
- Hotel booking pages — legacy WC Bookings. Remove entirely.
- Shop, Cart, Checkout, My Account — WooCommerce artifacts. Remove; training is catalog-only.
- `/elementor-12443/` — orphaned Elementor page. Remove.

---

## 4. CMS Data Model (Prisma Schema)

All content managed through the admin panel at `/admin`. Rachel and Riley have different permission levels.

### Content Types

```prisma
// Admin users
model AdminUser {
  id        String   @id @default(cuid())
  email     String   @unique
  name      String
  role      AdminRole // SUPER_ADMIN | EDITOR
  password  String   // hashed
  createdAt DateTime @default(now())
  updatedAt DateTime @updatedAt
}

enum AdminRole {
  SUPER_ADMIN  // Rachel — full access
  EDITOR       // Riley — can publish content, cannot edit core pages or settings
}

// Core pages (non-article content)
model Page {
  id          String   @id @default(cuid())
  slug        String   @unique
  title       String
  metaTitle   String?
  metaDesc    String?
  ogImage     String?
  blocks      Json     // block-based content (array of block objects)
  published   Boolean  @default(false)
  publishedAt DateTime?
  updatedAt   DateTime @updatedAt
  createdAt   DateTime @default(now())
}

// News articles (press releases, cargo pulse, in the news, etc.)
model Article {
  id          String      @id @default(cuid())
  slug        String      @unique
  title       String
  excerpt     String?
  content     String      // rich text HTML
  category    ArticleCategory
  coverImage  String?
  author      String?
  externalUrl String?     // for "In the News" — links to third-party article
  metaTitle   String?
  metaDesc    String?
  published   Boolean     @default(false)
  publishedAt DateTime?
  createdAt   DateTime    @default(now())
  updatedAt   DateTime    @updatedAt
  tags        Tag[]
}

enum ArticleCategory {
  PRESS_RELEASE
  CARGO_PULSE
  TIACA_IN_THE_NEWS
  MISSION_SUSTAINABILITY
  MEMBER_NEWS
  TNN  // placeholder for future
}

// Hall of Fame profiles
model HallOfFameProfile {
  id          String   @id @default(cuid())
  slug        String   @unique
  name        String
  title       String?
  company     String?
  bio         String   // rich text
  photo       String?
  yearInducted Int?
  sortOrder   Int      @default(0)
  published   Boolean  @default(true)
  createdAt   DateTime @default(now())
  updatedAt   DateTime @updatedAt
}

// Board of Directors profiles
model BoardMember {
  id          String   @id @default(cuid())
  slug        String   @unique
  name        String
  title       String
  company     String
  bio         String?
  photo       String?
  boardRole   String?  // Chair, Vice Chair, Treasurer, etc.
  sortOrder   Int      @default(0)
  isCurrent   Boolean  @default(true)
  createdAt   DateTime @default(now())
  updatedAt   DateTime @updatedAt
}

// Team member profiles (staff)
model TeamMember {
  id        String   @id @default(cuid())
  slug      String   @unique
  name      String
  title     String
  email     String?
  photo     String?
  bio       String?
  sortOrder Int      @default(0)
  isActive  Boolean  @default(true)
  createdAt DateTime @default(now())
  updatedAt DateTime @updatedAt
}

// Training courses (catalog — no e-commerce)
model Course {
  id           String   @id @default(cuid())
  slug         String   @unique
  title        String
  excerpt      String?
  description  String?
  provider     String?  // ICAO, SASI, etc.
  level        CourseLevel? // BEGINNER | INTERMEDIATE | ADVANCED
  category     String?
  externalUrl  String   // links to external platform
  coverImage   String?
  duration     String?
  sortOrder    Int      @default(0)
  published    Boolean  @default(true)
  createdAt    DateTime @default(now())
  updatedAt    DateTime @updatedAt
}

enum CourseLevel {
  BEGINNER
  INTERMEDIATE
  ADVANCED
  ALL_LEVELS
}

// BlueSky Accreditation applications
model BlueSkyApplication {
  id            String              @id @default(cuid())
  companyName   String
  contactName   String
  contactEmail  String
  contactPhone  String?
  country       String?
  companySize   String?
  website       String?
  notes         String?
  status        BlueSkyStatus       @default(PENDING)
  submittedAt   DateTime            @default(now())
  reviewedAt    DateTime?
  accreditedAt  DateTime?
  expiresAt     DateTime?
  reviewNotes   String?
  updatedAt     DateTime            @updatedAt
}

enum BlueSkyStatus {
  PENDING        // submitted, not yet reviewed
  IN_REVIEW      // assessor reviewing
  APPROVED       // accreditation granted
  REJECTED       // did not meet standards
  EXPIRED        // 2-year certification lapsed
  RENEWAL        // submitted for renewal
}

// BlueSky accredited members (public directory)
model BlueSkyMember {
  id          String   @id @default(cuid())
  companyName String
  country     String?
  logo        String?
  website     String?
  accreditedAt DateTime
  expiresAt   DateTime
  isActive    Boolean  @default(true)
  sortOrder   Int      @default(0)
  createdAt   DateTime @default(now())
  updatedAt   DateTime @updatedAt
}

// Member logos (homepage showcase)
model MemberLogo {
  id        String   @id @default(cuid())
  name      String
  logo      String
  website   String?
  tier      String?  // Trustee | Corporate | etc.
  sortOrder Int      @default(0)
  isActive  Boolean  @default(true)
  createdAt DateTime @default(now())
}

// Media library
model MediaFile {
  id         String   @id @default(cuid())
  filename   String
  url        String   // Vercel Blob URL
  mimeType   String
  size       Int
  alt        String?
  uploadedBy String?
  createdAt  DateTime @default(now())
}

// Tags (for articles)
model Tag {
  id       String    @id @default(cuid())
  name     String    @unique
  slug     String    @unique
  articles Article[]
}

// Site settings (key-value store)
model SiteSetting {
  key   String @id
  value String
}

// Form submissions (contact, media inquiry)
model FormSubmission {
  id          String   @id @default(cuid())
  formType    String   // contact | media | membership_inquiry
  data        Json
  submittedAt DateTime @default(now())
  isRead      Boolean  @default(false)
}
```

---

## 5. Admin Panel — `/admin`

The admin panel is the most important non-public deliverable. It must be so good that Rachel never wants to go back to WordPress. "Better and more refined" than WillRise and Veritwin — this is the benchmark.

### Authentication

- NextAuth.js v5 with Credentials provider
- JWT session tokens, httpOnly cookies
- Route middleware protects all `/admin/*` routes
- Login at `/admin/login` — TIACA-branded, not PCG
- Password reset via email (Resend)
- Session expiry: 8 hours with remember-me option (30 days)
- Rate limiting on login endpoint (5 attempts → 15 min lockout)
- HTTPS only; HSTS header enforced

### Admin Sections

**Dashboard** (`/admin`)
- Today's form submissions (contact, media inquiry, BlueSky applications)
- Recent published articles
- Quick publish shortcuts
- Site traffic snapshot (Vercel Analytics embed)
- Upcoming events widget

**Pages** (`/admin/pages`)
- List of all core pages with publish status
- Block-based editor per page (no raw HTML required)
- Block types: Hero, RichText, ImageText, CTABanner, CardGrid, StatBar, LogoGrid, AccordionFAQ, VideoEmbed, TeamGrid, Form
- SEO fields per page: meta title, meta description, OG image, canonical URL
- Preview mode: opens a draft preview in a new tab before publishing
- Publish/unpublish toggle with confirmation
- Last edited timestamp + who edited

**Newsroom** (`/admin/newsroom`)
- Tabbed by category: Press Releases | Cargo Pulse | TIACA In the News | Mission Sustainability | Member News | TNN
- List view with search, filter by date, filter by published status
- Rich text editor per article (Tiptap — extensible, clean, not Quill/TinyMCE)
- Cover image upload (Vercel Blob), external URL option
- Scheduled publishing (publish at a future date/time)
- SEO fields per article
- Slug is auto-generated from title but editable (critical for URL preservation)
- Bulk operations: publish, unpublish, delete

**Hall of Fame** (`/admin/hall-of-fame`)
- Grid of all 50+ inductees
- Add/edit profile: name, title, company, year inducted, bio (rich text), photo
- Drag-to-reorder sort
- Publish/archive individual profiles

**Board & Team** (`/admin/people`)
- Tabbed: Board of Directors | Team | Chairman's Council
- Same add/edit/sort interface as HoF
- Board member roles: Chair, Vice Chair, Treasurer, Secretary, Member
- "Current" vs "Former" toggle

**Training Catalog** (`/admin/training`)
- List of all 46 courses
- Add/edit: title, provider, level, category, external URL, description, cover image
- Category management (create/rename/delete categories)
- Sort order drag handle
- Publish/unpublish

**BlueSky Accreditation** (`/admin/bluesky`)
- Sub-sections: Applications | Accredited Members | Criteria
- **Applications**: incoming table with status badges (Pending/In Review/Approved/Rejected/Expired). Click row → review modal → update status, add review notes, set accreditation/expiry dates, trigger approval email
- **Accredited Members**: directory management — add logo, website, active/inactive toggle
- **Criteria**: edit the 8 sustainability objectives displayed on the public page (rich text)

**Events** (`/admin/events`)
- Manage event cards shown on the `/events` hub page
- Fields: name, dates, location, description, external link, cover image, featured toggle
- Not a full ticketing system — just content management for the hub page

**Member Logos** (`/admin/members`)
- Upload member logos for homepage showcase
- Fields: name, logo, website, tier, sort order, active toggle

**Media Library** (`/admin/media`)
- Full media library (all uploaded images/files via Vercel Blob)
- Search by filename or alt text
- Upload via drag-and-drop or file picker
- Copy URL, update alt text, delete
- Used across all content types

**Form Submissions** (`/admin/submissions`)
- Inbox for contact, media inquiry, and membership inquiry forms
- Mark as read, export to CSV
- Never auto-deletes

**Settings** (`/admin/settings`)
- Site-wide SEO defaults (default meta title suffix, default OG image)
- Navigation structure (primary nav links + dropdowns)
- Footer links
- Social media URLs
- Google Analytics ID
- Media partner logos (footer)
- Supporting organization logos (footer)
- Admin user management (SUPER_ADMIN only): invite new user, change role, reset password, deactivate

### Admin Design System

- TIACA primary blue `#005A9B` as accent throughout
- Clean white/light grey background — not dark theme (Rachel is a content editor, not a developer)
- shadcn/ui component library for consistency and speed
- Responsive — works on iPad for on-site event editing
- Sidebar navigation with section icons
- Breadcrumb navigation in all sub-pages
- Toast notifications for all save/publish actions
- Confirmation dialogs for destructive actions (delete, unpublish)
- Auto-save drafts every 30 seconds in the page editor

---

## 6. Public Site — Design Direction

### Visual System

| Element | Spec |
|---|---|
| Primary blue | `#005A9B` |
| Deep navy | `#002D56` (hero backgrounds, premium sections) |
| White | `#FFFFFF` (content areas) |
| Off-white | `#F8F9FB` (section alternation) |
| Accent gold | `#C9A84C` (awards sections only) |
| Typography — heading | Confirm from Brandbook; interim: **Inter 700/800** |
| Typography — body | Confirm from Brandbook; interim: **Inter 400/500** |
| Motion | Framer Motion: subtle scroll-triggered fade-ins, hero parallax, number count-up for stats |
| Photography | Executive, aviation, global logistics imagery — not stock clip art |
| Tone | Authoritative, global, future-forward. Never generic. Never corporate boilerplate. |

**Visual benchmark**: Davos/WEF meets premium B2B association — white space, confident typography, restrained use of color, impactful use of full-bleed imagery.

### Homepage Architecture

1. **Hero** — Full-width, deep navy background. Large heading with the TIACA why. Rotating featured initiative cards below (ACF, Summit, BlueSky, Awards). Two CTAs: "Register for ACF 2026" + "Explore Membership."
2. **The TIACA Why** — 2–3 sentence mission statement. Stats: years of operation, members, countries represented, event attendees (ACF 3,500 → 5,000 target). These numbers count up on scroll.
3. **What We Do** — Icon/card grid: Membership Community · Air Cargo Forum · Executive Summit · BlueSky Accreditation · Awards · Training Library
4. **Air Cargo Forum 2026** — Promotional strip. Miami Beach, Oct 26–29 2026. Sponsor/exhibit/attend CTAs.
5. **News Feed** — Latest 3 Press Releases + 3 Cargo Pulse articles. Tab or side-by-side.
6. **BlueSky Accreditation** — Feature block. Repositioned as the industry's leading sustainability accreditation. Apply CTA.
7. **Our Members** — Scrolling logo marquee of member logos grouped by tier.
8. **Media Supporters + Supporting Orgs** — Footer-adjacent logo grid.
9. **Footer** — Full site nav, TIACA HQ address, social links, legal links.

### Copy Standards (apply across all pages)

- No em dashes. Full stop.
- No passive voice.
- No "join us for a great experience" language — replace with specifics.
- Lead every section with a strong declarative statement of value.
- All CTAs are action verbs: "Register," "Apply," "Explore," "Nominate" — not "Learn More" unless truly appropriate.
- Institutional voice, not Glyn's personal voice.
- The TIACA why comes before the what.

---

## 7. BlueSky Accreditation — Full Feature Spec

This is a marquee feature of the redesign and a strategic product evolution.

### Public-Facing Pages

**`/bluesky`** — Main accreditation page:
- Hero: "The Air Cargo Industry's Sustainability Standard" — TIACA BlueSky Accreditation
- What it is: formal two-year accreditation for companies that demonstrate compliance with 8 sustainability objectives
- The 8 Criteria: visual grid with icon per objective (Decarbonization, Eliminate Waste, Protect Biodiversity, Support Economies, Improve Lives, Improve Efficiencies, Attract Employees, Partnerships)
- The Process: Apply → Third-Party Assessment → Accreditation Granted → Annual Review → Renewal
- Benefits of accreditation: badge display rights, TIACA directory listing, industry recognition, partner access
- **Accredited Members Directory**: logo grid of all accredited companies
- Partners: Change Horizon, ESG Consulting
- **Primary CTA**: "Apply for Accreditation" → opens application form

**Application form** (`/bluesky/apply`):
- Company name, contact name, email, phone
- Country, company size
- Website
- Industry segment (airline, airport, forwarder, handler, tech provider, other)
- "Tell us about your sustainability journey" (textarea)
- Submit → confirmation email sent via Resend → admin notified
- Submission stored in DB as BlueSkyApplication with status PENDING

### Admin Panel Side

- Rachel reviews incoming applications in `/admin/bluesky/applications`
- Status workflow: PENDING → IN_REVIEW → APPROVED or REJECTED
- On APPROVED: accredited member entry created, expiry set at +2 years, congratulations email triggered
- On REJECTED: rejection email with feedback
- EXPIRED: automated flag when `expiresAt` passes, email sent to company about renewal

---

## 8. Content Migration — WordPress Export Strategy

### What We Migrate

| Content | Count | Approach |
|---|---|---|
| News articles (all categories) | 900+ | Full WP XML export → migration script → Article table |
| Pages (core) | ~10 | Rebuild manually — copy from WP, rewrite and elevate |
| Hall of Fame profiles | 50+ | WP XML export → HallOfFameProfile table |
| Board of Directors | ~15 | WP XML export → BoardMember table |
| Team profiles | ~8 | Rebuild manually in admin |
| Training products | 46 | WP Product export → Course table |
| Media files | ~500+ | wget/scrape WP media library → re-upload to Vercel Blob |
| Taxonomies/categories | 8 | Mapped to ArticleCategory enum |

### Migration Script

A standalone Node.js script (`scripts/migrate-wp.ts`) that:
1. Parses WP XML export file (`wordpress-export.xml`)
2. Extracts posts, pages, custom post types, attachments
3. Maps WP post types → Prisma models
4. Maps WP categories → `ArticleCategory` enum
5. Downloads all referenced media from `tiaca.org/wp-content/uploads/` and re-uploads to Vercel Blob
6. Inserts all records into Vercel Postgres via Prisma
7. Generates a redirect map file (`redirects.json`) for any slug changes

### Article slug strategy

WP posts are typically at `tiaca.org/[post-slug]` (flat URL). New site: `tiaca.org/newsroom/[slug]`. All old flat URLs redirect 301 to the new newsroom path. The migration script extracts original slugs and writes them to a `redirects.json` which is injected into `next.config.js` redirects array.

### Pre-migration requirements

- [ ] Rachel provides WP admin export (Tools → Export → All content) — XML file
- [ ] Google Analytics access to identify high-traffic pages (protect those slug mappings)
- [ ] WP media library access (or the XML export will include attachment URLs we can wget)

---

## 9. SEO & URL Strategy

### URL Preservation Rules

| Content Type | Old URL | New URL | Action |
|---|---|---|---|
| News articles | `/[slug]` (flat) | `/newsroom/[slug]` | 301 redirect |
| Hall of Fame | `/hall-of-fame/[slug]` | `/awards/hall-of-fame/[slug]` | 301 redirect |
| Board profiles | `/board-of-directors/[slug]` | `/about/board/[slug]` | 301 redirect |
| Training products | `/product/[slug]` | `/training/[slug]` | 301 redirect |
| Core pages | Varies | New structure | Review GA first |
| Category pages | `/category/[slug]` | `/newsroom?category=[slug]` | 301 redirect |
| Removed pages | Various | `/` or nearest relevant page | 301 redirect |

All redirects are generated at build time from `redirects.json` via `next.config.js` — zero server-side redirect overhead.

### Technical SEO

- `next-sitemap` auto-generates XML sitemap for all published content
- Dynamic OG images via `@vercel/og` for articles and profiles
- Structured data: `Organization` schema on About, `Event` schema on event pages, `Article` schema on newsroom posts, `BreadcrumbList` on all pages
- Canonical URLs on all pages (handle www vs non-www)
- `robots.txt` blocks `/admin/*` entirely
- Google Search Console: submit new sitemap on launch

### Pre-launch GA review

Before restructuring any page URL, pull GA4 top-50 pages report. Any page with significant organic traffic gets its exact URL preserved OR gets a 301 redirect from old to new. This review happens in Week 1 alongside asset collection.

---

## 10. Training Library — Placeholder Strategy

**This phase**: Native catalog page at `/training`. No checkout, no WooCommerce.

- 46 courses imported into `Course` table
- Filterable by provider, level, category
- Each course card: title, provider badge, level badge, description excerpt, "Learn More" button → external URL
- No payment processing in this phase
- "Interested in group training? Contact us" CTA at bottom

**Future scope** (separate proposal): Native e-commerce — course purchase, access management, completion certificates. Rachel to confirm long-term vision with training partners (ICAO, SASI) before we spec this.

---

## 11. Phase Breakdown

### Pre-Build Gate (Week 0)

- [ ] Rachel provides WP XML export (all content)
- [ ] GA4 access — top-50 pages report for URL protection list
- [ ] Brandbook typography confirmed (or Inter locked in as permanent if no response)
- [ ] Rachel and Riley email addresses confirmed for admin accounts
- [ ] TIACA confirms they want BlueSky repositioned as Accreditation (Glyn sign-off)
- [ ] Kenneth's booking link confirmed
- [ ] Member logo pack requested from Rachel (for homepage showcase)
- [ ] Vercel project created, Postgres database provisioned

### Phase 1 — Foundation (Days 1–3)

- Next.js 15 project scaffolded with TypeScript, Tailwind, shadcn/ui
- Prisma schema finalized and initial migration applied to Vercel Postgres
- NextAuth.js configured — admin login at `/admin/login`
- Vercel Blob storage configured
- Base layout components: Header, Footer, Navigation
- Design tokens established (colors, typography, spacing)
- WP migration script drafted (runs against export XML when available)
- `tiaca.vercel.app` live (blank shell)

### Phase 2 — Admin Panel Build (Days 4–10)

All admin sections built before the public site — Rachel needs to be able to manage content before launch.

- Admin dashboard
- Pages list + block editor (core block types)
- Newsroom — article CRUD, categories, scheduled publish
- Hall of Fame management
- Board/Team management
- Training catalog management
- BlueSky Accreditation: application inbox + accredited members directory + criteria editor
- Media library
- Form submissions inbox
- Settings: nav, footer, social, analytics, media partners
- Admin user management
- Admin design system locked (TIACA blue, clean white UI, responsive)

**Internal review**: Aaren + Alicia sign off on admin panel before any client preview.

### Phase 3 — Public Site Build (Days 11–20)

Built page by page, using content already loaded into the CMS via the admin panel.

- Day 11–12: Homepage (all 9 sections)
- Day 13: About TIACA (consolidated Who We Are + What We Do)
- Day 14: Membership page
- Day 15: Events hub
- Day 16: ACF 2026 + Executive Summit 2026 bridge pages
- Day 17: BlueSky Accreditation (full feature — criteria, directory, application form)
- Day 18: Newsroom (hub + article list + individual article template)
- Day 19: Training Library + Awards + Hall of Fame (directory + profile template)
- Day 20: Contact + VSL landing page + Legal/Privacy/Terms

### Phase 4 — Content Migration (Days 14–21, parallel to Phase 3)

- WP XML export received → migration script executed
- 900+ articles imported and verified
- HoF profiles (50+) imported and verified
- Board profiles imported
- Training courses (46) imported
- Media files downloaded from WP, re-uploaded to Vercel Blob, URLs updated in DB
- Redirect map generated and injected into `next.config.js`
- GA URL review complete — protected slugs confirmed

### Phase 5 — QA (Days 22–24)

- All CTAs and forms tested (verify Resend delivery to correct inboxes)
- Mobile QA: iPhone SE, iPhone 14, Samsung Galaxy, iPad Pro
- Cross-browser: Chrome, Firefox, Safari, Edge — desktop + mobile
- Admin panel QA: Rachel-persona walkthrough (create article, upload image, update page, manage BlueSky application)
- SEO audit: check meta tags, OG images, sitemap, structured data, robots.txt
- Core Web Vitals: LCP < 2.5s, CLS < 0.1, INP < 200ms
- Redirect map verification: spot-check 20 old URLs → confirm 301 to new locations
- Load test admin panel under concurrent sessions

### Phase 6 — Rachel Onboarding + Handover Prep (Day 23–25)

- 1-hour walkthrough session with Rachel (Zoom): how to publish articles, edit pages, manage BlueSky applications, upload media, update nav
- Short Loom video library: one recording per major admin section (< 5 min each)
- Handover doc: admin login, password reset process, Vercel dashboard access, database backup procedure
- TIACA Vercel account setup (transfer project from Sirak account)

### Phase 7 — Launch (Day 25–28)

- Staging review sent to Rachel + Glyn (3-business-day window)
- Feedback resolved (same day where possible)
- DNS cutover: Glyn/Rachel coordinate with current hosting provider
- Vercel custom domain: `tiaca.org` + `www.tiaca.org`
- Google Analytics 4 property verified on live domain
- Google Search Console: new sitemap submitted
- Post-launch: 48-hour monitoring for broken links, form delivery, redirect issues
- TIACA.org main site is now the destination for all incoming traffic from the Warsaw ES landing page, ACF event site, TNN, and LinkedIn campaigns

**Target**: Live by **May 16, 2026** (2 weeks before Warsaw Jun 1–3) to serve as the destination for event-driven traffic.

---

## 12. Dashboard Task Board

**Board: TIACA — Website Redesign**

| Task | Status | Owner | Blocked on |
|---|---|---|---|
| WP XML export from Rachel | todo | Sirak | Rachel meeting |
| GA4 access + top-50 URL report | todo | Sirak | Rachel meeting |
| Brandbook typography confirmed | todo | Sirak | Rachel meeting |
| Admin user credentials confirmed (Rachel, Riley) | todo | Sirak | Rachel meeting |
| BlueSky Accreditation repositioning — Glyn sign-off | todo | Aaren | Glyn call |
| Member logo pack from Rachel | todo | Sirak | Rachel meeting |
| Kenneth booking link confirmed | todo | Aaren | Kenneth |
| Vercel project + Postgres provisioned | todo | Dev | — |
| Next.js project scaffold | todo | Dev | Vercel setup |
| Prisma schema + initial migration | todo | Dev | Scaffold done |
| NextAuth admin auth | todo | Dev | Schema done |
| Admin — dashboard view | todo | Dev | Auth done |
| Admin — Pages editor (block-based) | todo | Dev | Auth done |
| Admin — Newsroom CRUD | todo | Dev | Auth done |
| Admin — Hall of Fame management | todo | Dev | Auth done |
| Admin — Board/Team management | todo | Dev | Auth done |
| Admin — Training catalog | todo | Dev | Auth done |
| Admin — BlueSky Accreditation | todo | Dev | Auth done |
| Admin — Media library (Vercel Blob) | todo | Dev | Auth done |
| Admin — Settings + Nav management | todo | Dev | Auth done |
| Admin — Form submissions inbox | todo | Dev | Auth done |
| Internal admin panel review | todo | Aaren + Alicia | Admin build done |
| WP migration script | todo | Dev | WP export received |
| 900+ articles imported | todo | Dev | Migration script |
| HoF + Board profiles imported | todo | Dev | Migration script |
| Training catalog imported (46 courses) | todo | Dev | Migration script |
| Media re-upload to Vercel Blob | todo | Dev | Migration script |
| Redirect map generated + wired | todo | Dev | Migration script |
| Homepage build (all 9 sections) | todo | Dev | Admin done |
| About TIACA page | todo | Dev | Homepage done |
| Membership page | todo | Dev | Homepage done |
| Events hub | todo | Dev | Homepage done |
| ACF 2026 + Summit bridge pages | todo | Dev | Homepage done |
| BlueSky Accreditation public page + apply form | todo | Dev | Homepage done |
| Newsroom hub + article templates | todo | Dev | Homepage done |
| Training Library page | todo | Dev | Homepage done |
| Awards + Hall of Fame pages | todo | Dev | Homepage done |
| Contact + VSL landing + Legal pages | todo | Dev | Homepage done |
| Mobile QA | todo | QA | All pages done |
| Cross-browser QA | todo | QA | All pages done |
| Admin panel QA (Rachel persona) | todo | QA | All pages done |
| SEO audit (sitemap, OG, structured data) | todo | Dev | All pages done |
| Core Web Vitals check | todo | Dev | All pages done |
| Rachel onboarding session | todo | Sirak | QA passed |
| Loom video library (admin walkthrough) | todo | Sirak | QA passed |
| Staging review — Rachel + Glyn | todo | Sirak | QA + onboarding |
| Feedback resolved | todo | Dev | Staging review |
| DNS cutover to tiaca.org | todo | Sirak + Rachel | Glyn final approval |
| GA4 + Search Console verified on live domain | todo | Dev | DNS live |
| Sitemap submitted to Google Search Console | todo | Dev | DNS live |
| **SITE LIVE** | todo | Sirak | All above |

---

## 13. Open Items

- [ ] **Brandbook typography** — Rachel to confirm official typefaces + web font license. Blocking design start.
- [ ] **BlueSky Accreditation sign-off** — Glyn must confirm the repositioning from "program" to "accreditation" before the `/bluesky` page is written.
- [ ] **Training library long-term vision** — Rachel to discuss with ICAO/SASI partners whether native e-commerce is wanted. Affects whether we scope a future e-commerce build.
- [ ] **WP export** — Rachel provides via file transfer. Cannot start migration without this.
- [ ] **GA4 access** — Rachel to add Sirak as viewer. Needed for URL protection list before redirect map is built.
- [ ] **Riley's admin role** — Confirm whether Riley gets EDITOR access (can post newsroom content) or no admin access initially.
- [ ] **Member login** — Current: link to `tiaca.glueup.com`. Scope out native member portal as separate Phase 3 proposal (future).
- [ ] **TNN section in Newsroom** — Hidden (no visible category) until pilot is green-lit; category exists in DB for when it's needed.
- [ ] **Awards pricing/nomination** — Hall of Fame Nomination Form exists. Confirm if the nomination flow is in scope or just a link to an external form.
- [ ] **Executive Summit exact dates** — Jun 1–3 vs May 26–28 discrepancy. Confirm before summit bridge page is built.
- [ ] **Domain/DNS provider** — Who holds the tiaca.org DNS? Rachel or an external registrar? Need this for cutover.

---

## 14. What "Over-Deliver" Means on This Project

The $9,800 contract scopes 10 pages + CMS + design. We are delivering:

- Full custom Next.js platform (not WordPress, not a template)
- Custom admin panel that rivals commercial CMS products
- Complete content migration of 900+ articles, 50+ HoF profiles, 46 training courses
- BlueSky Accreditation as a full product feature (application flow, directory, admin workflow)
- SEO infrastructure (redirects, sitemap, structured data, OG images, Core Web Vitals)
- Rachel onboarding + Loom video library
- Vercel deployment + domain setup assistance
- A site that, when Glyn shows it at Warsaw, makes every industry contact ask "who built this?"

That last sentence is the quality bar.
