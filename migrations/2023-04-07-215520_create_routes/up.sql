CREATE TABLE routes (
  revision_id INTEGER NOT NULL,
  route TEXT NOT NULL,

  input_file_id TEXT NOT NULL,

  PRIMARY KEY(revision_id, route),

  FOREIGN KEY(revision_id) REFERENCES revisions(id) ON UPDATE CASCADE ON DELETE CASCADE,
  FOREIGN KEY(input_file_id) REFERENCES input_files(id) ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE INDEX idx_routes_input_file_id ON routes(input_file_id);