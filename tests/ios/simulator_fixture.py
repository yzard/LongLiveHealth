"""Synthetic API fixture for iOS UI tests; never uses real patient data."""

import base64
import io
import json
import time
import zipfile
from datetime import datetime, timezone, date, timedelta
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

exports = []
reports = []
upload_ids = []
fail_upload = False
upload_delay = 0
fail_review = False
observations = []
context = {}
analysis_enabled = False
runs = []
relation = {}
request_counts = {}
admin_password_changed = False
refresh_failure = False
source_preferences = {"version": 0, "preferences": {"source_aliases": {}, "source_priority": {}, "favorite_metrics": [], "sleep_target_minutes": None}}


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_POST(self):
        global fail_upload, upload_delay, fail_review, context, analysis_enabled, relation, admin_password_changed, refresh_failure, source_preferences
        length = int(self.headers.get('Content-Length', 0))
        raw = self.rfile.read(length)
        body = json.loads(raw) if self.headers.get('Content-Type') == 'application/json' else {}
        route = self.path.removeprefix('/api/v1/')
        request_counts[route] = request_counts.get(route, 0) + 1
        status = 200
        if route == 'health/sync' and self.headers.get('Authorization') == 'Bearer reject-sync':
            status = 503
        if route == 'test/unauthorized':
            status = 401
        if route == 'test/fail-refresh':
            refresh_failure = True
        if route == 'test/reset-counts':
            request_counts.clear()
        if route == 'test/reset':
            request_counts.clear()
            admin_password_changed = False
            refresh_failure = False
            exports.clear()
            reports.clear()
            upload_ids.clear()
            observations.clear()
            source_preferences = {"version": 0, "preferences": {"source_aliases": {}, "source_priority": {}, "favorite_metrics": [], "sleep_target_minutes": None}}
            context = {}
            runs.clear()
            relation = {}
            analysis_enabled = False
            fail_upload = False
            upload_delay = 0
            fail_review = False
        if route == 'test/seed-report':
            reports[:] = [
                {
                    'report_id': 'fixture-report',
                    'original_name': 'fixture.png',
                    'page_count': 1,
                    'revision': 1,
                    'created_at': 1790985600,
                }
            ]
            observations[:] = [
                {
                    'observation_id': 'fixture-glucose',
                    'report_id': 'fixture-report',
                    'revision': 1,
                    'status': 'confirmed',
                    'payload': {
                        'metric_id': 'glucose',
                        'raw_name': 'Glucose',
                        'raw_result': '5.5',
                        'raw_unit': 'mmol/L',
                        'sampled_at': '2026-10-01',
                        'source': {'page': 1, 'quote': 'Glucose 5.5 mmol/L'},
                    },
                }
            ]
        if route == 'test/fail-review':
            fail_review = True
        if route == 'reports/review' and fail_review:
            fail_review = False
            status = 503
        if route == 'reports/review' and status == 200:
            if body.get('context') is not None:
                context = body['context']
            for change in body.get('observations', []):
                old = next(
                    (item for item in observations if item['observation_id'] == change.get('observation_id')), None
                )
                revised = {
                    'observation_id': change.get('observation_id') or 'fixture-added',
                    'report_id': 'fixture-report',
                    'revision': (old or {}).get('revision', 0) + 1,
                    'status': change['status'],
                    'payload': change['payload'],
                }
                if old:
                    observations.remove(old)
                observations.append(revised)
            reports[0]['revision'] += 1
        if route == 'test/enable-workflows':
            analysis_enabled = True
            reports.append(
                {
                    'report_id': 'other-report',
                    'original_name': 'other.png',
                    'page_count': 1,
                    'revision': 1,
                    'created_at': 1790985601,
                }
            )
        if route == 'reports/relate':
            relation = {'kind': body.get('kind'), 'preferred_report_id': body.get('preferred_report_id')}
        if route == 'exports/delete':
            exports.clear()
        if route == 'analysis/create':
            runs[:] = [
                {
                    'run_id': 'fixture-analysis',
                    'status': 'ready',
                    'created_at': 1790985600,
                    'output': {'review': {'summary': 'Synthetic lipid review', 'findings': []}, 'evidence': []},
                    'input': {'observations': observations.copy()},
                    'feedback': [],
                }
            ]
        if route == 'analysis/feedback':
            runs[0]['feedback'].append({'feedback_id': 'fixture-feedback', 'note': body['note']})
        if route == 'test/slow-upload':
            upload_delay = 2
        if route == 'test/fail-upload':
            fail_upload = True
        if route == 'test/delete-report':
            reports.clear()
        if route == 'files/upload':
            delay, upload_delay = upload_delay, 0
            time.sleep(delay)
            upload_ids.append(self.headers.get('X-Upload-Id'))
            if fail_upload:
                fail_upload = False
                status = 503
            elif not reports:
                reports.append(
                    {
                        'report_id': 'fixture-report',
                        'original_name': 'fixture.png',
                        'page_count': 1,
                        'revision': 1,
                        'created_at': 1790985600,
                    }
                )
        if route == 'reports/get' and not reports:
            status = 404
        response = {
            'session/login': {
                'token': 'synthetic-token',
                'user': {'user_id': 'simulator-only', 'username': 'simulator'},
            },
            'user/get': {
                'user_id': 'simulator-only',
                'username': 'simulator',
                'is_admin': False,
                'must_change_password': False,
            },
            'server/status': {'capabilities': {'analysis': analysis_enabled}},
            'reports/list': {'reports': reports},
            'jobs/list': {'jobs': []},
            'metrics/list': {
                'metrics': [
                    {'metric_id': 'ldl_cholesterol', 'name': 'LDL', 'standard_unit': 'mg/dL'},
                    {'metric_id': 'hdl_cholesterol', 'name': 'HDL'},
                    {'metric_id': 'glucose', 'name': 'Glucose'},
                ]
            },
            'metrics/browse': {
                'metrics': [
                    {'metric_id': 'ldl_cholesterol', 'name': 'LDL', 'standard_unit': 'mg/dL'},
                    {'metric_id': 'clinical:calcium', 'name': 'Calcium', 'standard_unit': ''},
                ]
            },
            'metrics/history/get': {
                'points': [
                    {
                        'point_id': 'unified-' + str(index),
                        'source_name': name,
                        'source_id': name,
                        'sampled_at': '2026-10-01',
                        'timestamp': 1790812800 + index * 86400,
                        'value': str(100 + 10 * index),
                        'unit': 'mg/dL',
                        'report_id': 'fixture-report' if index else None,
                        'filename': 'fixture.png' if index else None,
                        'original': {'raw_result': str(100 + 10 * index), 'raw_unit': 'mg/dL', 'source': {'page': 1}},
                    }
                    for index, name in enumerate(['Apple Health', 'Function Health', 'Local Clinic'])
                ],
                'excluded_count': 0,
            },
            'trends/get': {
                'points': (
                    [
                        {
                            'observation_id': 'trend-' + str(index),
                            'report_id': 'fixture-report',
                            'sampled_at': date,
                            'timestamp': 1790812800 + index * 86400,
                            'value': 100 + index * 10,
                            'unit': 'mg/dL',
                            'revision': 1,
                            'original': {
                                'raw_name': 'Synthetic LDL',
                                'raw_result': str(100 + index * 10),
                                'raw_unit': 'mg/dL',
                                'source': {'page': 1},
                            },
                        }
                        for index, date in enumerate(['2026-10-01', '2026-10-02'])
                    ]
                    if analysis_enabled
                    else []
                ),
                'incomparable_count': 0,
            },
            'analysis/list': {'runs': runs},
            'health/coverage': {'coverage': [], 'total_records': 29330},
            'health/sync': {'replayed': False, 'total_records': 29330},
            'wellness/sleep/regularity': {'sources': [], 'notes': []},
            'wellness/import/list': {'imports': [], 'next_after_id': None},
            'wellness/timeline': {'events': [], 'next_cursor': None},
            'wellness/review': {'days': [], 'measurements': [], 'strength_bests': [], 'notes': []},
            'wellness/series': {'unit': 'ml/kg*min' if body.get('record_type') == 'vo2_max' else 'kg', 'sources': [{'source': 'apple_health:watch', 'points': [{'at': int(time.time()) - 365 * 86400, 'value': 42, 'record_id': 'older-vo2'}]}] if body.get('record_type') == 'vo2_max' and body.get('start_at', 0) <= int(time.time()) - 365 * 86400 < body.get('end_at', 0) else [], 'notes': []},
            'wellness/preferences/get': source_preferences,
            'wellness/day': {
                'date': body.get('date'),
                'timezone': body.get('timezone'),
                'metrics': [
                    {
                        'record_type': kind,
                        'unit': unit,
                        'current': {'state': 'ready'},
                        'history': [
                            {
                                'date': (
                                    date.fromisoformat(body.get('date', '2026-10-01')) - timedelta(days=offset)
                                ).isoformat(),
                                'alternatives': [{'source': 'apple_health:watch', 'value': value, 'unit': unit}],
                                'combined': {'value': value, 'unit': unit} if kind == 'steps' else None,
                            }
                            for offset in range(1, 10)
                        ],
                    }
                    for kind, unit, value in [('sleep', 'seconds', 27000), ('steps', 'count', 6500), ('heart_rate', 'count/min', 75), ('workout', 'seconds', 3600), ('resting_heart_rate', 'count/min', 58), ('hrv_sdnn', 'ms', 40), ('active_energy', 'kcal', 500), ('basal_energy', 'kcal', 1500)]
                ],
                'computed_at': 1790985600,
            },
            'wellness/sources': {'sources': [{'platform': 'apple_health', 'source_id': 'watch', 'record_type': 'sleep', 'record_count': 10}, {'platform': 'apple_health', 'source_id': 'watch', 'record_type': 'vo2_max', 'first_at': int(time.time()) - 400 * 86400}], 'catalog': [{'source_key': 'apple_health:watch', 'name': 'Apple Watch'}]},
            'wellness/sleep': {'sources': [], 'algorithm_version': 'synthetic'},
            'wellness/entries/list': {'entries': [{'record_id': 'manual-weight', 'at': time.time() - 60, 'version': 1, 'entry': {'kind': 'body', 'content': {'weight_kg': 72.5}}}] if body.get('kind') == 'body' else []},
            'health/aggregate': {'days': []},
            'health/connect': {'connection_id': 'synthetic-connection'},
            'exports/list': {'exports': exports},
            'files/upload': {'file': {'file_id': 'fixture-report'}},
            'reports/get': {
                'report': reports[0] if reports else {},
                'observations': observations,
                'context': context,
                'relation': relation,
                'pages': [{'page': 1, 'status': 'ready', 'content': 'Glucose 5.5 mmol/L'}],
                'extraction_inputs': [{'page': 1, 'text_status': 'available', 'run_id': 'fixture-ocr'}],
                'extraction_outputs': [{'page': 1, 'stage': 'ocr', 'run_id': 'fixture-ocr', 'model': 'synthetic'}],
            },
            'reports/review': {
                'report': reports[0] if reports else {},
                'observations': observations,
                'context': context,
                'relation': relation,
                'pages': [{'page': 1, 'status': 'ready', 'content': 'Glucose 5.5 mmol/L'}],
                'extraction_inputs': [{'page': 1, 'text_status': 'available', 'run_id': 'fixture-ocr'}],
                'extraction_outputs': [{'page': 1, 'stage': 'ocr', 'run_id': 'fixture-ocr', 'model': 'synthetic'}],
            },
            'observations/history': {'history': observations},
            'analysis/get': runs[0] if runs else {},
            'reports/input/get': {'text_layer': {'status': 'available', 'text': 'Synthetic page text', 'words': []}},
            'reports/extraction/get': {'content': 'Synthetic OCR output', 'response_body': '{"synthetic": true}'},
            'test/state': {'upload_ids': upload_ids, 'context': context},
        }.get(route, {})
        if route == 'metrics/history/get' and body.get('metric_id') == 'clinical:calcium':
            response = {
                'points': [],
                'excluded_count': 0,
                'records': [
                    {
                        'point_id': 'calcium-text',
                        'display_value': 'See laboratory note',
                        'unit': '',
                        'sampled_at': '2026-10-01',
                        'status': 'final',
                        'source_name': 'NewYork-Presbyterian',
                    }
                ],
            }
        if route == 'session/refresh':
            time.sleep(0.2)
            if refresh_failure:
                status = 503
                refresh_failure = False
            response = {
                'token': 'renewed-' + body['next_refresh_token'],
                'refresh_token': body['next_refresh_token'],
                'expires_at': time.time() + 3600,
                'refresh_after': time.time() + 1800,
                'user': {'user_id': 'simulator-only', 'username': 'simulator'},
            }
        if route == 'wellness/preferences/save':
            if body['expected_version'] != source_preferences['version']:
                status = 409
                response = {'error': 'Preferences changed'}
            else:
                source_preferences = {'version': source_preferences['version'] + 1, 'preferences': body['preferences']}
                response = source_preferences
        if route == 'exports/create':
            exports.append({'export_id': 'synthetic-export', 'status': 'ready', 'created_at': 1790985600})
        if route == 'wellness/series':
            if body.get('record_type') == 'heart_rate':
                response['unit'] = 'count/min'
                response['sources'] = [{'source': 'apple_health:watch', 'points': [{'at': datetime.now().astimezone().replace(hour=12, minute=21, second=0, microsecond=0).timestamp() + offset * 5, 'value': value, 'record_id': str(offset)} for offset, value in enumerate([70, 75, 90])]}]
            if body.get('record_type') == 'body_mass':
                response['sources'] = [{'source': 'manual:longlivehealth', 'points': [{'at': int(time.time()) - 60, 'value': 72.5, 'record_id': 'manual-weight'}]}]
            for source in response.get('sources', []):
                source['chart_points'] = source['points']
        def chart_ranges(points):
            from datetime import timedelta
            from zoneinfo import ZoneInfo
            zone = ZoneInfo(body.get('timezone', 'UTC')) if route == 'wellness/series' else timezone.utc
            def bounds(at, period, window):
                dt = datetime.fromtimestamp(at, zone)
                if period == 'H' or (period == 'D' and not window):
                    start = dt.replace(second=0, microsecond=0)
                    width = 60 if period == 'H' and not window else 3600
                    if width == 3600: start = start.replace(minute=0)
                    return start.timestamp(), start.timestamp() + width
                start = dt.replace(hour=0, minute=0, second=0, microsecond=0)
                if (period == 'D' and window) or (period in ('W', 'M') and not window):
                    end = start + timedelta(days=1)
                elif (period == 'W' and window) or (period == '6M' and not window):
                    start -= timedelta(days=(start.weekday() + 1) % 7)
                    end = start + timedelta(days=7)
                else:
                    start = start.replace(day=1)
                    months = 1
                    if period == '6M':
                        start = start.replace(month=1 if start.month <= 6 else 7); months = 6
                    if period == 'Y' and window:
                        start = start.replace(month=1); months = 12
                    n = start.year * 12 + start.month - 1 + months
                    end = start.replace(year=n // 12, month=n % 12 + 1)
                return start.timestamp(), end.timestamp()
            result = {'summaries': {}}
            for period in ('H', 'D', 'W', 'M', '6M', 'Y'):
                for window in (False, True):
                    groups = {}
                    for source, unit, at, value in points:
                        start, end = bounds(at, period, window)
                        groups.setdefault(('' if window else source, unit, start, end), []).append((at, value))
                    rows = [{'source': source, 'unit': unit, 'at': (start + end) / 2,
                             'start_at': start, 'end_at': end, 'first_at': min(p[0] for p in samples),
                             'last_at': max(p[0] for p in samples), 'latest_value': max(samples)[1],
                             'min': min(p[1] for p in samples), 'max': max(p[1] for p in samples), 'count': len(samples)}
                            for (source, unit, start, end), samples in groups.items()]
                    (result['summaries'] if window else result)[period] = rows
            return result
        if route == 'wellness/series':
            response['chart_ranges'] = chart_ranges([(s['source'], response['unit'], p['at'], p['value']) for s in response['sources'] for p in s['points']])
        if route == 'wellness/day':
            for metric in response['metrics']:
                metric['chart_ranges'] = chart_ranges([(p['source'], p['unit'], datetime.fromisoformat(day['date']).replace(tzinfo=timezone.utc).timestamp(), p['value']) for day in metric['history'] for p in day['alternatives']])
        if route == 'test/counts':
            response = request_counts
        if route == 'session/login' and body.get('username') == 'admin':
            response = {
                'token': 'admin-token',
                'user': {
                    'user_id': 'admin-test',
                    'username': 'admin',
                    'is_admin': True,
                    'must_change_password': not admin_password_changed,
                },
            }
        if route == 'user/password/change':
            admin_password_changed = True
            response = {'changed': True, 'sessions_revoked': True}
        if route == 'user/get' and self.headers.get('Authorization') == 'Bearer admin-token':
            response = {
                'user_id': 'admin-test',
                'username': 'admin',
                'is_admin': True,
                'must_change_password': not admin_password_changed,
            }
        data = json.dumps(response).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path == '/shared-report.pdf':
            objects = [
                b'<< /Type /Catalog /Pages 2 0 R >>',
                b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
                b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] /Contents 4 0 R >>',
                b'<< /Length 0 >>\nstream\n\nendstream',
            ]
            data = b'%PDF-1.4\n'
            offsets = [0]
            for index, content in enumerate(objects, 1):
                offsets.append(len(data))
                data += f'{index} 0 obj\n'.encode() + content + b'\nendobj\n'
            xref = len(data)
            data += b'xref\n0 5\n0000000000 65535 f \n'
            for offset in offsets[1:]:
                data += f'{offset:010d} 00000 n \n'.encode()
            data += f'trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF'.encode()
            self.send_response(200)
            self.send_header('Content-Type', 'application/pdf')
            self.send_header('Content-Disposition', 'inline; filename="shared-report.pdf"')
            self.send_header('Content-Length', str(len(data)))
            self.end_headers()
            self.wfile.write(data)
            return
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, 'w') as archive:
            archive.writestr('fixture.txt', 'Synthetic simulator test export')
        data = buffer.getvalue()
        if '/files/' in self.path:
            data = base64.b64decode(
                'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII='
            )
        self.send_response(200)
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)


if __name__ == '__main__':
    ThreadingHTTPServer(('127.0.0.1', 18765), Handler).serve_forever()
