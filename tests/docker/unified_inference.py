"""Opt-in real-model check against a marked, isolated API container. No production data."""

import argparse
import json
import secrets
import time
import urllib.request
import uuid
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--data-dir', type=Path, required=True)
args = parser.parse_args()
assert (args.data_dir / '.synthetic-check').read_text().strip() == 'synthetic-only'
base = 'http://127.0.0.1:8080/api/v1/'
token = None


def api(path, value=None, body=None, headers=None):
    data = body if body is not None else json.dumps(value or {}).encode()
    request = urllib.request.Request(
        base + path,
        data=data,
        headers={
            'Content-Type': 'application/json',
            **({'Authorization': 'Bearer ' + token} if token else {}),
            **(headers or {}),
        },
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def poll(action):
    until = time.monotonic() + 960
    while time.monotonic() < until:
        result = action()
        if result['status'] in ['succeeded', 'ready']:
            return result
        if result['status'] in ['failed', 'stale', 'canceled']:
            raise AssertionError(result)
        time.sleep(1)
    raise AssertionError('Synthetic job timed out')


session = api('session/login', {'username': 'admin', 'password': 'admin'})
token = session['token']
password = secrets.token_urlsafe(24)
api('user/password/change', {'current_password': 'admin', 'new_password': password})
token = api('session/login', {'username': 'admin', 'password': password})['token']
image = (Path(__file__).parent / 'fixtures/synthetic-blood-work.png').read_bytes()
boundary = 'llh-synthetic-only'
body = (
    (
        '--'
        + boundary
        + '\r\nContent-Disposition: form-data; name="file"; filename="synthetic-blood-work.png"\r\nContent-Type: image/png\r\n\r\n'
    ).encode()
    + image
    + ('\r\n--' + boundary + '--\r\n').encode()
)
upload = api(
    'files/upload',
    body=body,
    headers={'Content-Type': 'multipart/form-data; boundary=' + boundary, 'X-Upload-Id': str(uuid.uuid4())},
)
job_id = upload['job']['job_id']
poll(lambda: next(job for job in api('jobs/list', {'limit': 100})['jobs'] if job['job_id'] == job_id))
report_id = upload['file']['file_id']
report = api('reports/get', {'report_id': report_id})
rows = report['observations']
assert len(rows) == 2 and all(row['status'] == 'pending' for row in rows)
assert all(row['payload']['sampled_at'].startswith('2026-11-05') for row in rows)
output = report['extraction_outputs'][0]
original = api(
    'reports/extraction/get',
    {'report_id': report_id, 'run_id': output['run_id'], 'page': output['page'], 'stage': 'ocr'},
)
envelope = json.loads(original['response_body'])
assert envelope['engine'] == 'strata' and envelope['model'] == 'qwen3.8-flash-next-iq3_s'
review = api(
    'reports/review',
    {
        'report_id': report_id,
        'expected_revision': report['report']['revision'],
        'context': None,
        'observations': [
            {
                'observation_id': row['observation_id'],
                'expected_revision': row['revision'],
                'status': 'confirmed',
                'payload': row['payload'],
            }
            for row in rows
        ],
    },
)
run_id = review['analysis_trigger']['run_id']
analysis = poll(lambda: api('analysis/get', {'run_id': run_id}))
assert analysis['output']['verification_status'] == 'unverified'
print(
    json.dumps(
        {
            'single_api_service': True,
            'ocr_markers': len(rows),
            'collection_dates_correct': True,
            'pending_until_review': True,
            'analysis_ready': True,
            'model': envelope['model'],
        }
    )
)
