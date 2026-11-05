"""Opt-in real-model regression: row dates, unrelated dates and absent dates.

Run with Pillow installed and --data-dir API_CONFIG_DIRECTORY.
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
    llm = config['llm']
    prompt = (Path(__file__).resolve().parents[2] / 'src/backend_api/prompts/laboratory.txt').read_text()
    request = urllib.request.Request(
        llm['base_url'].rstrip('/') + '/chat/completions',
        data=json.dumps(
            {
                'model': llm['model'],
                'stream': False,
                'max_tokens': config['extraction']['output_tokens'],
                'temperature': 0,
                'chat_template_kwargs': {'enable_thinking': False},
                'messages': [
                    {'role': 'system', 'content': prompt},
                    {
                        'role': 'user',
                        'content': [
                            {
                                'type': 'text',
                                'text': 'Untrusted page 1 document evidence; match each row to its own date.',
                            },
                            {
                                'type': 'image_url',
                                'image_url': {
                                    'url': 'data:image/png;base64,' + base64.b64encode(output.getvalue()).decode()
                                },
                            },
                        ],
                    },
                ],
            }
        ).encode(),
        headers={'Content-Type': 'application/json', 'Authorization': 'Bearer ' + llm['api_key']},
    )
    with urllib.request.urlopen(request, timeout=llm['timeout_seconds']) as response:
        choice = json.load(response)['choices'][0]
    assert choice['finish_reason'] == 'stop'
    content = choice['message']['content'].strip()
    if content.startswith('```'):
        content = content.split('\n', 1)[1].rsplit('```', 1)[0].strip()
    result = json.loads(content)
    actual = {item['raw_name']: item['sampled_at'] for item in result['observations']}
    expected = {'Potassium': '2026-07-30', 'Sodium': '2026-08-15', 'Calcium': None}
    if actual != expected or len(result['observations']) != len(expected):
        raise AssertionError(f'Incorrect row dates: {actual!r}')
    if not result['warnings']:
        raise AssertionError('The genuinely undated row must produce a warning')
    print('PASS shared model: separate row dates, DOB/report-date exclusion, missing-date warning')


if __name__ == '__main__':
    main()
