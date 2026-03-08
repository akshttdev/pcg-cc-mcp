-- Google My Business-style fields for companies
ALTER TABLE companies ADD COLUMN phone           TEXT;
ALTER TABLE companies ADD COLUMN email           TEXT;
ALTER TABLE companies ADD COLUMN address         TEXT;
ALTER TABLE companies ADD COLUMN city            TEXT;
ALTER TABLE companies ADD COLUMN country         TEXT;
ALTER TABLE companies ADD COLUMN founded_year    INTEGER;
ALTER TABLE companies ADD COLUMN employee_count  TEXT;   -- '1-10','11-50','51-200','201-500','500+'
ALTER TABLE companies ADD COLUMN cover_image_url TEXT;
ALTER TABLE companies ADD COLUMN tags            TEXT DEFAULT '[]'; -- JSON array
ALTER TABLE companies ADD COLUMN gmb_rating      REAL;
ALTER TABLE companies ADD COLUMN gmb_review_count INTEGER;
ALTER TABLE companies ADD COLUMN gmb_place_id    TEXT;
ALTER TABLE companies ADD COLUMN instagram_handle TEXT;
ALTER TABLE companies ADD COLUMN linkedin_url     TEXT;
ALTER TABLE companies ADD COLUMN twitter_handle   TEXT;
ALTER TABLE companies ADD COLUMN facebook_url     TEXT;
ALTER TABLE companies ADD COLUMN whatsapp         TEXT;
ALTER TABLE companies ADD COLUMN business_hours   TEXT;  -- JSON {mon:'9-17', tue:'9-17', ...}
ALTER TABLE companies ADD COLUMN notes            TEXT;  -- internal notes
