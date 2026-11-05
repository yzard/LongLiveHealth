"""Initialize the API-owned configuration without manufacturing inference credentials."""

import argparse
import os
import tomllib
from pathlib import Path


def needs_update(project: Path) -> bool:
    return not (project / 'playground/backend_api/config.toml').exists()


def prepare(project: Path) -> None:
    if not project.is_absolute():
        raise ValueError('Project path must be absolute')
    root = project / 'playground/backend_api'
    root.mkdir(mode=0o700, parents=True, exist_ok=True)
    target = root / 'config.toml'
    if not target.exists():
        with open(target, 'x', opener=lambda path, flags: os.open(path, flags, 0o600)) as output:
            output.write((project / 'docker/config.toml').read_text())
    config = tomllib.loads(target.read_text())
    if 'ocr' in config or 'providers' in config or 'llm' not in config:
        raise ValueError(
            'Configure unified [llm] settings in API config.toml; obsolete OCR/provider sections are not supported'
        )
    os.chmod(target, 0o600)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('--project', type=Path, required=True)
    parser.add_argument('--needs-update', action='store_true')
    arguments = parser.parse_args()
    if arguments.needs_update:
        print('yes' if needs_update(arguments.project) else 'no')
    else:
        prepare(arguments.project)
        print('API config ready; configure [llm].api_key before starting the service')


if __name__ == '__main__':
    main()
