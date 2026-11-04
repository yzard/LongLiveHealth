"""Remove obsolete Android workspaces while preserving Docker-bound APK artifacts."""

import argparse
import errno
import fcntl
import json
import os
import shutil
import subprocess
import sys
from contextlib import contextmanager
from pathlib import Path

APK_DIRECTORY = Path('.build/Android/app/outputs/apk/debug')
WORKSPACES = ('client', 'research')
TOOLCHAIN = 'longlivehealth-android-toolchain:6.3.2'


@contextmanager
def build_lock(root: Path):
    path = root / 'build/.locks/docker.lock'
    path.parent.mkdir(parents=True, exist_ok=True)
    path.touch(exist_ok=True)
    # The normal build already owns fd 9. Do not unlock its shared description.
    try:
        inherited = os.fstat(9)
    except OSError as error:
        if error.errno != errno.EBADF:
            raise
        inherited = None
    expected = path.stat()
    if inherited and (inherited.st_dev, inherited.st_ino) == (expected.st_dev, expected.st_ino):
        fcntl.flock(9, fcntl.LOCK_EX | fcntl.LOCK_NB)
        yield
    else:
        with path.open('a+b') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            yield


def recognized_workspaces(cache: Path, artifacts: Path) -> list[str]:
    result = []
    for name in WORKSPACES:
        path = cache / name
        if not path.exists():
            continue
        if path.is_symlink():
            raise ValueError(f'Refusing a linked Android workspace: {name}')
        link = path / APK_DIRECTORY
        prepared = (path / 'Package.swift').is_file() and (path / '.build').is_dir()
        artifact_link = link.is_symlink() and link.resolve().is_relative_to(artifacts.resolve())
        if prepared or artifact_link:
            result.append(name)
        else:
            raise ValueError(f'Unrecognized Android workspace; no files removed: {name}')
    return result


def artifact_mount_plan(cache: Path, workspaces: list[str], containers: list[dict]) -> list[dict]:
    keep = {}
    for container in containers:
        sources = {mount['Source'] for mount in container.get('Mounts', []) if mount['Type'] == 'bind'}
        sources.update(bind.split(':', 1)[0] for bind in container.get('HostConfig', {}).get('Binds', []) or [])
        for mount in container.get('HostConfig', {}).get('Mounts', []) or []:
            if mount['Type'] == 'bind':
                sources.add(mount['Source'])
        for source in sources:
            source = Path(os.path.normpath(source))
            if cache.is_relative_to(source):
                raise ValueError('A container mounts the Android workspace; preserve it until that build stops')
            if not source.is_relative_to(cache):
                continue
            relative = source.relative_to(cache)
            for name in workspaces:
                directory = Path(name) / APK_DIRECTORY
                if relative == directory or relative.is_relative_to(directory):
                    keep.setdefault(str(directory), container['Id'][:12])
                    break
            else:
                raise ValueError(f'A container uses compilation files: {relative}; no files removed')
    return [{'source': source, 'container': container} for source, container in sorted(keep.items())]


