-- Full-text search index for tasks using SQLite FTS5
-- Enables fast keyword search across title, description, and tags

CREATE VIRTUAL TABLE IF NOT EXISTS tasks_fts USING fts5(
    task_id UNINDEXED,
    title,
    description,
    tags,
    content='tasks',
    content_rowid='rowid'
);

-- Populate FTS index from existing tasks
INSERT INTO tasks_fts(task_id, title, description, tags)
    SELECT HEX(id), title, COALESCE(description, ''), COALESCE(tags, '')
    FROM tasks;

-- Triggers to keep FTS index in sync with tasks table

CREATE TRIGGER IF NOT EXISTS tasks_fts_insert AFTER INSERT ON tasks BEGIN
    INSERT INTO tasks_fts(task_id, title, description, tags)
    VALUES (HEX(NEW.id), NEW.title, COALESCE(NEW.description, ''), COALESCE(NEW.tags, ''));
END;

CREATE TRIGGER IF NOT EXISTS tasks_fts_update AFTER UPDATE ON tasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = HEX(OLD.id);
    INSERT INTO tasks_fts(task_id, title, description, tags)
    VALUES (HEX(NEW.id), NEW.title, COALESCE(NEW.description, ''), COALESCE(NEW.tags, ''));
END;

CREATE TRIGGER IF NOT EXISTS tasks_fts_delete AFTER DELETE ON tasks BEGIN
    DELETE FROM tasks_fts WHERE task_id = HEX(OLD.id);
END;
