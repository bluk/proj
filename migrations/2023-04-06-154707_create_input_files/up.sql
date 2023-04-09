CREATE TABLE input_files (
  id TEXT NOT NULL PRIMARY KEY CHECK(length(id) < 512),

  logical_path TEXT NOT NULL,
  contents_hash BLOB NOT NULL,

  contents BLOB,

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_input_files_logical_path ON input_files(logical_path);
CREATE INDEX idx_input_files_contents_hash ON input_files(contents_hash);