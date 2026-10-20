"""Regression coverage for copy-on-write storage migration."""
import fcntl
import hashlib
import json
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'src/archive'))
import compact_storage


class CompactStorageTests(unittest.TestCase):
    def test_projection_digest_matches_server_and_removes_only_query_redundancy(self):
        record = dict(record_id='projection', source_id='synthetic', record_type='synthetic_raw',
                      start_at=1, end_at=2, version=1, deleted=False,
                      payload={'value': 3.5, 'unit': 'count',
                               'series': [1, 1.0, -0.0, 9007199254740993, 'µ 中文', True, None],
                               'device': {'name': 'Watch'},
                               'raw_archive': {'format': 'nskeyedarchiver-secure-v1', 'encoding': 'base64', 'data': 'YQ=='}})
        projection = compact_storage.check_export.health_projection(record)
        self.assertEqual(projection['content_hash'], 'b64eb0da43733256197a4c2f8b28109dfeadf396a9a7a091c6bdff5fcfcaf256')
        self.assertEqual(projection['payload'], {'value': 3.5, 'unit': 'count'})
        self.assertIn('series', record['payload'])

    def fixture(self, root):
        owner = 'ac8d7ca5-85f5-4185-92eb-30e466273ec9'
        db = root / 'database.sqlite'
        connection = sqlite3.connect(db)
        schema = compact_storage.SCHEMA.read_text()
        connection.executescript(schema)
        connection.execute("INSERT INTO users(user_id,username,password_hash,created_at) VALUES (?, 'test', 'hash', 1)", (owner,))
        record = {'record_id': 'sample', 'source_id': 'watch', 'record_type': 'steps', 'start_at': 1, 'end_at': 2, 'version': 1, 'deleted': False, 'payload': {'value': 42, 'unit': 'count', 'raw_archive': {'format': 'nskeyedarchiver-secure-v1', 'encoding': 'base64', 'data': 'YQ==' * 25000}}}
        payload = json.dumps(record)
        relative = f'raw/{owner}/apple_health/revision.json'
        raw = root / relative
        raw.parent.mkdir(parents=True)
        raw.write_text(payload)
        connection.execute('INSERT INTO health_records VALUES (?,?,?,?,?,?,?,?,?,?,?)', (owner, 'apple_health', 'watch', 'sample', 'steps', 1, 2, 1, 0, json.dumps(compact_storage.check_export.health_projection(record)), 'revision'))
        connection.execute('INSERT INTO health_originals VALUES (?,?,?,?,?,?,?)', (owner, 'apple_health', 'watch', 'sample', 'revision', relative, 1))
        connection.execute('PRAGMA user_version=7')
        connection.commit()
        connection.close()
        return db, raw, payload

    def test_compaction_retains_raw_and_backup_and_rebuilds_totals(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            db, raw, payload = self.fixture(root)
            result = compact_storage.compact(root)
            self.assertLessEqual(result['new_bytes'], result['old_bytes'])
            self.assertEqual(raw.read_text(), payload)
            with sqlite3.connect(db) as connection:
                projected = json.loads(connection.execute('SELECT payload_json FROM health_records').fetchone()[0])
                self.assertNotIn('raw_archive', projected['payload'])
                self.assertEqual(len(projected['content_hash']), 64)
                self.assertEqual(connection.execute("SELECT COUNT(*) FROM sqlite_master WHERE name IN ('health_revisions','observation_revisions')").fetchone(), (0,))
                self.assertEqual(connection.execute('SELECT record_count FROM health_totals').fetchone(), (1,))
                self.assertEqual(connection.execute('PRAGMA integrity_check').fetchone(), ('ok',))
            backup = next((root / 'backups').glob('*/database.sqlite'))
            with sqlite3.connect(backup) as connection:
                self.assertEqual(json.loads(connection.execute('SELECT payload_json FROM health_records').fetchone()[0]), compact_storage.check_export.health_projection(json.loads(payload)))

    def test_missing_current_original_index_does_not_replace_database(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            db, raw, payload = self.fixture(root)
            with sqlite3.connect(db) as connection:
                connection.execute('DELETE FROM health_originals')
            with self.assertRaises(compact_storage.check_export.InvalidExport):
                compact_storage.compact(root)
            with sqlite3.connect(db) as connection:
                self.assertEqual(json.loads(connection.execute('SELECT payload_json FROM health_records').fetchone()[0]), compact_storage.check_export.health_projection(json.loads(payload)))
            self.assertFalse(list(root.glob('.compact-*')))

    def test_running_server_blocks_compaction(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.fixture(root)
            with (root / 'server.lock').open('a+b') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                with self.assertRaises(BlockingIOError):
                    compact_storage.compact(root)



if __name__ == '__main__':
    unittest.main()
