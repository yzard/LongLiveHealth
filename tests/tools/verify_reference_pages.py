"""Compare private Android UI audit evidence with captured production assessments."""
import argparse
import json
from datetime import datetime, timezone
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('capture', type=Path)
parser.add_argument('pages', type=Path)
parser.add_argument('--require-complete', action='store_true')
args = parser.parse_args()
manifest = json.loads((args.capture / 'manifest.json').read_text())
cases = {c['entry']['kind']: c for c in manifest['cases']}
observations = {}
for line in (args.pages / 'pages.jsonl').read_text().splitlines():
    item = json.loads(line)
    observations[item['kind']] = item
report = []


def number(n):
    return f'{float(n):,.3f}'.rstrip('0').rstrip('.')


for kind, case in cases.items():
    item = observations.get(kind)
    if not item or not item.get('passed'):
        report.append({'kind': kind, 'result': 'pending', 'reason': (item or {}).get('error')})
        continue
    result = json.loads((args.capture / case['file']).read_text())
    components = result.get('components') or [result]
    years = [datetime.fromtimestamp(row['last_at'], timezone.utc).year for component in components for row in (component.get('chart_ranges') or {}).get('Y', [])]
    year = max(years) if years else None
    candidates = [row.get('reference_assessment', {}) for component in components for row in (component.get('chart_ranges') or {}).get('reference_days', []) if datetime.fromtimestamp(row['last_at'], timezone.utc).year == year and row.get('reference_assessment', {}).get('applicable_rule')]
    errors = []
    panels = item.get('information', [])
    if candidates and not panels:
        errors.append('Eligible reference rule was not shown')
    for panel in panels:
        text = panel['text']
        scoped_components = [c for c in components if panel.get('heading') == c.get('title','') + ' · Reference Levels']
        panel_candidates = candidates
        if scoped_components:
            panel_candidates = [row.get('reference_assessment', {}) for component in scoped_components for row in (component.get('chart_ranges') or {}).get('reference_days', []) if datetime.fromtimestamp(row['last_at'], timezone.utc).year == year and row.get('reference_assessment', {}).get('applicable_rule')]
        labels = [s.split(' · ', 1)[1] for s in text if s.startswith('Latest measurement in this interval · ')]
        if not labels or not any(a.get('label') in labels for a in panel_candidates):
            errors.append('Displayed classification does not match returned assessments')
        matching = [a for a in panel_candidates if a.get('label') in labels]
        printed_panel = any(line.startswith('Source: ') for line in text)
        matching = [a for a in matching if (a.get('kind') == 'report_reference') == printed_panel]
        if not matching:
            errors.append('Reference provenance does not match any returned assessment')
        numeric_intervals = []
        for a in matching:
            rule = a['applicable_rule']
            bounds = rule.get('bands') or [rule]
            for b in bounds:
                values = [(f'{int(round(float(b[k])/60))//60} hr {int(round(float(b[k])/60))%60} min' if kind == 'sleep' and a.get('unit') in ('s','seconds') else number(b[k])) for k in ('lower', 'upper') if b.get(k) is not None]
                if values:
                    numeric_intervals.append(all(any(value in line for line in text) for value in values))
        if numeric_intervals and not any(numeric_intervals):
            errors.append('Returned numeric interval is absent from reference information')
        printed = [a for a in matching if a.get('kind') == 'report_reference']
        if printed:
            aliases = manifest['preferences']['preferences'].get('source_aliases', {})
            names = {s['source_key']:s['name'] for s in manifest.get('sources',{}).get('catalog',[])}
            expected_sources = []
            for assessment in printed:
                origin = assessment.get('reference_origin', {})
                key = origin.get('source_key','')
                expected_sources.append('Source: ' + (aliases.get(key) or names.get(key) or origin.get('name') or 'Original report reference interval'))
            if not any(source in text for source in expected_sources):
                errors.append('Displayed original-report source does not match the returned source/alias')
        else:
            titles = [s['title'] for c in (scoped_components or components) for s in (c.get('reference_guidance') or {}).get('citations', [])]
            if titles and not all(title in text for title in titles):
                errors.append('Returned citation is absent from reference information')
    report.append({'kind':kind,'result':'failed' if errors else 'verified','errors':errors,'control_issues':[p['close_workaround'] for p in panels if p.get('close_workaround')]})
summary = {state:sum(r['result']==state for r in report) for state in ('verified','failed','pending')}
(args.pages / 'verification.json').write_text(json.dumps({'summary':summary,'metrics':report},indent=2))
print(json.dumps(summary))
for row in report:
    if row['result']=='failed':
        print(json.dumps(row))

if summary["failed"] or (args.require_complete and summary["pending"]):
    raise SystemExit(1)
