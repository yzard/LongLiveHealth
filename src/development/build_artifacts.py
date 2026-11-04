"""Validate the shared model and remove declared, inactive build workspaces."""

import argparse
import fcntl
import json
import re
import shutil
from contextlib import ExitStack
from pathlib import Path

MODEL_SHA256 = '74d2c57145e6ff11d1d2faa79594477f9bc903a611af1fb20218189fbbb77d82'


def model_blob(layout: Path, descriptor: dict) -> Path:
    digest = descriptor['digest']
    if not re.fullmatch(r'sha256:[0-9a-f]{64}', digest):
        raise ValueError('Invalid model OCI digest')
    path = layout / 'blobs' / 'sha256' / digest.split(':')[1]
    if path.is_symlink() or path.stat().st_size != descriptor['size']:
        raise ValueError('Missing or incomplete shared model blob')
    return path


def validate_model(layout: Path) -> None:
    if not layout.is_absolute() or layout.is_symlink():
        raise ValueError('Model layout must be an absolute directory')
    if json.loads((layout / 'oci-layout').read_text()) != {'imageLayoutVersion': '1.0.0'}:
        raise ValueError('Unsupported model OCI layout')
    index = json.loads((layout / 'index.json').read_text())
    if len(index['manifests']) != 1:
        raise ValueError('Expected a single model image')
    manifest = json.loads(model_blob(layout, index['manifests'][0]).read_text())
    config = json.loads(model_blob(layout, manifest['config']).read_text())
    if config['config'].get('Labels', {}).get('org.shared-model.sha256') != MODEL_SHA256:
        raise ValueError('Shared layout does not contain the pinned Qwen 3.8 model')
    if len(manifest['layers']) != 2 or len(config['rootfs']['diff_ids']) != 2:
        raise ValueError('Shared layout must contain only model and license layers')
    for layer in manifest['layers']:
        model_blob(layout, layer)


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
    model = commands.add_parser('model')
    model.add_argument('layout', type=Path)
    cleanup = commands.add_parser('clean')
    cleanup.add_argument('--project', type=Path, required=True)
    cleanup.add_argument('--work', type=Path, required=True)
    args = parser.parse_args()
    if args.command == 'model':
        validate_model(args.layout)
    else:
        from android_cleanup import clean_legacy_android

        clean_legacy_android(args.project)
        clean(args.project, args.work)


if __name__ == '__main__':
    main()
