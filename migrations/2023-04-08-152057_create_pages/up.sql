CREATE TABLE pages (
  input_file_id TEXT NOT NULL PRIMARY KEY CHECK(length(input_file_id) < 512),

  front_matter TEXT,
  -- content offset
  offset INTEGER NOT NULL,

  date TIMESTAMP,
  description TEXT,
  excerpt TEXT,
  draft BOOLEAN NOT NULL DEFAULT false,
  expiry_date TIMESTAMP,
  keywords TEXT,
  template TEXT,
  publish_date TIMESTAMP,
  summary TEXT,
  title TEXT,

  FOREIGN KEY(input_file_id) REFERENCES input_files(id) ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE page_aliases (
  input_file_id TEXT NOT NULL CHECK(length(input_file_id) < 512),
  alias TEXT NOT NULL,

  PRIMARY KEY(input_file_id, alias),

  FOREIGN KEY(input_file_id) REFERENCES input_files(id) ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TABLE page_tags (
  input_file_id TEXT NOT NULL CHECK(length(input_file_id) < 512),
  tag TEXT NOT NULL,

  PRIMARY KEY(input_file_id, tag),

  FOREIGN KEY(input_file_id) REFERENCES input_files(id) ON UPDATE CASCADE ON DELETE CASCADE
);