CREATE TABLE revisions (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_revisions_updated_at
AFTER UPDATE OF id, created_at ON revisions
FOR EACH ROW
WHEN OLD.updated_at = NEW.updated_at -- to prevent recursive updates
BEGIN
    UPDATE revisions
    SET updated_at = CURRENT_TIMESTAMP
    WHERE id = NEW.id;
END;