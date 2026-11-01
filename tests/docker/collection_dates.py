"""Opt-in real-model regression: row dates, unrelated dates and absent dates.

Run with the OCR virtualenv, --url SERVICE_URL --data-dir OCR_CONFIG_DIRECTORY.
Only synthetic laboratory data is sent. The service credential stays in TOML.
"""

import argparse
import base64
import io
import json
import tomllib
import urllib.request
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--url', required=True)
    parser.add_argument('--data-dir', type=Path, required=True)
    args = parser.parse_args()
    config = tomllib.loads((args.data_dir / 'config.toml').read_text())
    image = Image.new('RGB', (1400, 1000), 'white')
    draw = ImageDraw.Draw(image)
    font = ImageFont.load_default(size=30)
    for position, text in [
        ((50, 40), 'Example Laboratory'),
        ((50, 100), 'DOB: Apr 23, 1984'),
        ((50, 150), 'Report generated: Oct 30, 2026'),
        ((50, 300), 'Potassium'),
        ((50, 350), '4.5 mmol/L'),
        ((1050, 300), 'Jul 30, 2026'),
        ((50, 500), 'Sodium'),
        ((50, 550), '139 mmol/L'),
        ((1050, 500), 'Aug 15, 2026'),
        ((50, 700), 'Calcium'),
        ((50, 750), '9.5 mg/dL'),
    ]:
        draw.text(position, text, fill='black', font=font)
    for y in [420, 620, 820]:
        draw.line((50, y, 1350, y), fill='gray', width=2)
    output = io.BytesIO()
    image.save(output, format='PNG')
    request = urllib.request.Request(
        args.url.rstrip('/') + '/api/v1/documents/extract',
        data=json.dumps(
            {
                'page': 1,
                'image_url': 'data:image/png;base64,' + base64.b64encode(output.getvalue()).decode(),
                'text_layer': None,
            }
        ).encode(),
        headers={
            'Content-Type': 'application/json',
            'Authorization': 'Bearer ' + config['server']['api_key'],
        },
    )
    with urllib.request.urlopen(request, timeout=660) as response:
        result = json.load(response)
    actual = {item['raw_name']: item['sampled_at'] for item in result['observations']}
    expected = {'Potassium': '2026-07-30', 'Sodium': '2026-08-15', 'Calcium': None}
    if actual != expected or len(result['observations']) != len(expected):
        raise AssertionError(f'Incorrect row dates: {actual!r}')
    if not result['warnings']:
        raise AssertionError('The genuinely undated row must produce a warning')
    print(f"PASS {result['prompt_version']}: separate row dates, DOB/report-date exclusion, missing-date warning")


if __name__ == '__main__':
    main()
