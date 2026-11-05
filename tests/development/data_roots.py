import importlib.util
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('data_roots', ROOT / 'src/development/data_roots.py')
data_roots = importlib.util.module_from_spec(spec)
spec.loader.exec_module(data_roots)


class ConfigurationTests(unittest.TestCase):
    def test_one_private_api_config_preserves_operator_credentials(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'docker').mkdir()
            (root / 'docker/config.toml').write_text((ROOT / 'docker/config.toml').read_text())
            self.assertTrue(data_roots.needs_update(root))
            data_roots.prepare(root)
            path = root / 'playground/backend_api/config.toml'
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            self.assertFalse((root / 'playground/backend_ocr').exists())
            text = path.read_text().replace('api_key = ""', 'api_key = "operator-configured-private-key"')
            path.write_text(text)
            data_roots.prepare(root)
            self.assertEqual(path.read_text(), text)
            self.assertFalse(data_roots.needs_update(root))

    def test_obsolete_schema_is_rejected_without_replacing_it(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / 'playground/backend_api/config.toml'
            path.parent.mkdir(parents=True)
            old = '[ocr]\napi_key="retained-private-key"\n'
            path.write_text(old)
            with self.assertRaises(ValueError):
                data_roots.prepare(root)
            self.assertEqual(path.read_text(), old)
