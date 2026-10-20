#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
SIGNING=()
case "${1:-}" in
    --unsigned) SIGNING=(CODE_SIGNING_ALLOWED=NO) ;;
    --team)
        [ -n "${2:-}" ] || { echo 'Missing Apple Developer Program team ID' >&2; exit 2; }
        SIGNING=("DEVELOPMENT_TEAM=$2" -allowProvisioningUpdates) ;;
    *) echo 'Usage: ./archive_ios.sh --team TEAM_ID | --unsigned' >&2; exit 2 ;;
esac
mkdir -p build/ios
xcodegen generate --spec src/ios/project.yml --project build/ios
xcodebuild -project build/ios/LongLiveHealth.xcodeproj -scheme LongLiveHealth \
    -configuration Release -destination 'generic/platform=iOS' \
    -archivePath build/ios/LongLiveHealth.xcarchive \
    -derivedDataPath build/ios/ArchiveDerivedData \
    "${SIGNING[@]}" archive
printf '%s\n' 'Archive: build/ios/LongLiveHealth.xcarchive'
if [ "$1" = --unsigned ]; then
    printf '%s\n' 'Unsigned preparation only. Use ./export_testflight.sh TEAM_ID to apply distribution signing.'
else
    printf '%s\n' 'Open this archive in Xcode Organizer to validate and distribute to App Store Connect.'
fi
