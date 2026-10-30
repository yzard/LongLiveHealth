"""Offline, resumable conversion of Apple Health JSON files into category frames.

The server must be stopped. Original files survive until every frame and its index
are durable. Interrupted copying resumes from the committed staging index; only
uncommitted file tails are truncated. Schema publication is one transaction.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import fcntl
import hashlib
import json
import os
import re
import sqlite3
from pathlib import Path

import pack_health_queries as queries

SCHEMA = Path(__file__).resolve().parents[1] / 'backend_api/database/schema.sql'


def category_file(category):
    if not re.fullmatch('[a-z0-9_-]{1,180}', category) or category.startswith('encoded-'):
        category = 'encoded-' + hashlib.sha256(category.encode()).hexdigest()
    return category + '.frames'


def fsync_directory(path):
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def checked_path(root, relative):
    parts = relative.split('/')
    if len(parts) != 4 or parts[0] != 'raw' or any(part in ('', '.', '..') or '\\' in part for part in parts):
        raise ValueError('Invalid original path')
    return root / relative


def clean_originals(root, manifest):
    if not manifest.exists():
        return 0
    # Every Apple original has moved to a frame before schema publication.
    # Scan legacy JSONs in directory order rather than looking up millions of
    # random UUID paths from the manifest. Never remove category frame files.
    directories = set()
    with manifest.open() as paths:
        for line in paths:
            path = checked_path(root, json.loads(line))
            if path.suffix != '.json' or path.parent.name != 'apple_health':
                raise ValueError('Unexpected cleanup path')
            directories.add(path.parent)
    removed = 0
    with ThreadPoolExecutor(max_workers=16) as workers:
        for directory in directories:
            if not directory.exists():
                continue
            if not directory.resolve().is_relative_to(root.resolve()):
                raise ValueError('Cleanup directory escapes data root')
            descriptor = os.open(directory, os.O_RDONLY | os.O_DIRECTORY)
            try:
                def unlink(name):
                    try:
                        os.unlink(name, dir_fd=descriptor)
                        return 1
                    except FileNotFoundError:
                        return 0
                while True:
                    found = 0
                    with os.scandir(directory) as entries:
                        chunk = []
                        for entry in entries:
                            if not entry.name.endswith('.json'):
                                continue
                            chunk.append(entry.name)
                            found += 1
                            if len(chunk) == 4096:
                                removed += sum(workers.map(unlink, chunk))
                                chunk.clear()
                        removed += sum(workers.map(unlink, chunk))
                    if not found:
                        break
                    print(json.dumps({'phase': 'cleanup', 'removed': removed}), flush=True)
                os.fsync(descriptor)
            finally:
                os.close(descriptor)
    manifest.unlink()
    fsync_directory(root)
    return removed


def read_original(root, row):
    if row['platform'] != 'apple_health':
        return None, None
    with checked_path(root, row['raw_path']).open('rb') as stream:
        data = stream.read(40 * 1024 * 1024 + 1)
    if not 0 < len(data) <= 40 * 1024 * 1024:
        raise ValueError('Invalid raw original size')
    category = json.loads(data)['record_type']
    if not isinstance(category, str) or not category:
        raise ValueError('Original has no record category')
    return data, category


def read_rows(root, rows):
    # Bounded read-ahead lets NAS disks schedule small-file reads concurrently.
    # At most sixteen originals are prefetched; writes and SQLite stay serialized.
    with ThreadPoolExecutor(max_workers=16) as readers:
        for start in range(0, len(rows), 16):
            chunk = rows[start:start+16]
            pending = [readers.submit(read_original, root, row) for row in chunk]
            for row, future in zip(chunk, pending):
                data, category = future.result()
                yield row, data, category


def pack(root):
    if not root.is_absolute() or not (root / 'database.sqlite').is_file():
        raise ValueError('Existing absolute data directory required')
    with (root / 'server.lock').open('a+b') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return pack_locked(root)


def pack_locked(root):
    manifest = root / '.packed-originals-cleanup.jsonl'
    with sqlite3.connect(root / 'database.sqlite') as db:
        db.row_factory = sqlite3.Row
        version = db.execute(queries.VERSION).fetchone()[0]
        if version == 8:
            return {'status': 'complete', 'removed': clean_originals(root, manifest)}
        if version != 7:
            raise ValueError('Expected schema 7 or an already converted schema 8')
        db.execute(queries.FULL_SYNC)
        db.execute(queries.COPY_CACHE)
        # The server is stopped: let durable WAL commits accumulate and checkpoint
        # on close, instead of repeatedly rewriting random index pages per batch.
        db.execute(queries.DEFER_CHECKPOINT)
        db.execute(queries.FOREIGN_KEYS)
        canonical = sqlite3.connect(':memory:')
        canonical.executescript(SCHEMA.read_text())
        create_table = canonical.execute(queries.SCHEMA_TABLE).fetchone()[0]
        indexes = [row[0] for row in canonical.execute(queries.SCHEMA_INDEXES)]
        canonical.close()
        if not db.execute(queries.STAGING_EXISTS).fetchone()[0]:
            db.execute(create_table.replace('CREATE TABLE health_originals', 'CREATE TABLE packed_health_originals', 1))
        db.execute(queries.CREATE_END_INDEX)
        db.commit()
        copied = 0
        cursor = (0,)
        while True:
            rows = db.execute(queries.MISSING, cursor).fetchall()
            if not rows:
                break
            buffers = {}
            prepared = []
            buffered_bytes = 0
            for row, data, category in read_rows(root, rows):
                offset = length = None
                path = row['raw_path']
                if row['platform'] == 'apple_health':
                    path = f"raw/{row['user_id']}/apple_health/{category_file(category)}"
                    if path not in buffers:
                        end = db.execute(queries.END, (path,)).fetchone()
                        buffers[path] = [end[0] if end else 0, bytearray()]
                    end, buffer = buffers[path]
                    offset, length = end + len(buffer), len(data)
                    buffer.extend(length.to_bytes(8, 'little'))
                    buffer.extend(data)
                    buffered_bytes += len(data) + 8
                prepared.append(tuple(row[key] for key in ('user_id','platform','source_id','record_id','raw_id')) + (path, row['received_at'], offset, length))
                cursor = (row["migration_rowid"],)
                if buffered_bytes >= 32 * 1024 * 1024:
                    break
            for path, (end, buffer) in buffers.items():
                target = checked_path(root, path)
                target.parent.mkdir(parents=True, exist_ok=True)
                with target.open('r+b' if target.exists() else 'w+b') as output:
                    output.seek(0, os.SEEK_END)
                    if output.tell() < end:
                        raise ValueError('Packed file shorter than committed staging index')
                    output.truncate(end)
                    output.seek(end)
                    output.write(buffer)
                    output.flush()
                    os.fsync(output.fileno())
                fsync_directory(target.parent)
            db.executemany(queries.INSERT, prepared)
            db.commit()
            copied += len(prepared)
            print(json.dumps({'phase': 'packing', 'copied_this_run': copied}), flush=True)
        old_count, new_count = db.execute(queries.COUNTS).fetchone()
        if old_count != new_count:
            raise ValueError('Original index count mismatch')
        with manifest.open('w') as output:
            for row in db.execute(queries.OLD_APPLE_PATHS):
                output.write(json.dumps(row[0]) + '\n')
            output.flush()
            os.fsync(output.fileno())
        fsync_directory(root)
        db.execute(queries.BEGIN_WRITE)
        db.execute(queries.DROP_OLD)
        db.execute(queries.RENAME)
        db.execute(queries.DROP_END_INDEX)
        for statement in indexes:
            db.execute(statement)
        if db.execute(queries.CHECK_KEYS).fetchone() is not None or db.execute(queries.CHECK_LINKS).fetchone()[0]:
            raise ValueError('Converted original links failed validation')
        db.execute(queries.INVALIDATE_EXPORTS)
        db.execute(queries.SET_VERSION)
        db.commit()
    removed = clean_originals(root, manifest)
    return {'status': 'complete', 'originals': new_count, 'removed': removed}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data-dir', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(pack(args.data_dir)), flush=True)


if __name__ == '__main__':
    main()
