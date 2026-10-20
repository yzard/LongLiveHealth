#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
if [ "$#" -lt 1 ] || [ "$#" -gt 2 ] || { [ "$#" = 2 ] && [ "$2" != --upload ]; }; then
    echo 'Usage: ./export_testflight.sh TEAM_ID [--upload]' >&2; exit 2
fi
TEAM_ID="$1"
ARCHIVE=build/ios/LongLiveHealth.xcarchive
APP="$ARCHIVE/Products/Applications/LongLiveHealth.app"
[ -d "$APP" ] || { echo 'Run ./archive_ios.sh --unsigned first' >&2; exit 1; }
# The unsigned preparation archive has no signed entitlements. Attach the actual
# app entitlements before Xcode re-signs it with Apple distribution credentials.
codesign --force --sign - --entitlements build/ios/LongLiveHealth.entitlements "$APP"
python3 - "$TEAM_ID" <<'PY'
import plistlib,sys
options={'method':'app-store-connect','destination':'export','teamID':sys.argv[1],
         'signingStyle':'automatic','testFlightInternalTestingOnly':True,
         'manageAppVersionAndBuildNumber':False}
plistlib.dump(options,open('build/ios/TestFlightExportOptions.plist','wb'))
options['destination']='upload'
plistlib.dump(options,open('build/ios/TestFlightUploadOptions.plist','wb'))
PY
xcodebuild -exportArchive -archivePath "$ARCHIVE" -exportPath dist/ios/TestFlight \
    -exportOptionsPlist build/ios/TestFlightExportOptions.plist -allowProvisioningUpdates
mkdir -p build/ios/export-inspection
unzip -q -o dist/ios/TestFlight/LongLiveHealth.ipa -d build/ios/export-inspection
python3 - "$TEAM_ID" <<'PY'
import plistlib,subprocess,sys
app='build/ios/export-inspection/Payload/LongLiveHealth.app'
r=subprocess.run(['codesign','-d','--entitlements',':-',app],capture_output=True,check=True)
e=plistlib.loads(r.stdout)
with open(app+'/Info.plist','rb') as f: info=plistlib.load(f)
assert info.get('UIDeviceFamily')==[1], info.get('UIDeviceFamily')
assert e.get('com.apple.developer.team-identifier')==sys.argv[1], e
assert e.get('com.apple.developer.healthkit') is True, e
assert e.get('com.apple.developer.healthkit.background-delivery') is True, e
assert 'health-records' in e.get('com.apple.developer.healthkit.access',[]), e
assert e.get('beta-reports-active') is True, e
assert e.get('get-task-allow') is False, e
print('Verified TestFlight and HealthKit distribution entitlements.')
PY
if [ "${2:-}" = --upload ]; then
    xcodebuild -exportArchive -archivePath "$ARCHIVE" -exportPath dist/ios/TestFlightUpload \
        -exportOptionsPlist build/ios/TestFlightUploadOptions.plist -allowProvisioningUpdates
fi
