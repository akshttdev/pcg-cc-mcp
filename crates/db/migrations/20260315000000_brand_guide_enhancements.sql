-- Brand guide pipeline enhancements
ALTER TABLE organization_brand_profiles ADD COLUMN mood_board_urls TEXT; -- JSON array of DALL-E generated image URLs
ALTER TABLE organization_brand_profiles ADD COLUMN clearbit_logo_url TEXT; -- Auto-fetched from Clearbit CDN
ALTER TABLE organization_brand_profiles ADD COLUMN brand_photography_notes TEXT; -- AI photography style direction
