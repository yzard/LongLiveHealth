"""Loopback-only replay of private operator captures; never changes their medical values/rules."""
import argparse
import importlib.util
import io
import json
from http.server import ThreadingHTTPServer
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('capture', type=Path)
parser.add_argument('--port', type=int, default=18765)
parser.add_argument('--passed-file', type=Path)
args = parser.parse_args()
manifest = json.loads((args.capture / 'manifest.json').read_text())
cases = manifest['cases']
by_kind = {c['entry']['kind']: c for c in cases}
spec = importlib.util.spec_from_file_location('fixture', Path(__file__).with_name('simulator_fixture.py'))
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)


class Handler(fixture.Handler):
    def do_POST(self):
        raw = self.rfile.read(int(self.headers.get('Content-Length', 0)))
        body = json.loads(raw or b'{}')
        route = self.path.removeprefix('/api/v1/')
        response = None
        if route == 'test/reference-cases':
            passed = json.loads(args.passed_file.read_text()) if args.passed_file and args.passed_file.exists() else []
            response = {'cases': [c for c in cases if c['entry']['kind'] not in passed], 'preferences':manifest['preferences'],'sources':manifest.get('sources',{})}
        elif route == 'test/reference-case-get':
            case = by_kind[body['kind']]
            response = json.loads((args.capture / case['file']).read_text())
        elif route == 'browse/catalog':
            response = manifest['catalog']
        elif route == 'wellness/preferences/get':
            response = manifest['preferences']
        elif route == 'wellness/sources':
            response = manifest.get('sources', {'sources': []})
        elif route == 'wellness/records/list':
            kind = 'electrocardiogram' if body.get('kind') == 'ecg' else body.get('kind')
            case = by_kind.get(kind)
            if case:
                captured = json.loads((args.capture / case['file']).read_text())
                # Replay the captured first page. Pagination is checked against the real SQL separately.
                response = {'records': captured.get('records', []) if not body.get('after') else [], 'next_cursor': None}
        elif route == 'wellness/records/get':
            for case in cases:
                captured = json.loads((args.capture / case['file']).read_text())
                detail = captured.get('latest_detail')
                if detail and detail.get('record_id') == body.get('record_id'):
                    response = detail
                    break
        elif route in ('wellness/chart', 'wellness/history', 'metrics/history/get'):
            case = by_kind.get(body.get('record_type', body.get('metric_id')))
            if case:
                response = json.loads((args.capture / case['file']).read_text())
        if response is None:
            self.rfile = io.BytesIO(raw)
            return super().do_POST()
        data = json.dumps(response, ensure_ascii=False).encode()
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)


ThreadingHTTPServer(('127.0.0.1', args.port), Handler).serve_forever()
