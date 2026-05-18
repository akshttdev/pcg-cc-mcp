-- Delete social_posts rows whose id column is stored as BLOB (binary UUID) or
-- as TEXT (36-char string in a BLOB-declared column). The Rust model decodes
-- social_posts.id as uuid::Uuid which expects a 16-byte binary BLOB. Any row
-- whose id is stored as TEXT (typeof != 'blob' or length != 16) can't be
-- decoded and causes SELECT * to fail with "invalid length: expected 16 bytes".
-- These rows are orphaned content from old publisher runs; deleting them is
-- safe — the list endpoint becomes usable and new inserts are clean BLOB UUIDs.
DELETE FROM social_posts WHERE typeof(id) != 'blob';
DELETE FROM social_posts WHERE typeof(id) = 'blob' AND length(id) != 16;
