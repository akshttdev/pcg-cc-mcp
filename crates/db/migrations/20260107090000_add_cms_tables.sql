-- CMS Tables for Omega Wireless and Multi-tenant Site Management
PRAGMA foreign_keys = ON;

-- Site configuration (multi-tenant support)
CREATE TABLE cms_sites (
    id              BLOB PRIMARY KEY,
    slug            TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    domain          TEXT NOT NULL UNIQUE,
    theme_config    TEXT DEFAULT '{}',
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Products table
CREATE TABLE cms_products (
    id                  BLOB PRIMARY KEY,
    site_id             BLOB NOT NULL,
    slug                TEXT NOT NULL,
    name                TEXT NOT NULL,
    short_description   TEXT,
    long_description    TEXT,
    price_cents         INTEGER NOT NULL,
    currency            TEXT NOT NULL DEFAULT 'USD',
    stripe_price_id     TEXT,
    image_url           TEXT,
    gallery_images      TEXT DEFAULT '[]',
    specs               TEXT DEFAULT '{}',
    features            TEXT DEFAULT '[]',
    is_active           INTEGER NOT NULL DEFAULT 1,
    is_featured         INTEGER NOT NULL DEFAULT 0,
    stock_status        TEXT DEFAULT 'in_stock',
    sort_order          INTEGER DEFAULT 0,
    created_at          TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (site_id) REFERENCES cms_sites(id) ON DELETE CASCADE,
    UNIQUE(site_id, slug)
);

-- FAQ items
CREATE TABLE cms_faq_items (
    id              BLOB PRIMARY KEY,
    site_id         BLOB NOT NULL,
    category        TEXT,
    question        TEXT NOT NULL,
    answer          TEXT NOT NULL,
    sort_order      INTEGER DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (site_id) REFERENCES cms_sites(id) ON DELETE CASCADE
);

-- Page sections (flexible content blocks)
CREATE TABLE cms_page_sections (
    id              BLOB PRIMARY KEY,
    site_id         BLOB NOT NULL,
    page_slug       TEXT NOT NULL,
    section_key     TEXT NOT NULL,
    content         TEXT NOT NULL DEFAULT '{}',
    sort_order      INTEGER DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (site_id) REFERENCES cms_sites(id) ON DELETE CASCADE,
    UNIQUE(site_id, page_slug, section_key)
);

-- Site settings (global configuration)
CREATE TABLE cms_site_settings (
    id              BLOB PRIMARY KEY,
    site_id         BLOB NOT NULL,
    setting_key     TEXT NOT NULL,
    setting_value   TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (site_id) REFERENCES cms_sites(id) ON DELETE CASCADE,
    UNIQUE(site_id, setting_key)
);

-- CMS user roles (separate from project roles)
CREATE TABLE cms_user_roles (
    id              BLOB PRIMARY KEY,
    user_id         BLOB NOT NULL,
    site_id         BLOB NOT NULL,
    role            TEXT NOT NULL DEFAULT 'viewer'
                       CHECK (role IN ('admin','editor','viewer')),
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (site_id) REFERENCES cms_sites(id) ON DELETE CASCADE,
    UNIQUE(user_id, site_id)
);

-- Create indexes for performance
CREATE INDEX idx_cms_products_site ON cms_products(site_id);
CREATE INDEX idx_cms_products_slug ON cms_products(slug);
CREATE INDEX idx_cms_products_active ON cms_products(is_active, is_featured);
CREATE INDEX idx_cms_faq_items_site ON cms_faq_items(site_id);
CREATE INDEX idx_cms_faq_items_category ON cms_faq_items(site_id, category);
CREATE INDEX idx_cms_page_sections_site ON cms_page_sections(site_id, page_slug);
CREATE INDEX idx_cms_site_settings_site ON cms_site_settings(site_id);
CREATE INDEX idx_cms_user_roles_user ON cms_user_roles(user_id);
CREATE INDEX idx_cms_user_roles_site ON cms_user_roles(site_id);
