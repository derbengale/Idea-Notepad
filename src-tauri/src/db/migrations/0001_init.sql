CREATE TABLE meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE project (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  color       TEXT NOT NULL DEFAULT '#64748b',
  icon        TEXT NOT NULL DEFAULT 'folder',
  sort_order  INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);

CREATE TABLE schema (
  id                TEXT PRIMARY KEY,
  key               TEXT NOT NULL UNIQUE,
  name              TEXT NOT NULL,
  icon              TEXT NOT NULL DEFAULT 'file-text',
  color             TEXT NOT NULL DEFAULT '#64748b',
  sort_order        INTEGER NOT NULL DEFAULT 0,
  title_field_id    TEXT,
  subtitle_field_id TEXT,
  badge_field_id    TEXT,
  format_version    INTEGER NOT NULL DEFAULT 1,
  created_at        TEXT NOT NULL,
  updated_at        TEXT NOT NULL
);

CREATE TABLE field_def (
  id           TEXT PRIMARY KEY,
  schema_id    TEXT NOT NULL REFERENCES schema(id) ON DELETE CASCADE,
  key          TEXT NOT NULL,
  label        TEXT NOT NULL,
  type         TEXT NOT NULL CHECK (type IN
                 ('text','longtext','number','scale','status','date','checkbox','tags','formula')),
  required     INTEGER NOT NULL DEFAULT 0,
  sort_order   INTEGER NOT NULL DEFAULT 0,
  config_json  TEXT NOT NULL DEFAULT '{}',
  expression   TEXT,
  show_in_list INTEGER NOT NULL DEFAULT 1,
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL,
  UNIQUE (schema_id, key)
);

CREATE TABLE entry (
  id               TEXT PRIMARY KEY,
  schema_id        TEXT NOT NULL REFERENCES schema(id) ON DELETE RESTRICT,
  project_id       TEXT REFERENCES project(id) ON DELETE SET NULL,
  title            TEXT NOT NULL,
  body_md          TEXT NOT NULL DEFAULT '',
  created_at       TEXT NOT NULL,
  updated_at       TEXT NOT NULL,
  deleted_at       TEXT,
  drive_file_id    TEXT,
  drive_etag       TEXT,
  drive_modified_at TEXT,
  local_rev        INTEGER NOT NULL DEFAULT 0,
  sync_state       TEXT NOT NULL DEFAULT 'local'
                   CHECK (sync_state IN ('local','clean','dirty','conflict')),
  conflict_of      TEXT REFERENCES entry(id) ON DELETE SET NULL,
  extra_json       TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_entry_schema   ON entry(schema_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_entry_project  ON entry(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_entry_updated  ON entry(updated_at DESC);
CREATE INDEX idx_entry_deleted  ON entry(deleted_at);

CREATE TABLE entry_value (
  entry_id   TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
  field_id   TEXT NOT NULL REFERENCES field_def(id) ON DELETE CASCADE,
  text_value TEXT,
  num_value  REAL,
  date_value TEXT,
  bool_value INTEGER,
  PRIMARY KEY (entry_id, field_id)
);

CREATE INDEX idx_value_field_num  ON entry_value(field_id, num_value);
CREATE INDEX idx_value_field_text ON entry_value(field_id, text_value);

CREATE TABLE tag (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  color      TEXT NOT NULL DEFAULT '#64748b',
  created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_tag_name ON tag(name COLLATE NOCASE);

CREATE TABLE entry_tag (
  entry_id TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
  tag_id   TEXT NOT NULL REFERENCES tag(id) ON DELETE CASCADE,
  PRIMARY KEY (entry_id, tag_id)
);

CREATE INDEX idx_entry_tag_tag ON entry_tag(tag_id);

CREATE TABLE link (
  id         TEXT PRIMARY KEY,
  from_id    TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
  to_id      TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL,
  UNIQUE (from_id, to_id),
  CHECK (from_id <> to_id)
);

CREATE INDEX idx_link_to ON link(to_id);

CREATE TABLE md_link (
  from_entry_id TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
  to_entry_id   TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
  PRIMARY KEY (from_entry_id, to_entry_id)
);

CREATE INDEX idx_md_link_to ON md_link(to_entry_id);

CREATE TABLE attachment (
  id            TEXT PRIMARY KEY,
  entry_id      TEXT NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
  kind          TEXT NOT NULL CHECK (kind IN ('image','file','sketch')),
  file_name     TEXT NOT NULL,
  mime_type     TEXT NOT NULL,
  byte_size     INTEGER NOT NULL,
  sha256        TEXT NOT NULL,
  local_path    TEXT NOT NULL,
  drive_file_id TEXT,
  preview_path  TEXT,
  created_at    TEXT NOT NULL
);

CREATE INDEX idx_attachment_entry ON attachment(entry_id);

CREATE TABLE sync_file (
  drive_file_id  TEXT PRIMARY KEY,
  path           TEXT NOT NULL UNIQUE,
  kind           TEXT NOT NULL,
  etag           TEXT,
  modified_at    TEXT,
  local_rev      INTEGER NOT NULL DEFAULT 0,
  sha256         TEXT,
  last_synced_at TEXT
);

CREATE TABLE outbox (
  id              TEXT PRIMARY KEY,
  op              TEXT NOT NULL,
  target          TEXT NOT NULL,
  payload_json    TEXT NOT NULL,
  attempts        INTEGER NOT NULL DEFAULT 0,
  next_attempt_at TEXT,
  created_at      TEXT NOT NULL,
  last_error      TEXT
);

CREATE INDEX idx_outbox_next ON outbox(next_attempt_at);

CREATE VIRTUAL TABLE entry_fts USING fts5(
  entry_id UNINDEXED,
  title,
  body,
  field_text,
  tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER entry_fts_ai AFTER INSERT ON entry BEGIN
  INSERT INTO entry_fts(entry_id, title, body, field_text)
  VALUES (new.id, new.title, new.body_md, '');
END;

CREATE TRIGGER entry_fts_au AFTER UPDATE OF title, body_md ON entry BEGIN
  DELETE FROM entry_fts WHERE entry_id = old.id;
  INSERT INTO entry_fts(entry_id, title, body, field_text)
  VALUES (
    new.id,
    new.title,
    new.body_md,
    coalesce((SELECT group_concat(text_value, ' ')
              FROM entry_value
              WHERE entry_id = new.id AND text_value IS NOT NULL), '')
  );
END;

CREATE TRIGGER entry_fts_ad AFTER DELETE ON entry BEGIN
  DELETE FROM entry_fts WHERE entry_id = old.id;
END;
