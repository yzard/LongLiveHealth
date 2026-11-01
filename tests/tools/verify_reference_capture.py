"""Validate captured real Browse payloads, independently checking each numeric assessment."""
import argparse
import json
import math
from collections import Counter
from datetime import date, datetime, timezone
from pathlib import Path


def contains(value, bound):
    lo, hi = bound.get('lower'), bound.get('upper')
    return (lo is None or value > float(lo) or (value == float(lo) and bound.get('lower_inclusive', True))) and (hi is None or value < float(hi) or (value == float(hi) and bound.get('upper_inclusive', True)))


def verify(folder):
    manifest = json.loads((folder / 'manifest.json').read_text())
    profile = manifest['preferences']['preferences']['reference_profile']
    report = []
    failures = []
    for case in manifest['cases']:
        entry = case['entry']
        result = json.loads((folder / case['file']).read_text())
        states = Counter()
        reasons = set()
        sources = set()
        if result.get('capture_error'):
            failures.append((entry['kind'], result['capture_error']))
        for component in (result.get('components') or [result]):
            guidance = component.get('reference_guidance', {})
            sources.update(s['url'] for s in guidance.get('citations', []))
            for period, rows in component.get('chart_ranges', {}).items():
                if not isinstance(rows, list):
                    continue
                for row in rows:
                    assessment = row.get('reference_assessment', {})
                    status = assessment.get('status', 'missing_assessment')
                    states[status] += 1
                    if assessment.get('reason'):
                        reasons.add(assessment['reason'])
                    if status == 'missing_assessment':
                        failures.append((entry['kind'], f'{period}: numeric range has no assessment'))
                    if status == 'not_assessed':
                        if not assessment.get('reason'):
                            failures.append((entry['kind'], 'Unassessed without reason'))
                        continue
                    rule = assessment.get('applicable_rule', {})
                    value = row.get('mean') if period == 'reference_days' and entry['kind'] == 'vo2_max' else row.get('latest_value')
                    if value is None or not math.isfinite(float(value)) or not rule:
                        failures.append((entry['kind'], 'Assessment without value/rule'))
                        continue
                    value = float(value)
                    if rule.get('kind') == 'report_reference':
                        origin = assessment.get('reference_origin', {})
                        if origin.get('source_key') != row['source']:
                            failures.append((entry['kind'], 'Printed range is attached to the wrong source'))
                    if rule.get('kind') != 'report_reference':
                        born = date.fromisoformat(profile['birth_date'])
                        measured = datetime.fromtimestamp(row['last_at'], timezone.utc).date()
                        age = measured.year - born.year - ((measured.month, measured.day) < (born.month, born.day))
                        if age < rule.get('min_age', 0) or (rule.get('max_age_exclusive') is not None and age >= rule['max_age_exclusive']):
                            failures.append((entry['kind'], 'Incorrect age population'))
                        if rule.get('sex') is not None and rule['sex'] != profile.get('reference_sex'):
                            failures.append((entry['kind'], 'Incorrect reference sex population'))
                        if assessment.get('age_at_measurement') != age:
                            failures.append((entry['kind'], 'Incorrect age at measurement'))
                    if rule.get('bands'):
                        matches = [b for b in rule['bands'] if contains(value, b)]
                        if len(matches) != 1 or matches[0]['status'] != status or matches[0]['label'] != assessment.get('label'):
                            failures.append((entry['kind'], f'{period}: incorrect band classification'))
                    else:
                        expected = 'in_range' if contains(value, rule) else ('low' if rule.get('lower') is not None and value <= float(rule['lower']) else 'high')
                        if status != expected:
                            failures.append((entry['kind'], f'{period}: incorrect interval classification'))
        report.append({'kind': entry['kind'], 'title': entry['title'], 'presentation': entry['presentation'], 'states': dict(states), 'reasons': sorted(reasons), 'sources': sorted(sources)})
    output = {'entries': len(report), 'failures': failures, 'metrics': report}
    (folder / 'verification.json').write_text(json.dumps(output, indent=2, ensure_ascii=False))
    print(json.dumps({'entries': len(report), 'failures': failures}, ensure_ascii=False))
    return output


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('folder', type=Path)
    args = parser.parse_args()
    raise SystemExit(bool(verify(args.folder)['failures']))
