-- Seed initial CMS content for Omega Wireless site

-- Create the site
INSERT INTO cms_sites (id, slug, name, domain, theme_config, is_active)
VALUES (
    'omega-wireless-001',
    'omega-wireless',
    'Omega Wireless',
    'omega-presale.vercel.app',
    '{"primaryColor": "#f5c542", "backgroundColor": "#0a0a0a", "fontFamily": "Inter"}',
    1
);

-- Create the Founders' Edition product
INSERT INTO cms_products (id, site_id, slug, name, short_description, long_description, price_cents, image_url, specs, features, is_featured, is_active, stripe_price_id, sort_order)
VALUES (
    'product-founders-001',
    'omega-wireless-001',
    'founders-edition',
    'Founders'' Edition',
    'Limited batch APN-compatible hardware with engraved serial and lifetime firmware updates.',
    'The Founders'' Edition is a limited batch of proprietary hardware designed for the open-source Alpha Protocol Network. Each device includes an engraved serial number and lifetime firmware updates.',
    100000,
    '/device.png',
    '{"radio": "LoRa SX1262 module", "connectivity": "Dual-band WiFi + Bluetooth", "compute": "Secure edge board", "power": "DC input; solar/battery compatible", "storage": "Removable microSD", "enclosure": "Engraved Founders'' Edition, serialised", "firmware": "APN reference stack pre-installed", "ports": "Ethernet, USB-C (service), GPIO header", "dimensions": "TBD (prototype phase)", "updates": "OTA with signed releases"}',
    '["LoRa SX1262", "Dual-band WiFi + BT", "Edge-secure compute", "Solar/battery ready", "APN firmware", "Engraved serial"]',
    1,
    1,
    NULL,
    0
);

-- Create FAQ items
INSERT INTO cms_faq_items (id, site_id, question, answer, category, sort_order, is_active) VALUES
('faq-001', 'omega-wireless-001', 'Is this a token sale?', 'No. This is a hardware pre-order. The Alpha Protocol Network (APN) is open-source; tokens are not sold or allocated here. Devices may earn based on verifiable contribution.', 'General', 0, 1),
('faq-002', 'omega-wireless-001', 'Open-source protocol vs. hardware?', 'The APN protocol and reference software are open-source. Omega Wireless sells proprietary, APN-compatible hardware. Anyone will be able to contribute to APN using their own hardware when the APN Dashboard is released.', 'General', 1, 1),
('faq-003', 'omega-wireless-001', 'APN Dashboard status?', 'The APN Dashboard is currently in Beta. Users will receive testnet tokens until otherwise notified.', 'General', 2, 1),
('faq-004', 'omega-wireless-001', 'When will devices ship?', 'We will announce the manufacturing timeline after the Shenzhen sprint. Estimated window will be shown at checkout or in your order email.', 'Shipping', 3, 1);

-- Create page sections for home page
INSERT INTO cms_page_sections (id, site_id, page_slug, section_key, content, sort_order, is_active) VALUES
-- Hero section
('section-hero-001', 'omega-wireless-001', 'home', 'hero', '{
    "headline": "Host Your Own Secure Network. Earn Rewards.",
    "subheadline": "Join the Alpha Protocol Network (APN), powered by the Bitcoin blockchain. Contribute mesh coverage and services to a decentralized network and get rewarded for strengthening the system.",
    "cta_text": "Ships worldwide - Lifetime firmware updates",
    "image_url": "/device.png",
    "features": ["LoRa SX1262", "Dual-band WiFi + BT", "Edge-secure compute", "Solar/battery ready", "APN firmware", "Engraved serial"]
}', 0, 1),

-- Value props section
('section-valueprops-001', 'omega-wireless-001', 'home', 'value_props', '{
    "items": [
        {"title": "Purpose-Built for APN", "description": "Omega Wireless builds proprietary hardware designed for the open-source Alpha Protocol Network."},
        {"title": "Founders'' Edition", "description": "Limited batch. Engraved serial + lifetime firmware updates."},
        {"title": "Earn + Purchase", "description": "Purchase bandwidth on APN and earn by contributing verifiable mesh coverage and services."}
    ]
}', 1, 1),

-- Features section
('section-features-001', 'omega-wireless-001', 'home', 'features', '{
    "heading": "How It Works",
    "items": [
        {"title": "Connect", "description": "Power on and the device forms/joins a local APN mesh."},
        {"title": "Contribute", "description": "Provide bandwidth and relay traffic; optionally run edge services."},
        {"title": "Earn & Purchase", "description": "Devices can earn based on contribution. Users can also purchase bandwidth on-demand."}
    ]
}', 2, 1),

-- Specs section
('section-specs-001', 'omega-wireless-001', 'home', 'specs', '{
    "heading": "Technical Specs (Founders'' Edition)",
    "columns": [
        ["Radio: LoRa SX1262 module", "Connectivity: Dual-band WiFi + Bluetooth", "Compute: Secure edge board", "Power: DC input; solar/battery compatible", "Storage: Removable microSD"],
        ["Enclosure: Engraved Founders'' Edition, serialised", "Firmware: APN reference stack pre-installed", "Ports: Ethernet, USB-C (service), GPIO header", "Dimensions: TBD (prototype phase)", "Updates: OTA with signed releases"]
    ]
}', 3, 1),

-- Payment options section
('section-payment-001', 'omega-wireless-001', 'home', 'payment_options', '{
    "heading": "Payment Options"
}', 4, 1);

-- Create site settings
INSERT INTO cms_site_settings (id, site_id, setting_key, setting_value) VALUES
('setting-btc-001', 'omega-wireless-001', 'btc_address', '12cbTwnap9MT5GS6ah7J9cm32Zaz7hjk2B'),
('setting-eth-001', 'omega-wireless-001', 'eth_address', '0x49cE915A25Af93254E125Ce84a69E790aD13Dc1e'),
('setting-email-001', 'omega-wireless-001', 'contact_email', 'innovate@powerclubglobal.com'),
('setting-twitter-001', 'omega-wireless-001', 'social_twitter', 'https://twitter.com/omegawireless'),
('setting-discord-001', 'omega-wireless-001', 'social_discord', 'https://discord.gg/omegawireless');
