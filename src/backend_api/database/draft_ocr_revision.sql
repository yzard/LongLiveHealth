ALTER TABLE observations ADD COLUMN ocr_revision INTEGER NOT NULL DEFAULT 1;
PRAGMA user_version = 2;
