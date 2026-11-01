"""Operator-directed read-only Browse audit on an authorized Android device.
Requires the separately installed, loopback-only ReadOnlyBrowseProbe instrumentation.
Private screenshots and UI text stay in the supplied output directory.
"""
import argparse
import json
import re
from datetime import datetime, timezone
import socket
import subprocess
import time
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--adb', required=True)
parser.add_argument('--serial', required=True)
parser.add_argument('--catalog', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--probe-port', type=int, default=38766)
parser.add_argument('--check-history', action='store_true')
parser.add_argument('--skip-reference', action='store_true')
parser.add_argument('--check-sessions', action='store_true')
parser.add_argument('--server-log-host')
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
catalog = json.loads(args.catalog.read_text())
entries = catalog.get('catalog', catalog)['entries']
adb = [args.adb, '-s', args.serial]


def command(*words):
    return subprocess.check_output(adb + list(words))


def probe(action, **values):
    with socket.create_connection(('127.0.0.1', args.probe_port), timeout=15) as connection:
        connection.sendall((json.dumps(dict(action=action, **values)) + '\n').encode())
        return json.loads(connection.makefile().readline())


def nodes():
    return probe('dump')['nodes']


def texts(items):
    return list(dict.fromkeys(n['text'] or n['desc'] for n in items if n['text'] or n['desc']))


def click(node):
    a, b, c, d = node['bounds']
    command('shell', 'input', 'tap', str((a+c)//2), str((b+d)//2))


def find(items, label):
    height = items[0]['bounds'][3] if items else 0
    return next((n for n in items if label in (n['text'], n['desc']) and n['bounds'][1] >= 0 and 0 < n['bounds'][3] <= height and n['bounds'][2] > n['bounds'][0]), None)


def tap(label):
    for _ in range(20):
        if probe('click', text=label).get('ok'):
            return
        match = find(nodes(), label)
        if match is not None:
            click(match)
            return
        time.sleep(.1)
    raise AssertionError('Missing control: ' + label)


def scroll(up):
    items = nodes()
    width = items[0]['bounds'][2]
    height = items[0]['bounds'][3]
    top, bottom = int(height*.35), int(height*.78)
    command('shell', 'input', 'swipe', str(width//2), str(bottom if up else top), str(width//2), str(top if up else bottom), '220')
    time.sleep(.25)


def back():
    for _ in range(24):
        if find(nodes(),'Back'):
            tap('Back');time.sleep(.5)
            return
        scroll(False)
    raise AssertionError('Back control is not reachable')


for _ in range(24):
    if find(nodes(), 'Done'):
        tap('Done');time.sleep(.6);continue
    if any('EditText' in n['class'] for n in nodes()):
        break
    if find(nodes(), 'Back'):
        tap('Back')
    elif find(nodes(), 'Browse'):
        tap('Browse');scroll(False)
    time.sleep(.4)
log = args.output / 'pages.jsonl'
completed = {json.loads(line)['kind'] for line in log.read_text().splitlines() if json.loads(line).get('passed')} if log.exists() else set()
for index, entry in enumerate(entries):
    if entry['kind'] in completed:
        continue
    result = {'kind': entry['kind'], 'title': entry['title'], 'index': index, 'passed': False}
    try:
        assert probe('search', text=entry['kind']).get('ok')
        for _ in range(30):
            match = next((n for n in nodes() if 'EditText' not in n['class'] and entry['title'] in (n['text'],n['desc'])), None)
            if match:
                break
            time.sleep(.2)
        assert match, 'Metric is absent from search results'
        click(match)
        time.sleep(.5)
        assert find(nodes(),'Back'), 'Metric detail did not open'
        for _ in range(120):
            current = texts(nodes())
            if not any(t.startswith('Loading') for t in current):
                break
            time.sleep(.25)
        else:
            raise AssertionError('Metric did not finish loading')
        if find(nodes(), 'Y'):
            tap('Y'); time.sleep(.7)
            for _ in range(100):
                if not any(t.startswith('Loading') for t in texts(nodes())): break
                time.sleep(.2)
            time.sleep(.3)
        seen, information, visited = [], [], set()
        for page in range(9):
            current = nodes()
            seen.extend(texts(current))
            for info in ([] if args.skip_reference else [n for n in current if (n['desc'] == 'About Reference Levels' or n['text'] == 'About Reference Levels') and n['bounds'][1] >= 0 and 0 < n['bounds'][3] <= current[0]['bounds'][3]]):
                heading = next((n['text'] for n in reversed(current[:current.index(info)]) if 'Reference Levels' in n['text']), 'Reference Levels')
                if heading in visited:
                    continue
                opened_at = datetime.now(timezone.utc).isoformat()
                assert probe('click', text='About Reference Levels', after=heading).get('ok'), 'Reference information control is not accessible'
                for _ in range(40):
                    page_text = texts(nodes())
                    if 'Reference Levels' in page_text and 'About Reference Levels' not in page_text: break
                    time.sleep(.15)
                modal = texts(nodes())
                assert 'Reference Levels' in modal and 'About Reference Levels' not in modal, 'Reference information did not open'
                information.append({'heading':heading,'text':modal})
                back()
                for _ in range(30):
                    if find(nodes(),'About Reference Levels'): break
                    time.sleep(.15)
                assert find(nodes(),'About Reference Levels'), 'Reference Back did not restore the chart'
                if args.server_log_host:
                    time.sleep(3)
                    log_output = subprocess.run(['ssh','-o','ClearAllForwardings=yes',args.server_log_host,'docker','logs','--since',opened_at,'longlivehealth'],capture_output=True,text=True,check=True)
                    plain = re.sub(r'\x1b\[[0-9;]*m','',log_output.stdout+log_output.stderr)
                    routes = re.findall(r'route="/api/v1/([^" ]+)"',plain)
                    reads = [route for route in routes if route in ('wellness/chart','wellness/history','metrics/history/get')]
                    information[-1]['return_requests'] = reads
                    assert not reads, 'Reference navigation reloaded the underlying metric: '+str(reads)
                visited.add(heading)
            raw = command('exec-out', 'screencap', '-p')
            offset = raw.find(b'\x89PNG')
            assert offset >= 0, 'No screenshot'
            (args.output / f'{index}-{page}.png').write_bytes(raw[offset:])
            if find(current,'About this metric') or find(current,'ABOUT THIS METRIC'):
                break
            scroll(True)
        seen = list(dict.fromkeys(seen))
        errors = [t for t in seen if any(x in t.lower() for x in ('could not complete', 'request failed', 'unauthorized', 'timed out', 'could not be loaded', 'outstanding read data'))]
        assert not errors, str(errors)
        if args.check_history and entry['presentation'] != 'records':
            for _ in range(16):
                try:
                    tap('Historical Values')
                    break
                except AssertionError:
                    scroll(True)
            else:
                raise AssertionError('Historical Values is not reachable')
            observed_dates = []
            for _ in range(100):
                if any(re.search(r'\b\d{4}-\d{2}-\d{2} (?:\d{2}:\d{2} [AP]M|—)',t) for t in texts(nodes())): break
                time.sleep(.2)
            for _ in range(3):
                visible = texts(nodes())
                for text in visible:
                    for value in re.findall(r'\b\d{4}-\d{2}-\d{2} (?:\d{2}:\d{2} [AP]M|—)',text):
                        if value not in observed_dates: observed_dates.append(value)
                scroll(True)
            assert observed_dates, 'No dated historical rows were displayed'
            dates = [datetime.strptime(v,'%Y-%m-%d %I:%M %p') if not v.endswith('—') else datetime.strptime(v[:10],'%Y-%m-%d') for v in observed_dates]
            assert dates == sorted(dates,reverse=True), 'Historical values are not newest first'
            result['historical_dates']=observed_dates
            back()
        if args.check_sessions and entry['kind'] == 'sleep':
            for _ in range(16):
                if find(nodes(),'Sleep Sessions'): break
                scroll(True)
            tap('Sleep Sessions')
            for _ in range(150):
                if any(re.search(r'\b\d{4}-\d{2}-\d{2} \d{2}:\d{2} [AP]M',t) for t in texts(nodes())): break
                time.sleep(.2)
            dates=[]
            first=None
            for _ in range(3):
                for text in texts(nodes()):
                    for value in re.findall(r'\b\d{4}-\d{2}-\d{2} \d{2}:\d{2} [AP]M',text):
                        if first is None: first=value
                        if value not in dates: dates.append(value)
                scroll(True)
            assert dates, 'Sleep sessions did not load'
            times=[datetime.strptime(v,'%Y-%m-%d %I:%M %p') for v in dates]
            assert times==sorted(times,reverse=True), 'Sleep sessions are not globally newest first'
            result['session_dates']=dates
            for _ in range(16):
                if find(nodes(),first): break
                scroll(False)
            tap(first);time.sleep(.5)
            for _ in range(3):
                assert nodes(), 'Sleep stage expansion lost the app window'
                scroll(True)
            back()
        result.update(text=seen, information=information)
        (args.output / f'{index}.json').write_text(json.dumps(result, indent=2, ensure_ascii=False))
        back()
        for _ in range(20):
            if any('EditText' in n['class'] for n in nodes()): break
            time.sleep(.15)
        assert any('EditText' in n['class'] for n in nodes()), 'Could not return to Browse'
        result['passed']=True
    except Exception as error:
        result['error'] = type(error).__name__ + ': ' + str(error)
    with log.open('a') as file:
        file.write(json.dumps(result, ensure_ascii=False) + '\n')
    print(f"{index+1}/{len(entries)} {entry['kind']} {'PASS' if result['passed'] else result.get('error')}", flush=True)
    if not result['passed']:
        raise SystemExit(1)
