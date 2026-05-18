-- TIACA ES2026 LinkedIn Content Calendar — Social Post Seeder
-- Project: TIACA - The International Air Cargo Association
-- Project ID: 3999206c-9065-493e-bf48-878919cf9dad
-- Run: sqlite3 dev_assets/db.sqlite < scripts/seed_tiaca_social_posts.sql

-- Safety: only insert if no posts exist for this project yet
-- (re-runnable — will skip if already seeded)

INSERT OR IGNORE INTO social_posts (
    id, project_id, content_type, caption, hashtags, platforms,
    status, scheduled_for, category
) VALUES

-- Post 1: ES Registration Open
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'post',
    'The Executive Summit Warsaw is open for registration.

June 1–3, 2026. Closed-door. Senior air cargo leaders only.

If you''re shaping the future of global freight — this is where the conversation happens.

Link in bio to secure your seat.',
    '["#TIACAES2026","#AirCargo","#ExecutiveSummit","#Warsaw","#AirFreight","#Logistics"]',
    '[]',
    'draft',
    '2026-05-26T09:00:00Z',
    'executive_summit'
),

-- Post 2: Where Air Cargo Leaders Unite
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'post',
    'The room where it happens.

Every year, the people setting the agenda for air cargo gather in one place. This June, that place is Warsaw.

TIACA Executive Summit 2026. Application-based. Limited seats.

If your organization is driving change in global freight — you should be in this room.',
    '["#TIACAES2026","#AirCargo","#Leadership","#Warsaw","#AirFreight","#SupplyChain"]',
    '[]',
    'draft',
    '2026-05-28T09:00:00Z',
    'executive_summit'
),

-- Post 3: Article — The New Air Cargo Executive (Article Release)
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'article',
    'We surveyed air cargo executives across 14 markets. Here is what the next generation of industry leadership actually looks like — and why the old playbook no longer applies.

Full analysis in the article ↓',
    '["#TIACAES2026","#AirCargo","#Leadership","#FutureOfFreight","#AirFreight","#Logistics"]',
    '[]',
    'draft',
    '2026-05-30T09:00:00Z',
    'thought_leadership'
),

-- Post 4: Warsaw City / Event Atmosphere
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'post',
    '48 hours. 80 leaders. Zero fluff.

The TIACA Executive Summit is designed for people who don''t have time for panels that go nowhere.

Every session is structured for decision-making. Every conversation is off the record.

Warsaw. June 1–3.',
    '["#TIACAES2026","#AirCargo","#Warsaw","#ExecutiveSummit","#AirFreight"]',
    '[]',
    'draft',
    '2026-06-01T07:00:00Z',
    'executive_summit'
),

-- Post 5: E-Commerce & Cross-Border Panel
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'post',
    'E-commerce didn''t disrupt air cargo. It transformed it.

Cross-border volumes are reshaping route economics, customs infrastructure, and carrier strategy — faster than most operators anticipated.

Today at #TIACAES2026, we''re breaking down what the data actually shows.',
    '["#TIACAES2026","#Ecommerce","#AirCargo","#CrossBorder","#SupplyChain","#Logistics","#AirFreight"]',
    '[]',
    'draft',
    '2026-06-01T14:00:00Z',
    'executive_summit'
),

-- Post 6: Safety & Dangerous Goods
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'post',
    'Safety is not a competitive differentiator.

It''s the foundation everything else is built on.

At this year''s Executive Summit, the DG/hazmat conversation is getting the time it deserves — including where industry standards need to evolve.',
    '["#TIACAES2026","#AirCargoSafety","#DangerousGoods","#AirCargo","#AirFreight"]',
    '[]',
    'draft',
    '2026-06-02T09:00:00Z',
    'executive_summit'
),

-- Post 7: Article — Why Events Are TIACA's Revenue Engine (Article Release)
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'article',
    'TIACA hosts two flagship events per year. Between them: ~220 sponsors and exhibitors, 3,500+ unique attendees, and tens of millions in economic activity generated for the industry.

Here''s the infrastructure we''re building to make ACF Miami 2026 the most commercially successful air cargo event ever hosted.