def apply_plan(cache: Path, artifacts: Path, workspaces: list[str], mounts: list[dict], uid: int, gid: int) -> dict:
    if cache.is_symlink() or artifacts.is_symlink():
        raise ValueError('Cleanup roots must not be symlinks')
    if any(name not in WORKSPACES for name in workspaces):
        raise ValueError('Only declared Android workspaces may be removed')
    moves = []
    keep = []
    # Validate every destination and source before moving or deleting anything.
    for mount in mounts:
        relative = Path(mount['source'])
        if relative not in [Path(name) / APK_DIRECTORY for name in workspaces]:
            raise ValueError('Unexpected artifact source')
        if not mount['container'].isalnum() or len(mount['container']) != 12:
            raise ValueError('Unexpected container identity')
        source = cache / relative
        if source.parent.resolve() != source.parent:
            raise ValueError('Artifact parent directories must not be symlinks')
        if source.is_symlink():
            if not source.resolve().is_relative_to(artifacts.resolve()) or not source.is_dir():
                raise ValueError('Unexpected artifact link')
            keep.append(source)
            continue
        if source.resolve() != source or not source.is_dir():
            raise ValueError('Expected an ordinary APK directory')
        files = list(source.iterdir())
        if not any(path.suffix == '.apk' for path in files):
            raise ValueError('Mounted directory does not contain an APK')
        if any(path.is_symlink() or not path.is_file() or path.suffix not in ('.apk', '.json') for path in files):
            raise ValueError('Mounted directory contains files other than APK artifacts')
        destination = artifacts / mount['container']
        if destination.exists() or destination.is_symlink():
            raise ValueError('Artifact destination already exists; neither copy was changed')
        if source.stat().st_dev != artifacts.parent.stat().st_dev:
            raise ValueError('Artifacts must stay on the same filesystem to preserve the active bind mount')
        moves.append((source, destination))
        keep.append(source)
    for source, destination in moves:
        artifacts.mkdir(parents=True, exist_ok=True)
        os.chown(artifacts, uid, gid)
        source.rename(destination)
        # Moving preserves the directory inode held by the live Docker bind mount.
        # The link also preserves the source path recorded for container restarts.
        try:
            source.symlink_to(os.path.relpath(destination, source.parent), target_is_directory=True)
        except OSError:
            destination.rename(source)
            raise
        os.chown(destination, uid, gid)
        for path in destination.iterdir():
            os.chown(path, uid, gid)

    def prune(path: Path):
        if path in keep:
            return
        if path.is_symlink():
            path.unlink()
            return
        if any(preserved.is_relative_to(path) for preserved in keep):
            for child in path.iterdir():
                prune(child)
        elif path.is_dir():
            shutil.rmtree(path)
        elif path.exists():
            path.unlink()

    for name in workspaces:
        prune(cache / name)
    for source in keep:
        os.lchown(source, uid, gid)
        for parent in source.parents:
            if not parent.is_relative_to(cache):
                break
            os.chown(parent, uid, gid)
    return {
        'removed_workspaces': workspaces,
        'preserved_artifact_links': [str(path.relative_to(cache)) for path in keep],
    }


def clean_legacy_android(root: Path) -> None:
    cache = root / 'build/android'
    artifacts = root / 'dist/android/emulator'
    if not cache.exists():
        return
    if root.resolve() != root or cache.resolve() != cache or artifacts.parent.resolve() != artifacts.parent:
        raise ValueError('Cleanup requires canonical project paths')
    with build_lock(root):
        workspaces = recognized_workspaces(cache, artifacts)
        if not workspaces:
            return
        apk = root / 'dist/android/longlivehealth.apk'
        if not apk.is_file() or apk.stat().st_size == 0:
            raise ValueError('A successfully exported APK is required before Android cleanup')
        ids = subprocess.check_output(['docker', 'ps', '-aq'], text=True).split()
        containers = json.loads(subprocess.check_output(['docker', 'inspect', *ids], text=True)) if ids else []
        mounts = artifact_mount_plan(cache, workspaces, containers)
        plan = {'workspaces': workspaces, 'mounts': mounts, 'uid': os.getuid(), 'gid': os.getgid()}
        # Both artifact paths need one filesystem view for an inode-preserving rename.
        # The worker only mutates build/android and dist/android; protect other state too.
        command = ['docker', 'run', '--rm', '-i', '--network', 'none', '-v', f'{root}:/project']
        for relative in ['src', 'tests', 'docs', 'playground', '.git', 'build/backups', 'dist/ios']:
            path = root / relative
            if path.exists():
                command.extend(['-v', f'{path}:/project/{relative}:ro'])
        command.extend(
            [
                '-v',
                f'{Path(__file__).resolve()}:/cleanup.py:ro',
                '--entrypoint',
                'python3',
                TOOLCHAIN,
                '/cleanup.py',
                'worker',
            ]
        )
        result = subprocess.run(command, input=json.dumps(plan), text=True, capture_output=True, check=True)
        print('Cleaned obsolete Android compilation directories: ' + result.stdout.strip())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('operation', choices=['clean', 'worker'])
    parser.add_argument('--project', type=Path)
    args = parser.parse_args()
    if args.operation == 'worker':
        plan = json.load(sys.stdin)
        print(
            json.dumps(
                apply_plan(
                    Path('/project/build/android'),
                    Path('/project/dist/android/emulator'),
                    plan['workspaces'],
                    plan['mounts'],
                    plan['uid'],
                    plan['gid'],
                )
            )
        )
    else:
        if args.project is None or not args.project.is_absolute():
            parser.error('--project requires an absolute project directory')
        clean_legacy_android(args.project)


if __name__ == '__main__':
    main()
