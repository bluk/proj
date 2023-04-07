CREATE TABLE revision_files (
  revision_id INTEGER NOT NULL,
  input_file_id TEXT NOT NULL,

  PRIMARY KEY(revision_id, input_file_id),

  FOREIGN KEY(revision_id) REFERENCES revisions(id) ON UPDATE CASCADE ON DELETE CASCADE,
  FOREIGN KEY(input_file_id) REFERENCES input_files(id) ON UPDATE CASCADE ON DELETE CASCADE
);