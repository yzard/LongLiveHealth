import json
import pathlib
import re
import unittest
import subprocess
import sys
ROOT = pathlib.Path(__file__).resolve().parents[2]

class AppleCatalogCoverage(unittest.TestCase):
    def test_every_synced_type_has_an_explicit_browse_policy(self):
        catalog = json.loads((ROOT/'src/backend_api/apple_catalog.json').read_text())
        kinds = [d['kind'] for d in catalog]
        self.assertEqual(len(kinds),len(set(kinds)))
        source=(ROOT/'src/ios/HealthCatalog.swift').read_text()
        synced=set(re.findall(r'kind: "([^"]+)"',source))
        snapshots=(ROOT/'src/ios/HealthSnapshots.swift').read_text()
        synced.update(re.findall(r'case "([^"]+)":',snapshots))
        synced.update('HKCharacteristicTypeIdentifier'+v for v in ['BiologicalSex','BloodType','DateOfBirth','FitzpatrickSkinType','WheelchairUse','ActivityMoveMode'])
        self.assertFalse(synced-set(kinds),synced-set(kinds))
        for d in catalog:
            self.assertTrue(d['title'])
            self.assertTrue(d['description'])
            if d['family']=='quantity': self.assertTrue(d['unit'])
        blocked=[d['kind'] for d in catalog if not d['browse']]
        self.assertEqual(blocked,['HKClinicalTypeIdentifierCoverageRecord'])

    @unittest.skipUnless(sys.platform == 'darwin', 'Requires the installed Apple SDK')
    def test_installed_sdk_sample_identifiers_are_in_sync_catalog(self):
        sdk=pathlib.Path(subprocess.check_output(['xcrun','--sdk','iphoneos','--show-sdk-path'],text=True).strip())
        header=(sdk/'System/Library/Frameworks/HealthKit.framework/Headers/HKTypeIdentifiers.h').read_text()
        identifiers=set(re.findall(r'const (HK(?:Quantity|Category|Correlation|Clinical)TypeIdentifier\w+)\s',header))
        source=(ROOT/'src/ios/HealthCatalog.swift').read_text()
        synced=set(re.findall(r'identifier: "([^"]+)"',source))
        self.assertFalse(identifiers-synced, identifiers-synced)

if __name__ == '__main__': unittest.main()
