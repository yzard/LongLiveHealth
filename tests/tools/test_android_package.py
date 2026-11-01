import pathlib
import hashlib
import json
import sys
import unittest
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2] / 'src/tools'))
from check_android_package import required_methods, validate_package


class AndroidPackageTests(unittest.TestCase):
    def test_category_icons_cover_the_server_catalog_with_original_raster_assets(self):
        root = pathlib.Path(__file__).resolve().parents[2]
        resources = root / 'src/android/Swift/Resources'
        manifest = json.loads((resources / 'MaterialIcons/provenance.json').read_text())
        categories = json.loads((root / 'src/backend_api/browse_categories.json').read_text())['categories']
        self.assertEqual(manifest['license'], 'Apache-2.0')
        self.assertTrue((resources / 'MaterialIcons/LICENSE.txt').is_file())
        self.assertEqual({row['symbol'] for row in manifest['icons']}, {row['symbol'] for row in categories})
        for row in manifest['icons']:
            asset = resources / 'HealthCategoryIcons.xcassets' / (row['asset'] + '.imageset')
            contents = json.loads((asset / 'Contents.json').read_text())
            data = (asset / contents['images'][0]['filename']).read_bytes()
            self.assertTrue(data.startswith(b'\x89PNG\r\n\x1a\n'))
            self.assertEqual(hashlib.sha256(data).hexdigest(), row['sha256'])
            self.assertIn('/' + manifest['revision'] + '/png/', row['url'])

    def test_stale_native_library_is_rejected_against_actual_dex_declarations(self):
        methods = required_methods('''<api><package name="health.app"><class name="BrowseView"><method name="Swift_initState_healthDefinitions" native="true"><parameter type="long"/></method><method name="Evaluate" native="false"/></class></package></api>''')
        self.assertEqual(methods, [('Java_health_app_BrowseView_Swift_1initState_1healthDefinitions', 'Java_health_app_BrowseView_Swift_1initState_1healthDefinitions__J')])
        with self.assertRaisesRegex(ValueError, 'missing JNI exports.*healthDefinitions'):
            validate_package({'arm64-v8a'}, methods, {'Java_health_app_BrowseView_Swift_1release'})
        validate_package({'arm64-v8a'}, methods, {methods[0][0]})
        validate_package({'arm64-v8a'}, methods, {methods[0][1]})

    def test_only_arm64_and_nonempty_declarations_are_allowed(self):
        for abis in [set(), {'x86_64'}, {'arm64-v8a', 'x86_64'}]:
            with self.assertRaises(ValueError):
                validate_package(abis, [('method', 'method__J')], {'method'})
        with self.assertRaises(ValueError):
            validate_package({'arm64-v8a'}, [], set())

    def test_nested_classes_arrays_and_overloads_use_jni_encoding(self):
        methods = required_methods('''<api><package name="health.app"><class name="Model$Companion"><method name="load_items" native="true"><parameter type="java.lang.String[]"/></method></class></package></api>''')
        self.assertEqual(methods[0][1], 'Java_health_app_Model_00024Companion_load_1items___3Ljava_lang_String_2')


if __name__ == '__main__':
    unittest.main()
