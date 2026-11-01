"""Every public business route must have a maintained mobile caller; no legacy aliases."""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
ROUTES = ROOT / 'src/backend_api/routes.rs'
source = ROUTES.read_text()
routes = set(re.findall(r'"(/(?:api/v1/[^" ]+|longlivehealth.apk|downloads/longlivehealth.apk))"', source))
callers = {}
files = [p for root in ('src/shared', 'src/ios', 'src/android/Swift') for p in (ROOT / root).rglob('*.swift')]
for file in files:
    for match in re.finditer(r'\b(?:request|post|action|download)\(\s*"([^"\\]+)"', file.read_text()):
        path = match[1]
        if '/' in path:
            callers.setdefault('/api/v1/' + path, set()).add(str(file.relative_to(ROOT)))
# These paths interpolate the identifier or one of the three supported track formats.
dynamic = {
    '/api/v1/files/{file_id}/download': ('src/shared/Application/AppModel.swift', 'files/\\(id)/download'),
    '/api/v1/exports/{export_id}/download': ('src/shared/UI/ExportSection.swift', 'exports/\\(id)/download'),
    **{'/api/v1/wellness/import/' + kind: ('src/shared/UI/TrainingImportsView.swift', 'wellness/import/\\(file.format)') for kind in ('gpx', 'tcx', 'fit')},
}
for route, (file, literal) in dynamic.items():
    assert literal in (ROOT / file).read_text(), f'Missing dynamic caller: {route}'
    callers[route] = {file}
# The initial Android install is explicitly served to a browser before the app is installed.
callers['/longlivehealth.apk'] = {'Initial Android installation download'}
assert not routes - callers.keys(), f'Routes without mobile callers: {sorted(routes - callers.keys())}'
assert not callers.keys() - routes, f'Mobile calls without server routes: {sorted(callers.keys() - routes)}'
print(f'Public API inventory passed: {len(routes)} routes, each with an explicit caller or installation purpose')
if __name__ == '__main__' and '--write-inventory' in __import__('sys').argv:
    target = ROOT / 'docs/reviews/2026-10-31-mobile-api-inventory.md'
    target.write_text('# Mobile API inventory\n\nBoth clients compile the shared application and UI sources below. Platform health bridges use the same connect/sync/coverage endpoints. Downloads are GET; business operations are POST. Internal calculation functions are not additional API routes.\n\n| Route | Caller |\n| --- | --- |\n' + ''.join('| `' + route + '` | ' + ', '.join(sorted(callers[route])) + ' |\n' for route in sorted(routes)))
