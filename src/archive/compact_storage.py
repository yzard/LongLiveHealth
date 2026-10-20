"""Compact the current database offline without changing original raw files."""
import argparse
import fcntl
import json
import os
import sqlite3
import time
import uuid
from pathlib import Path

import check_export
import restore_queries
import storage_queries as queries

SCHEMA = Path(__file__).resolve().parents[1] / 'backend_api/database/schema.sql'


def compact(root):
    check_export.require(root.is_absolute(), 'Data directory must be absolute')
    database = root / 'database.sqlite'
    check_export.require(database.is_file(), 'Existing database required')
    with (root / 'server.lock').open('a+b') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return compact_locked(root, database)


def compact_locked(root, database):
    temporary = root / ('.compact-' + str(uuid.uuid4()) + '.sqlite')
    backup = root / 'backups' / ('before-compaction-' + str(time.time_ns()))
    source = sqlite3.connect(f'file:{database}?mode=rw', uri=True)
    source.execute(queries.COPY_CACHE)
    source.row_factory = sqlite3.Row
    check_export.require(source.execute('PRAGMA user_version').fetchone()[0] == 7, 'Only current schema 7 can be compacted')
    check_export.require(source.execute('PRAGMA wal_checkpoint(TRUNCATE)').fetchone()[0] == 0, 'Database checkpoint is busy')
    canonical = sqlite3.connect(':memory:')
    canonical.executescript(SCHEMA.read_text())
    definitions = canonical.execute(queries.SCHEMA).fetchall()
    tables = sorted(name for kind, name, _ in definitions if kind == 'table')
    check_export.require(set(row[0] for row in source.execute(queries.TABLES)) == set(tables), 'Unknown source tables')
    for table in tables:
        check_export.require(
            sorted(map(tuple, restore_queries.columns(canonical, table))) == sorted(map(tuple, restore_queries.columns(source, table))),
            'Source columns differ from schema: ' + table,
        )
    counts = {}
    destination = None
    try:
        temporary.touch(mode=0o600, exist_ok=False)
        destination = sqlite3.connect(temporary)
        destination.execute(queries.COPY_CACHE)
        destination.execute('PRAGMA journal_mode=DELETE')
        destination.execute('PRAGMA synchronous=FULL')
        for kind, _, statement in definitions:
            if kind == 'table':
                destination.execute(statement)
        destination.commit()
        for table in tables:
            if table == 'health_totals':
                continue
            columns = [r[0] for r in restore_queries.columns(canonical, table)]
            insert = restore_queries.insert(table, columns)
            count = 0
            for row in source.execute(queries.select_rows(table)):
                destination.execute(insert, [row[c] for c in columns])
                count += 1
                if count % 10000 == 0:
                    destination.commit()
                    print(json.dumps({'table': table, 'copied': count}), flush=True)
            destination.commit()
            check_export.require(
                source.execute(queries.count_rows(table)).fetchone()[0] == count,
                'Source row count differs: ' + table,
            )
            counts[table] = count
        print(json.dumps({"phase": "building_indexes"}), flush=True)
        for kind, _, statement in definitions:
            if kind == 'index':
                destination.execute(statement)
        destination.execute(queries.REBUILD_TOTALS)
        for kind, _, statement in definitions:
            if kind == 'trigger':
                destination.execute(statement)
        destination.execute(restore_queries.SET_VERSION)
        destination.commit()
        print(json.dumps({'phase': 'integrity_check'}), flush=True)
        check_export.require(destination.execute('PRAGMA integrity_check').fetchall() == [('ok',)], 'Compacted database integrity failed')
        check_export.require(not destination.execute('PRAGMA foreign_key_check').fetchall(), 'Compacted foreign keys failed')
        print(json.dumps({'phase': 'checking_current_original_links'}), flush=True)
        check_export.require(destination.execute(queries.CHECK_ORIGINALS).fetchone()[0] == 0, 'Current health record has no matching original index')
        for table, count in counts.items():
            check_export.require(destination.execute(f'SELECT COUNT(*) FROM "{table}"').fetchone()[0] == count, 'Row count mismatch')
        destination.close()
        destination = None
        source.close()
        canonical.close()
        backup.mkdir(parents=True, mode=0o700)
        os.link(database, backup / 'database.sqlite')
        receipt = {'tables': counts, 'old_bytes': database.stat().st_size, 'new_bytes': temporary.stat().st_size}
        (backup / 'receipt.json').write_text(json.dumps(receipt, indent=2))
        directories = {root, root / 'raw', backup, backup.parent}
        with (backup / 'receipt.json').open('rb') as stream:
            os.fsync(stream.fileno())
        for directory in directories:
            fd = os.open(directory, os.O_RDONLY)
            try:
                os.fsync(fd)
            finally:
                os.close(fd)
        with temporary.open('rb') as stream:
            os.fsync(stream.fileno())
        # Original is closed/checkpointed. Replace atomically; never expose a missing DB.
        for suffix in ('-wal', '-shm'):
            Path(str(database) + suffix).unlink(missing_ok=True)
        os.replace(temporary, database)
        fd = os.open(root, os.O_RDONLY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
        print(json.dumps({'status': 'complete', 'backup': str(backup), **receipt}), flush=True)
        return receipt
    finally:
        if destination is not None:
            destination.close()
        source.close()
        canonical.close()
        temporary.unlink(missing_ok=True)
        Path(str(temporary) + '-journal').unlink(missing_ok=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data-dir', required=True, type=Path)
    args = parser.parse_args()
    compact(args.data_dir)
