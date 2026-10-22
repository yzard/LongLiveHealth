"""Verify startup never changes existing data ownership; use disposable data only."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument('image')
args = parser.parse_args()
with tempfile.TemporaryDirectory(prefix='longlivehealth-permissions-') as directory:
    root = Path(directory)
    base = ['docker', 'run', '--rm', '--network', 'none', '-v', f'{root}:/data']
    subprocess.run(base + ['--entrypoint', 'sh', args.image, '-c', 'touch /data/locked; chmod 600 /data/locked'], check=True)
    locked = root / 'locked'
    before = locked.stat()
    identity = ['-e', f'PUID={os.getuid()}', '-e', f'PGID={os.getgid()}', '-e', 'UMASK=077', '--entrypoint', '/app/entrypoint.sh']
    try:
        subprocess.run(base + identity + [args.image, '/app/longlivehealth', '--help'], check=True, stdout=subprocess.DEVNULL)
        after = locked.stat()
        assert (after.st_uid, after.st_gid, after.st_mode) == (before.st_uid, before.st_gid, before.st_mode)
        denied = subprocess.run(base + identity + [args.image, 'sh', '-c', 'cat /data/locked'], capture_output=True, text=True)
        assert denied.returncode != 0 and 'Permission denied' in denied.stderr
        uid = subprocess.check_output(base + identity + [args.image, 'id', '-u'], text=True).strip()
        assert uid == str(os.getuid())
        print('Startup preserves ownership and permissions; configured non-root identity remains active.')
    finally:
        subprocess.run(base + ['--entrypoint', 'sh', args.image, '-c', 'rm /data/locked'], check=True)
