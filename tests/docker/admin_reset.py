"""Verify startup reset semantics against a built image using disposable data."""

import argparse
import json
import os
import secrets
import sqlite3
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def docker(arguments):
    return subprocess.check_output(['docker', *arguments], text=True).strip()


def request(origin, path, body, token):
    headers = {'Content-Type': 'application/json'}
    if token is not None:
        headers['Authorization'] = 'Bearer ' + token
    operation = urllib.request.Request(origin + path, json.dumps(body).encode(), headers)
    try:
        with urllib.request.urlopen(operation, timeout=5) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        return error.code, json.load(error)


def credentials(directory):
    with sqlite3.connect(directory / 'database.sqlite') as connection:
        return connection.execute(
            "SELECT password_hash,credential_version,must_change_password FROM users WHERE username='admin'"
        ).fetchone()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image', required=True)
    options = parser.parse_args()
    output = ROOT / 'build' / 'deployment-check'
    output.mkdir(parents=True, exist_ok=True)
    name = 'longlivehealth-admin-check-' + uuid.uuid4().hex[:12]
    with tempfile.TemporaryDirectory(prefix='admin-reset-', dir=output) as temporary:
        directory = Path(temporary)
        template = (ROOT / 'docker/config.toml').read_text()
        template = template.replace(
            'api_key = "" # Set the shared OCR key here; playground initialization generates it.',
            'api_key = ' + json.dumps(secrets.token_urlsafe(32)),
        )
        (directory / 'config.toml').write_text(template)
        (directory / 'config.toml').chmod(0o600)

        def stop():
            docker(['stop', '--time', '10', name])
            docker(['rm', name])

        def start(reset):
            arguments = [
                'run',
                '-d',
                '--name',
                name,
                '-p',
                '127.0.0.1::8080',
                '-e',
                f'PUID={os.getuid()}',
                '-e',
                f'PGID={os.getgid()}',
                '-v',
                f'{directory}:/data',
            ]
            if reset is not None:
                arguments += ['-e', 'RESET_ADMIN_PASSWORD=' + reset]
            docker(arguments + [options.image])
            port = docker(['port', name, '8080/tcp']).rsplit(':', 1)[1]
            origin = 'http://127.0.0.1:' + port
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline:
                try:
                    with urllib.request.urlopen(origin, timeout=1) as response:
                        if response.status == 200:
                            return origin
                except (urllib.error.URLError, TimeoutError, ConnectionError):
                    pass
                time.sleep(0.1)
            raise AssertionError('Isolated API did not become ready')

        try:
            origin = start(None)
            status, login = request(origin, '/api/v1/session/login', {'username': 'admin', 'password': 'admin'}, None)
            assert status == 200 and login['user']['must_change_password'] is True
            password = 'synthetic-admin-password-' + secrets.token_hex(8)
            status, _ = request(
                origin,
                '/api/v1/user/password/change',
                {'current_password': 'admin', 'new_password': password},
                login['token'],
            )
            assert status == 200
            baseline = credentials(directory)
            assert baseline[2] == 0
            stop()

            for reset in [None, 'false']:
                origin = start(reset)
                assert credentials(directory) == baseline
                status, login = request(
                    origin, '/api/v1/session/login', {'username': 'admin', 'password': password}, None
                )
                assert status == 200 and login['user']['must_change_password'] is False
                status, _ = request(origin, '/api/v1/session/login', {'username': 'admin', 'password': 'admin'}, None)
                assert status == 401
                retained_token = login['token']
                stop()
                print(f'PASS reset={reset!r}: existing password retained')

            for invalid in ['TRUE', '1', '']:
                docker(
                    [
                        'run',
                        '-d',
                        '--name',
                        name,
                        '-e',
                        f'PUID={os.getuid()}',
                        '-e',
                        f'PGID={os.getgid()}',
                        '-e',
                        'RESET_ADMIN_PASSWORD=' + invalid,
                        '-v',
                        f'{directory}:/data',
                        options.image,
                    ]
                )
                result = subprocess.run(
                    ['docker', 'wait', name], check=True, capture_output=True, text=True, timeout=15
                )
                assert result.stdout.strip() != '0'
                assert credentials(directory) == baseline
                docker(['rm', name])
                print(f'PASS reset={invalid!r}: startup rejected without resetting password')

            origin = start('true')
            status, _ = request(origin, '/api/v1/user/get', {}, retained_token)
            assert status == 401
            status, login = request(origin, '/api/v1/session/login', {'username': 'admin', 'password': 'admin'}, None)
            assert status == 200 and login['user']['must_change_password'] is True
            assert credentials(directory)[1] > baseline[1]
            print('PASS reset=true: reset to admin, password change required, previous session revoked')
        finally:
            subprocess.run(
                ['docker', 'rm', '-f', name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False
            )


if __name__ == '__main__':
    main()
