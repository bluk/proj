CREATE TABLE input_files (
  id TEXT NOT NULL PRIMARY KEY CHECK(length(id) < 512),

  logical_path TEXT NOT NULL,
  content_hash BLOB NOT NULL,

  content BLOB,

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_input_files_updated_at
AFTER UPDATE OF id, logical_path, content_hash, content, created_at ON input_files
FOR EACH ROW
WHEN OLD.updated_at = NEW.updated_at -- to prevent recursive updates
BEGIN
    UPDATE input_files
    SET updated_at = CURRENT_TIMESTAMP
    WHERE id = NEW.id;
END;