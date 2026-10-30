"""Verify exact raw preservation, resumability, and safe offline publication."""
import fcntl
import json
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'src/archive'))
import pack_health_raw
import test_compact_storage


class PackHealthTests(unittest.TestCase):
    def fixture(self, root):
        db, raw, payload = test_compact_storage.CompactStorageTests().fixture(root)
        with sqlite3.connect(db) as connection:
            connection.execute('DROP INDEX health_originals_frame')
            connection.execute('DROP INDEX health_originals_file')
            # Recreate the actual previous schema, including its path uniqueness.
            connection.executescript('''
                ALTER TABLE health_originals RENAME TO saved;
                CREATE TABLE health_originals (
                    user_id TEXT NOT NULL, platform TEXT NOT NULL, source_id TEXT NOT NULL,
                    record_id TEXT NOT NULL, raw_id TEXT NOT NULL, raw_path TEXT NOT NULL UNIQUE,
                    received_at INTEGER NOT NULL, PRIMARY KEY(user_id,raw_id),
                    FOREIGN KEY(user_id,platform,source_id,record_id) REFERENCES health_records(user_id,platform,source_id,record_id) ON DELETE CASCADE);
                INSERT INTO health_originals SELECT user_id,platform,source_id,record_id,raw_id,raw_path,received_at FROM saved;
                DROP TABLE saved;
                CREATE INDEX health_originals_identity ON health_originals(user_id,platform,record_id,source_id);
                PRAGMA user_version=7;
            ''')
        return db, raw, payload

    def test_pack_preserves_bytes_ids_projection_and_is_idempotent(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            database, raw, payload = self.fixture(root)
            before = payload.encode()
            result = pack_health_raw.pack(root)
            self.assertEqual(result['originals'], 1)
            self.assertFalse(raw.exists())
            with sqlite3.connect(database) as db:
                path, offset, length = db.execute('SELECT raw_path,raw_offset,raw_length FROM health_originals').fetchone()
                self.assertEqual(db.execute('PRAGMA user_version').fetchone(), (8,))
                self.assertEqual(db.execute('PRAGMA foreign_key_check').fetchall(), [])
                self.assertEqual(db.execute('SELECT raw_id FROM health_records').fetchone(), ('revision',))
                self.assertEqual(db.execute("SELECT COUNT(*) FROM sqlite_master WHERE name='packed_health_originals'").fetchone(), (0,))
            self.assertEqual(offset, 0)
            self.assertEqual(length, len(before))
            self.assertEqual((root / path).read_bytes(), len(before).to_bytes(8, 'little') + before)
            self.assertEqual(pack_health_raw.pack(root)['status'], 'complete')

    def test_missing_original_leaves_old_schema_and_retries(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            database, raw, payload = self.fixture(root)
            raw.unlink()
            with self.assertRaises(FileNotFoundError):
                pack_health_raw.pack(root)
            with sqlite3.connect(database) as db:
                self.assertEqual(db.execute('PRAGMA user_version').fetchone(), (7,))
            raw.write_text(payload)
            target = raw.with_name('steps.frames')
            target.write_bytes(b'uncommitted partial prefix')
            pack_health_raw.pack(root)
            self.assertEqual(target.read_bytes()[8:], payload.encode())

    def test_resume_preserves_committed_prefix_after_later_missing_file(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            database, raw, payload = self.fixture(root)
            with sqlite3.connect(database) as db:
                db.execute("INSERT INTO health_records SELECT user_id,platform,source_id,'second',record_type,start_at,end_at,version,deleted,payload_json,'zz-second' FROM health_records")
                db.execute("INSERT INTO health_originals SELECT user_id,platform,source_id,'second','zz-second',replace(raw_path,'revision.json','missing.json'),received_at FROM health_originals")
            with patch.object(pack_health_raw.queries, 'MISSING', pack_health_raw.queries.MISSING.replace('LIMIT 5000', 'LIMIT 1')):
                with self.assertRaises(FileNotFoundError):
                    pack_health_raw.pack(root)
                target = raw.with_name('steps.frames')
                committed = target.read_bytes()
                with target.open('ab') as output:
                    output.write(b'uncommitted-tail')
                raw.with_name('missing.json').write_text(payload)
                pack_health_raw.pack(root)
                self.assertEqual(target.read_bytes(), committed + committed)
                with sqlite3.connect(database) as db:
                    self.assertEqual(db.execute('SELECT COUNT(*) FROM health_originals').fetchone(), (2,))

    def test_server_lock_prevents_migration(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            with (root / 'server.lock').open('a+b') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                with self.assertRaises(BlockingIOError):
                    pack_health_raw.pack(root)
