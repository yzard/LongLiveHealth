"""Synthetic API fixture for iOS UI tests; never uses real patient data."""

import base64
import io
import json
import time
import zipfile
from pathlib import Path
_parents = Path(__file__).resolve().parents
_catalog = (_parents[2] / "src/backend_api/apple_catalog.json") if len(_parents) > 2 else Path("/tmp/apple_catalog.json")
if not _catalog.exists():
    _catalog = Path("/tmp/apple_catalog.json")
apple_catalog = json.loads(_catalog.read_text())
apple_definitions = {d["kind"]: d for d in apple_catalog}
from datetime import datetime, timezone, date, timedelta
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

exports = []
reports = []
upload_ids = []
fail_upload = False
upload_delay = 0
fail_review = False
report_status_chart = False
observations = []
context = {}
relation = {}
request_counts = {}
admin_password_changed = False
refresh_failure = False
fail_preferences = False
source_preferences = {"version": int(time.time() * 1000), "preferences": {"reference_profile": {"birth_date": None, "reference_sex": None}, "source_aliases": {}, "source_priority": {}, "sleep_target_minutes": None}}


def report_pdf(page_count):
    page_ids = ' '.join(f'{3 + index * 2} 0 R' for index in range(page_count))
    objects = [
        b'<< /Type /Catalog /Pages 2 0 R >>',
        f'<< /Type /Pages /Kids [{page_ids}] /Count {page_count} >>'.encode(),
    ]
    for index in range(page_count):
        objects.append(
            f'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] /Contents {4 + index * 2} 0 R >>'.encode()
        )
        drawing = f'0.1 0.4 {0.2 + index * 0.12:.2f} rg 30 {60 + index * 35} 240 45 re f'.encode()
        objects.append(f'<< /Length {len(drawing)} >>\nstream\n'.encode() + drawing + b'\nendstream')
    data = bytearray(b'%PDF-1.4\n')
    offsets = []
    for index, content in enumerate(objects, 1):
        offsets.append(len(data))
        data.extend(f'{index} 0 obj\n'.encode() + content + b'\nendobj\n')
    xref = len(data)
    size = len(objects) + 1
    data.extend(f'xref\n0 {size}\n0000000000 65535 f \n'.encode())
    for offset in offsets:
        data.extend(f'{offset:010d} 00000 n \n'.encode())
    data.extend(f'trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF'.encode())
    return data


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_POST(self):
        global fail_upload, upload_delay, fail_review, context, relation, admin_password_changed, refresh_failure, source_preferences, fail_preferences, report_status_chart
        length = int(self.headers.get('Content-Length', 0))
        raw = self.rfile.read(length)
        body = json.loads(raw) if self.headers.get('Content-Type') == 'application/json' else {}
        route = self.path.removeprefix('/api/v1/')
        request_counts[route] = request_counts.get(route, 0) + 1
        status = 200
        created_metric = None
        if route == 'health/sync' and self.headers.get('Authorization') == 'Bearer reject-sync':
            status = 503
        if route == 'test/unauthorized':
            status = 401
        if route == 'test/fail-refresh':
            refresh_failure = True
        if route == 'test/fail-preferences-once':
            fail_preferences = True
        if route == 'test/reset-counts':
            request_counts.clear()
        if route == 'test/reset':
            request_counts.clear()
            admin_password_changed = False
            refresh_failure = False
            fail_preferences = False
            exports.clear()
            reports.clear()
            upload_ids.clear()
            observations.clear()
            source_preferences = {"version": int(time.time() * 1000), "preferences": {"reference_profile": {"birth_date": None, "reference_sex": None}, "source_aliases": {}, "source_priority": {}, "sleep_target_minutes": None}}
            context = {}
            relation = {}
            fail_upload = False
            upload_delay = 0
            fail_review = False
            report_status_chart = False
        if route == 'test/enable-report-status-chart':
            report_status_chart = True
        if route in ('test/seed-report', 'test/seed-multipage-report'):
            multipage = route == 'test/seed-multipage-report'
            report_id = 'fixture-multipage' if multipage else 'fixture-report'
            reports[:] = [
                {
                    'report_id': report_id,
                    'original_name': 'fixture.pdf' if multipage else 'fixture.png',
                    'context': {'source_name': 'Example Laboratory'},
                    'collection_date': '2026-10-01',
                    'page_count': 4 if multipage else 1,
                    'revision': 1,
                    'created_at': 1790985600,
                }
            ]
            observations[:] = [
                {
                    'observation_id': 'fixture-glucose',
                    'interpretation': {'result': {'display': '99.1 mg/dL'}, 'reference': {'reason': 'No reference interval was printed'}},
                    'report_id': report_id,
                    'revision': 1,
                    'status': 'confirmed',
                    'payload': {
                        'metric_id': 'glucose',
                        'raw_name': 'Glucose',
                        'raw_result': '5.5',
                        'raw_unit': 'mmol/L',
                        'sampled_at': '2026-10-01',
                        'source': {'page': 2 if multipage else 1, 'quote': 'Glucose 5.5 mmol/L'},
                    },
                }
            ]
        if route == 'test/report-test-code':
            row = observations[0]
            row['display_name'] = 'Thyroid-stimulating hormone'
            row['lab_test_code'] = '11580-8'
            row['lab_test_name'] = 'Thyrotropin [Units/volume] in Serum or Plasma'
            row['payload']['raw_name'] = 'TSH (11580-8)'
        if route == 'test/report-urine-mapping':
            row = observations[0]
            row['display_name'] = 'Appearance - urine'
            row['lab_test_code'] = '5767-9'
            row['resolved_metric_id'] = 'urine_appearance'
            row['interpretation'] = {}
            row['payload'].update(metric_id=None, raw_name='Appearance (5767-9)', raw_result='Clear',
                                  raw_unit=None, source={'page': 1, 'quote': 'Appearance (5767-9) Clear'})
        if route == 'test/report-issue':
            observations[0]['browse_issues'] = [{'code': 'unsupported_unit', 'scope': 'browse', 'message': 'Cannot appear in Browse: unit invalid-unit is not supported for this test.'}]
        if route in ('test/report-metric-suggestions', 'test/report-no-metric-suggestions', 'test/report-negative-suggestion'):
            row = observations[0]
            row['status'] = 'pending'
            row['payload']['metric_id'] = None
            row['browse_issues'] = [{
                'code': 'unmapped_metric', 'scope': 'browse',
                'message': 'No matching Browse metric. Review this result.',
            }]
            row['metric_suggestions'] = [] if route == 'test/report-no-metric-suggestions' else [{
                'metric_id': 'glucose', 'name': 'Blood glucose', 'standard_unit': 'mg/dL',
                'unit_match': 'convertible',
            }]
            if route == 'test/report-negative-suggestion':
                row['payload'].update(raw_name='Appearance', raw_result='Negative', raw_unit='mg/dL')
                row['interpretation'] = {}
                row['metric_suggestions'] = [{
                    'metric_id': 'urine_appearance', 'name': 'Appearance - urine',
                    'standard_unit': '1', 'unit_match': 'incompatible',
                }, {
                    'metric_id': 'glucose', 'name': 'Blood glucose',
                    'standard_unit': 'mg/dL', 'unit_match': 'exact',
                }]
                row['browse_issues'] += [
                    {'code': 'unsupported_unit', 'scope': 'browse', 'message': 'Unit mismatch · mg/dL'},
                    {'code': 'negative_with_unit', 'scope': 'review', 'message': 'Verify the assay threshold.'},
                ]
            if route == 'test/report-no-metric-suggestions':
                row['payload']['raw_name'] = 'Zyphorin activity'
                row['payload']['ocr_new_metric'] = {'name': 'Zyphorin panel activity', 'standard_unit': 'mmol/L'}
                row['interpretation'] = {}
        if route == 'test/expand-report':
            template = observations[0]
            observations[:] = [
                dict(template, observation_id=f'fixture-marker-{index}', status='pending',
                     payload=dict(template['payload'], raw_name=f'Marker {index + 1}'))
                for index in range(126)
            ]
            if body.get('needs_review'):
                for row in observations[1:]:
                    row['browse_issues'] = [{'code': 'unsupported_unit', 'scope': 'review',
                                             'message': 'Check the reported unit.'}]
        if route == 'test/mark-last-result-uncertain':
            row = observations[-1]
            row['status'] = 'pending'
            row['payload'].update(raw_result='Negative', raw_unit='mg/dL')
            row['reported_negative_bound'] = '<0.1 mg/dL'
            row['browse_issues'] = [{'code': 'negative_with_unit', 'scope': 'review',
                                     'message': 'Reported negative threshold: <0.1 mg/dL.'}]
        if route == 'reports/delete':
            reports[:] = [item for item in reports if item['report_id'] != body.get('report_id')]
            observations[:] = [item for item in observations if item['report_id'] != body.get('report_id')]
        if route == 'test/fail-review':
            fail_review = True
        if route == 'reports/review' and fail_review:
            fail_review = False
            status = 503
        if route == 'reports/review' and status == 200:
            if body.get('new_metric'):
                created_metric = dict(body['new_metric'], metric_id='custom:fixture')
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
                if created_metric:
                    revised['payload']['metric_id'] = created_metric['metric_id']
                    revised['created_metric'] = created_metric
                if old:
                    observations.remove(old)
                observations.append(revised)
            reports[0]['revision'] += 1
        if route == 'test/seed-related-report':
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
            'server/status': {'status': 'ok'},
            'reports/list': {'reports': reports},
            'jobs/list': {'jobs': []},
            'metrics/list': {
                'metrics': [
                    {'metric_id': 'ldl_cholesterol', 'name': 'LDL', 'standard_unit': 'mg/dL'},
                    {'metric_id': 'hdl_cholesterol', 'name': 'HDL'},
                    {'metric_id': 'glucose', 'name': 'Glucose'},
                    {'metric_id': 'urine_appearance', 'name': 'Appearance - urine', 'standard_unit': '1'},
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
            'insights/get': {
                'computed_at': int(time.time()),
                'cardiovascular': {
                    'title':'Cardiovascular Review', 'risk_label':'10-year total CVD risk', 'risk_percent':None,
                    'explanation':'Risk needs confirmed diagnoses and medicines.', 'inputs':[], 'findings':[], 'cards':[],
                    'missing':['Confirm: Current smoking'], 'actions':[],
                    'method':'Synthetic assessment method for UI verification.',
                    'references':[{'title':'AHA PREVENT', 'url':'https://professional.heart.org/en/guidelines-and-statements/about-prevent-calculator'}]
                },
                'training': {
                    'title':'Training & Recovery', 'status':'More comparable data needed', 'actions':[],
                    'missing':['Comparable HRV baseline from the same source'], 'fitness':[],
                    'method':'Synthetic recovery method for UI verification.', 'references':[],
                    'signals':[{'id':device,'metric_id':'hrv_sdnn','title':'HRV SDNN','source':device,'source_name':device,'unit':'ms',
                        'baseline':50,'recent':40,'baseline_days':28,'recent_days':7,'change_percent':-20,'cue':'lower_hrv',
                        'series':[{'title':'HRV','points':[{'at':1793000000,'value':50},{'at':1793100000,'value':40}]}]}
                        for device in ['First Watch','Second Watch']]
                }
            },
            'health/coverage': {'coverage': [{'record_type':'heart_rate','status':'observed','last_success_at':1790985600}, {'record_type':'workout_route','status':'error'}, {'record_type':'sleep','status':'no_visible_samples'}], 'total_records': 29330},
            'health/sync': {'replayed': False, 'total_records': 29330},
            'wellness/sleep/regularity': {'sources': [], 'notes': []},
            'wellness/import/list': {'imports': [], 'next_after_id': None},
            'wellness/timeline': {'events': [], 'next_cursor': None},
            'wellness/review': {'days': [], 'measurements': [], 'strength_bests': [], 'notes': []},
            'wellness/preferences/get': source_preferences,
            'wellness/sources': {'sources': [{'platform': 'apple_health', 'source_id': 'watch', 'record_type': 'sleep', 'record_count': 10}, {'platform': 'apple_health', 'source_id': 'watch', 'record_type': 'vo2_max', 'first_at': int(time.time()) - 400 * 86400}], 'catalog': [{'source_key': 'apple_health:watch', 'name': 'Apple Watch'}]},
            'wellness/sleep': {'sources': [], 'algorithm_version': 'synthetic'},
            'wellness/entries/list': {'entries': [{'record_id': 'manual-weight', 'at': time.time() - 60, 'version': 1, 'entry': {'kind': 'body', 'content': {'weight_kg': 72.5}}}] if body.get('kind') == 'body' else []},
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
                'created_metric': created_metric,
                'report': reports[0] if reports else {},
                'observations': observations,
                'context': context,
                'relation': relation,
                'pages': [{'page': 1, 'status': 'ready', 'content': 'Glucose 5.5 mmol/L'}],
                'extraction_inputs': [{'page': 1, 'text_status': 'available', 'run_id': 'fixture-ocr'}],
                'extraction_outputs': [{'page': 1, 'stage': 'ocr', 'run_id': 'fixture-ocr', 'model': 'synthetic'}],
            },
            'observations/history': {'history': observations},
            'reports/input/get': {'text_layer': {'status': 'available', 'text': 'Synthetic page text', 'words': []}},
            'reports/extraction/get': {'content': 'Synthetic OCR output', 'response_body': '{"synthetic": true}'},
            'test/state': {'upload_ids': upload_ids, 'context': context},
        }.get(route, {})
        if route == 'metrics/history/get' and report_status_chart and body.get('metric_id') == 'ldl_cholesterol':
            response = {'points': [
                {
                    'point_id': row['observation_id'], 'report_id': row['report_id'],
                    'source_id': 'fixture-report', 'source_name': 'Example Laboratory',
                    'timestamp': int(time.time()), 'value': '99.1', 'unit': 'mg/dL',
                }
                for row in observations if row['status'] == 'confirmed'
            ], 'excluded_count': 0}
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
            fail_preferences = False
            response = {
                'token': 'renewed-' + body['next_refresh_token'],
                'refresh_token': body['next_refresh_token'],
                'expires_at': time.time() + 3600,
                'refresh_after': time.time() + 1800,
                'user': {'user_id': 'simulator-only', 'username': 'simulator'},
            }
        if route == 'wellness/preferences/save':
            if fail_preferences:
                fail_preferences = False
                status = 409
                response = {'error': 'Synthetic preferences conflict'}
            elif body['expected_version'] != source_preferences['version']:
                status = 409
                response = {'error': 'Preferences changed'}
            else:
                source_preferences = {'version': source_preferences['version'] + 1, 'preferences': body['preferences']}
                response = source_preferences
        if route == 'exports/create':
            exports.append({'export_id': 'synthetic-export', 'status': 'ready', 'created_at': 1790985600})
        if route == 'wellness/records/list':
            record_id = body['kind'] if body['kind'].startswith('HKCharacteristic') else 'event-1'
            response = {'records': [{'platform':'apple_health','source_id':'watch','record_id':record_id,'record_type':body['kind'],'at':int(time.time())-60,'version':1}], 'next_cursor':None}
        if route == 'wellness/records/get':
            response = {'record_type':'HKCategoryTypeIdentifierAppleStandHour','start_at':int(time.time())-60,'details':{'category':'Stood'},'notes':[]}
            if body['record_id'].startswith('HKCharacteristic'):
                details = {'value': 2}
                if body['record_id'].endswith('DateOfBirth'):
                    details = {'date_of_birth': {'NS.year':1980, 'NS.month':2, 'NS.day':3}}
                response = {'record_type':body['record_id'], 'start_at':int(time.time())-60, 'details':details, 'notes':[]}
        def chart_ranges(points):
            from datetime import timedelta
            from zoneinfo import ZoneInfo
            zone = ZoneInfo(body.get('timezone', 'UTC'))
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
        if route == 'wellness/records/list':
            record_id = body['kind'] if body['kind'].startswith('HKCharacteristic') else 'event-1'
            response = {'records': [{'platform':'apple_health','source_id':'watch','record_id':record_id,'record_type':body['kind'],'at':int(time.time())-60,'version':1}], 'next_cursor':None}
        if route == 'wellness/records/get':
            response = {'record_type':'HKCategoryTypeIdentifierAppleStandHour','start_at':int(time.time())-60,'details':{'category':'Stood'},'notes':[]}
            if body['record_id'].startswith('HKCharacteristic'):
                details = {'value': 2}
                if body['record_id'].endswith('DateOfBirth'):
                    details = {'date_of_birth': {'NS.year':1980, 'NS.month':2, 'NS.day':3}}
                response = {'record_type':body['record_id'], 'start_at':int(time.time())-60, 'details':details, 'notes':[]}
        if route == 'metrics/history/get':
            response['chart_ranges'] = chart_ranges([(p.get('source_key', p.get('source_id', '')), p['unit'], p['timestamp'], float(p['value'])) for p in response.get('points', [])])
        if route in ('wellness/chart', 'wellness/measurements'):
            kind = body['record_type']
            unit = {'heart_rate':'count/min','respiratory_rate':'count/min','body_mass':'kg','vo2_max':'ml/kg*min','blood_glucose':'mg/dL','body_fat':'%','oxygen_saturation':'%','HKQuantityTypeIdentifierHeartRateVariabilityRMSSD':'ms'}.get(kind, apple_definitions.get(kind, {}).get('unit', 'count'))
            value = 72.5 if kind == 'body_mass' else 42 if kind == 'vo2_max' else 70
            source = 'manual:longlivehealth' if kind == 'body_mass' else 'apple_health:watch'
            now = int(datetime.now().astimezone().replace(hour=12, minute=21, second=10, microsecond=0).timestamp()) if kind == 'heart_rate' else int(time.time()) - 60
            all_points = [(source,unit,at,value+n) for n,at in enumerate([now,now-400*86400,1577836800])]
            if kind == 'heart_rate':
                all_points += [(source,unit,now-5,90)]
            points = [p for p in all_points if body['start_at'] <= p[2] < body['end_at']]
            response = {'chart_ranges':chart_ranges(points),'unit':unit,'sources':[source], 'first_at':1577836800,'last_at':now,'samples_read':len(points),'valid_samples':len(points),'excluded_samples':0,'complete':True,'data_revision':1}
            if route == 'wellness/measurements':
                response = {'points':[{'at':p[2],'value':p[3],'source':p[0],'record_id':str(p[2]),'unit':unit} for p in points],'next_offset':None,'data_revision':1}
        if route == 'wellness/chart' and body['record_type'] == 'HKCorrelationTypeIdentifierBloodPressure':
            now = int(time.time()) - 60
            source = 'apple_health:watch'
            components = []
            for name, value in [('Systolic', 120), ('Diastolic', 80)]:
                components.append({'id':'blood_pressure_'+name.lower(), 'title':name, 'unit':'mmHg', 'chart_ranges':chart_ranges([(source,'mmHg',now,value)])})
            response = {'components':components, 'sources':[source], 'first_at':now, 'last_at':now, 'valid_samples':2, 'complete':True, 'latest_reading':{'at':now,'source':source,'values':{c['id']:v for c,v in zip(components,[120,80])}}}
        if route == 'insights/get':
            now = int(time.time())
            signal = response['training']['signals'][0]
            evidence = dict(signal, latest=58, latest_at=now-86400, baseline_low=54, baseline_high=65)
            response['dashboard'] = [dict(id=key, title=title, symbol=symbol, summary='Synthetic evidence for layout verification.',
                method='Synthetic method', references=[], evidence=[dict(evidence, metric_id=metric, title=title, unit=unit)])
                for key,title,symbol,metric,unit in [
                    ('sleep','Sleep','bed.double.fill','sleep','seconds'),
                    ('readiness','Readiness','sparkles','hrv_sdnn','ms'),
                    ('vitals','Vitals','waveform.path.ecg','resting_heart_rate','count/min'),
                    ('heart','Heart','heart.fill','heart_rate','count/min'),
                    ('fitness','Fitness','figure.walk','active_energy','kcal'),
                    ('cardiovascular','Cardiovascular Review','heart.fill','apob','mg/dL')]]
            response['dashboard'][0].update(goal_minutes=420, source='First Watch', session={
                'start_at':now-28800, 'end_at':now, 'asleep_seconds':27000,
                'timeline':[{'start_at':now-28800+i*3600,'end_at':now-25200+i*3600,'category':[3,4,3,5,3,4,5,2][i]} for i in range(8)]})
        if route == 'wellness/history':
            kind = body['record_type']
            unit, value = {'sleep': ('seconds', 27000), 'workout': ('seconds', 3600), 'resting_heart_rate': ('count/min', 58), 'hrv_sdnn': ('ms', 40), 'active_energy': ('kcal', 500), 'basal_energy': ('kcal', 1500), 'steps': ('count', 6500)}[kind]
            window = body.get('dates')
            comparing_trends = window and (date.fromisoformat(window[1]) - date.fromisoformat(window[0])).days == 34
            dates = ['2020-01-01'] + [(date.today()-timedelta(days=n)).isoformat() for n in range(35 if comparing_trends else 9, 0, -1)]
            all_dates = dates[:]
            if body.get('dates'): dates = [d for d in dates if body['dates'][0] <= d <= body['dates'][1]]
            history = [{'date': d, 'alternatives': [{'source': 'apple_health:watch', 'value': value, 'unit': unit}]} for d in dates]
            response = {'record_type': kind, 'unit': unit, 'history': history, 'first_record_at':1577836800, 'last_record_at':int(datetime.fromisoformat(all_dates[-1]).replace(tzinfo=timezone.utc).timestamp())}
            response['chart_ranges'] = chart_ranges([('apple_health:watch', unit, datetime.fromisoformat(d).replace(tzinfo=timezone.utc).timestamp(), value) for d in dates])
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
        if route in ('metrics/history/get', 'wellness/chart', 'wellness/history') and isinstance(response, dict) and 'chart_ranges' in response:
            response['reference_guidance'] = {'version': 'fixture', 'unit': 'catalog-unit', 'reason': '', 'note': 'Synthetic interval for presentation tests.', 'rules': [{'id':'fixture-rule', 'min_age':18, 'lower':80, 'upper':130, 'lower_inclusive':True, 'upper_inclusive':True, 'bands':[], 'context':{}}], 'citations':[{'id':'fixture-reference','title':'Synthetic reference source','url':'https://example.com/reference'}]}
            for rows in response['chart_ranges'].values():
                if not isinstance(rows, list): continue
                for row in rows:
                    row['reference_assessment'] = {'status':'in_range','label':'In range','kind':'reference_interval','lower':80,'upper':130,'unit':row['unit'],'applicable_rule':response['reference_guidance']['rules'][0]}
            response['chart_ranges']['reference_days'] = [dict(row) for row in response['chart_ranges'].get('W', [])]
        if route == 'wellness/chart' and body.get('record_type') == 'respiratory_rate':
            response['chart_ranges']['reference_days'] = []
            response['reference_guidance'] = {'reason': 'No applicable interval in this fixture.', 'citations': []}
        if route == 'browse/catalog':
            from pathlib import Path
            taxonomy = json.loads((Path(__file__).resolve().parents[2] / 'src/backend_api/browse_categories.json').read_text())
            entries = []
            for d in apple_catalog:
                if not d['browse'] or d['kind'] in ('blood_pressure_systolic', 'blood_pressure_diastolic'): continue
                d = dict(d)
                kind = d['kind']
                title = taxonomy['titles'].get(kind, d['title'])
                d['title'] = title
                text = ''.join(c for c in (kind + title).lower() if c.isalnum())
                category = next((r['category'] for r in taxonomy['rules'] if any(t in text for t in r['terms'])), 'other')
                presentation = 'sleep' if kind == 'sleep' else 'daily' if kind in ('steps','active_energy','basal_energy','resting_heart_rate','hrv_sdnn','workout') else 'measurement' if d['family'] in ('quantity','correlation') else 'records'
                entries.append(dict(id='health:'+kind,kind=kind,title=title,category_id=category,presentation=presentation,definition=d))
            for kind,title in [('ldl_cholesterol','LDL'),('clinical:calcium','Calcium')]:
                entries.append(dict(id='lab:'+kind,kind=kind,title=title,category_id='bloodWork',presentation='lab',definition=dict(metric_id=kind,name=title)))
            entries.sort(key=lambda e: e['title'].lower())
            response = dict(categories=[c for c in taxonomy['categories'] if any(e['category_id']==c['id'] for e in entries)],entries=entries)
        if route == 'wellness/records/list' and body['kind'] == 'HKCorrelationTypeIdentifierBloodPressure':
            response = {'records':[{'platform':'apple_health','source_id':'watch','record_id':'pressure-pair','record_type':body['kind'],'at':1793451600,'version':1,'values':[{'id':'blood_pressure_systolic','value':120,'unit':'mmHg'},{'id':'blood_pressure_diastolic','value':80,'unit':'mmHg'}]}],'next_cursor':None}
        if route == 'wellness/records/list' and body['kind'] == 'ecg':
            response = {'records':[{'platform':'apple_health','source_id':'watch','record_id':'ecg-'+str(i),'record_type':'electrocardiogram','at':int(time.time())-i*86400,'version':1} for i in range(2)],'next_cursor':None}
        if route == 'wellness/records/get' and body['record_id'].startswith('ecg-'):
            import math
            offset = int(body.get('sample_offset', 0))
            count = 12000
            response = {'record_type':'electrocardiogram','version':1,'start_at':int(time.time())-int(body['record_id'].split('-')[-1])*86400,'sampling_frequency_hz':500,'sample_count':count,'samples':[{'time_since_start':i/500,'lead_i_volts':math.sin(i/15)/1000} for i in range(offset,min(count,offset+5000))],'next_sample_offset':offset+5000 if offset+5000<count else None,'source_classification':1,'details':{'ecg_symptoms_status':1},'notes':[]}
        data = json.dumps(response).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path in ('/shared-report.pdf', '/api/v1/files/fixture-multipage/download'):
            data = report_pdf(1 if self.path == '/shared-report.pdf' else 4)
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
                'iVBORw0KGgoAAAANSUhEUgAAAEAAAABgCAIAAAAip+O/AAAAfklEQVR4nO3PMQ0AMAzAsPInvYLYYVWKESTzjhsd8KsBrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQGtAa0BrQFtAeMWvCrZgh7FAAAAAElFTkSuQmCC'
            )
        self.send_response(200)
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)


if __name__ == '__main__':
    ThreadingHTTPServer(('127.0.0.1', 18765), Handler).serve_forever()