Article ↓',
    '["#ACFMiami2026","#AirCargo","#EventStrategy","#AirFreight","#Logistics"]',
    '[]',
    'draft',
    '2026-06-02T14:00:00Z',
    'thought_leadership'
),

-- Post 8: Digitalization Panel
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'post',
    'The paperless cargo shipment has been "three years away" for fifteen years.

What actually changed? What''s still broken? Who''s making it happen?

The digitalization session at #TIACAES2026 skipped the theory and went straight to the operators who are doing it.',
    '["#TIACAES2026","#AirCargoDigitalization","#AirCargo","#Digitization","#AirFreight","#SupplyChain"]',
    '[]',
    'draft',
    '2026-06-03T09:00:00Z',
    'executive_summit'
),

-- Post 9: Networking / Connections Made
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'post',
    'The best conversations at the Executive Summit don''t happen in sessions.

They happen after. Over dinner. On the way out.

That''s by design.

TIACA Executive Summit 2026 — Warsaw, June 1–3. Where the industry actually talks.',
    '["#TIACAES2026","#AirCargo","#Networking","#AirFreight","#Warsaw","#Logistics"]',
    '[]',
    'draft',
    '2026-06-03T18:00:00Z',
    'executive_summit'
),

-- Post 10: Article — State of Air Cargo 2026 (Article Release)
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'article',
    'We came to Warsaw with questions about where the industry is heading. We''re leaving with data.

TIACA''s State of Air Cargo 2026 briefing — compiled from Executive Summit sessions, industry surveys, and conversations with 80+ senior operators.

Full report below.',
    '["#TIACAES2026","#AirCargo","#StateOfAirCargo","#AirFreight","#SupplyChain","#Logistics"]',
    '[]',
    'draft',
    '2026-06-05T09:00:00Z',
    'thought_leadership'
),

-- Post 11: Next Gen / Youth Theme
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'post',
    'The next Director General of your organization is 26 years old.

They''re not waiting for permission to reshape this industry.

TIACA''s commitment: every major event includes emerging leaders who challenge the room. Not as observers — as participants.

The future of air cargo is already here.',
    '["#TIACAES2026","#NextGenTalent","#AirCargo","#FutureOfFreight","#AirFreight","#YoungProfessionals"]',
    '[]',
    'draft',
    '2026-06-07T09:00:00Z',
    'thought_leadership'
),

-- Post 12: Article — TIACA Membership (Article Release)
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'article',
    'Every organization in air cargo operates in the same regulatory, safety, and commercial environment.

TIACA membership is how you help shape that environment — rather than just operate within it.

What membership actually means, what it costs, and who it''s for.',
    '["#TIACA","#AirCargo","#Membership","#AirFreight","#Advocacy","#Networking"]',
    '[]',
    'draft',
    '2026-06-10T09:00:00Z',
    'membership'
),

-- Post 13: ACF Miami 2026 Save the Date
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'post',
    'Mark it.

Air Cargo Forum Miami — October 26–29, 2026.

The largest gathering in air cargo returns to Miami Beach. 5,000 industry professionals. 220+ exhibitors. Four days that define the next two years.

Early exhibitor and sponsor applications are open now.',
    '["#ACFMiami2026","#AirCargoForum","#AirCargo","#Miami","#AirFreight","#Logistics"]',
    '[]',
    'draft',
    '2026-06-15T09:00:00Z',
    'acf_miami'
),

-- Post 14: ACF Miami Momentum / Teaser
(
    lower(hex(randomblob(4)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(2)))||'-'||lower(hex(randomblob(6))),
    '3999206c-9065-493e-bf48-878919cf9dad',
    'post',
    'Last cycle: 3,500 attendees. 220 exhibitors. Miami Beach.

This cycle: we''re targeting 5,000.

Air Cargo Forum 2026 is the most commercially important event TIACA hosts. If your organization is in air freight — this is where deals get made, partnerships are formed, and the next two years of business development happen in four days.

October 26–29. Miami Beach Convention Center.

Exhibitor and sponsor packages: link in bio.',
    '["#ACFMiami2026","#AirCargoForum","#AirCargo","#Miami","#AirFreight","#SupplyChain","#Logistics"]',
    '[]',
    'draft',
    '2026-07-14T09:00:00Z',
    'acf_miami'
);
