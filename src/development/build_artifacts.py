"""Remove declared, inactive build workspaces."""

import argparse
import fcntl
import shutil
from contextlib import ExitStack
from pathlib import Path


def owned_path(root: Path, path: Path) -> Path:
    if not path.is_relative_to(root) or path.resolve() != path:
        raise ValueError('Refusing cleanup outside the project or through a symlink')
    return path


def clean(root: Path, work: Path) -> None:
    if not root.is_absolute() or root.resolve() != root:
        raise ValueError('Project must be an absolute canonical directory')
    if work.parent != root / 'build/docker' or not work.name.startswith('work.'):
        raise ValueError('Unexpected build workspace')
    owned_path(root, work)
    targets = []
    with ExitStack() as locks:
        for relative in ['build/backend_api/target', 'src/backend_api/target']:
            target = owned_path(root, root / relative)
            if not target.exists():
                continue
            if not (target / 'CACHEDIR.TAG').read_text().startswith('Signature: 8a477f597d28d172789f06886806bc55'):
                raise ValueError('Refusing an unrecognized Rust cache')
            for profile in ['debug', 'release']:
                lock_path = target / profile / '.cargo-lock'
                if lock_path.exists():
                    lock = locks.enter_context(lock_path.open('r+b'))
                    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            targets.append(target)
        # Preserve diagnostics if another cache cannot be removed.
        for target in targets + [work]:
            if target.exists():
                shutil.rmtree(target)
    print('Cleaned build workspace, Android/Gradle scratch caches and Rust targets')


def main() -> None:
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest='command', required=True)
    cleanup = commands.add_parser('clean')
    cleanup.add_argument('--project', type=Path, required=True)
    cleanup.add_argument('--work', type=Path, required=True)
    args = parser.parse_args()
    from android_cleanup import clean_legacy_android

    clean_legacy_android(args.project)
    clean(args.project, args.work)


if __name__ == '__main__':
    main()
