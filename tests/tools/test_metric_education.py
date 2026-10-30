"""Coverage against the API catalog prevents a new Browse item silently losing its explanation."""
import csv
import json
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]


class MetricEducationCoverage(unittest.TestCase):
    def test_all_backend_definitions_and_ios_kinds_have_education(self):
        with (ROOT / 'docs/metrics/education.tsv').open() as stream:
            rows = list(csv.DictReader(stream, delimiter='|'))
        ids = {row['id'] for row in rows}
        base = (ROOT / 'src/backend_api/laboratory.rs').read_text().split('pub fn metrics()')[1].split('.into_iter()')[0]
        extra = (ROOT / 'src/backend_api/laboratory_names.rs').read_text().split('pub(crate) const EXISTING_ALIASES')[0]
        defined = set(re.findall(r'\(\s*"([a-z0-9_]+)",\s*"', base + extra))
        measurements = (ROOT / 'src/shared/UI/MetricMeasurementsView.swift').read_text().split('static let kinds = ')[1].split('\n')[0]
        defined.update(re.findall(r'\("([A-Za-z0-9_]+)"', measurements))
        defined.update(['sleep', 'sleep_sessions', 'workout', 'steps', 'active_energy', 'basal_energy', 'resting_heart_rate', 'hrv_sdnn'])
        self.assertFalse(defined - ids, defined - ids)
        refs = json.loads((ROOT / 'docs/metrics/education-references.json').read_text())
        for row in rows:
            self.assertIn(row['reference'], refs)
            self.assertGreater(len(row['description'].split()), 15, row['id'])
        aliases = json.loads((ROOT / 'docs/metrics/education-aliases.json').read_text())
        for name in json.loads((ROOT / 'docs/metrics/audited-names.json').read_text()):
            key = ''.join(c for c in name.lower() if c.isalnum() or c in '%/')
            self.assertIn(key, aliases, name)
            self.assertIn(aliases[key], ids, name)


if __name__ == '__main__':
    unittest.main()
