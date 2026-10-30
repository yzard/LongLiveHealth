#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODE="${1:-build}"
[[ $# -le 1 && ( "$MODE" == build || "$MODE" == prepare ) ]] || { echo 'Usage: build_android.sh [build|prepare]' >&2; exit 2; }
WORK="$ROOT/build/android/client"
mkdir -p "$WORK/Sources/HealthApp" "$WORK/Android" "$ROOT/dist/android"
# Only generated source directories are replaced; dependency and Gradle caches survive.
rsync -a --delete "$ROOT/src/android/Swift/" "$WORK/Sources/HealthApp/"
for directory in Core Application UI; do
    mkdir -p "$WORK/Sources/HealthApp/$directory"
    rsync -a --delete "$ROOT/src/shared/$directory/" "$WORK/Sources/HealthApp/$directory/"
done
rsync -a --delete "$ROOT/src/android/app/" "$WORK/Android/app/"
mkdir -p "$WORK/Android/app/src/androidTest/kotlin"
rsync -a --delete "$ROOT/tests/android/instrumentation/" "$WORK/Android/app/src/androidTest/kotlin/"
rsync -a --delete "$ROOT/src/android/gradle/" "$WORK/Android/gradle/"
mkdir -p "$WORK/Android/app/src/main/res/drawable"
cp "$ROOT/src/ios/Assets.xcassets/AppIcon.appiconset/AppIcon.png" "$WORK/Android/app/src/main/res/drawable/app_icon.png"
cp "$ROOT/src/android/Package.swift" "$ROOT/src/android/Skip.env" "$WORK/"
cp "$ROOT/src/android/settings.gradle.kts" "$ROOT/src/android/gradle.properties" "$WORK/Android/"
if [[ -f "$ROOT/src/android/Package.resolved" ]]; then
    cp "$ROOT/src/android/Package.resolved" "$WORK/Package.resolved.new"
    mv -f "$WORK/Package.resolved.new" "$WORK/Package.resolved"
fi
[[ "$MODE" == prepare ]] && exit 0
python3 "$ROOT/tests/tools/test_android_package.py"
command -v docker >/dev/null || { echo 'Docker is required for Android compilation.' >&2; exit 1; }
docker build -f "$ROOT/docker/android.Dockerfile" -t longlivehealth-android-toolchain:6.3.2 "$ROOT"
# The named signing volume preserves the development identity across clean workspaces.
docker run --rm -v "$WORK:/workspace" -v longlivehealth-android-gradle:/root/.gradle \
    -v longlivehealth-android-signing:/root/.android -v "$ROOT/src/tools:/checks:ro" \
    -w /workspace longlivehealth-android-toolchain:6.3.2 \
    bash -euc 'skip export --no-ios --debug --debug-info line-tables --host-build transpile --module HealthApp --swift /usr/bin/swift --arch aarch64
      # Gradle defaults to --arch automatic (the Docker host), which can leave a
      # stale ARM64 library alongside freshly transpiled Kotlin. Build ARM64 explicitly.
      mapfile -t modules < <(find .build/plugins/outputs -maxdepth 6 -path "*/skipstone/HealthApp/build.gradle.kts")
      [[ ${#modules[@]} == 1 ]] || { echo "Expected one generated HealthApp Gradle module" >&2; exit 1; }
      native="$(dirname "${modules[0]}")/build"
      SKIP_BRIDGE=1 TARGET_OS_ANDROID=1 skip android build --dir "$native/jni-libs" --package-path /workspace --configuration debug --product HealthApp --scratch-path "$native/swift" --arch aarch64 -Xcc -fPIC -Xswiftc -DSKIP_BRIDGE -Xswiftc -DTARGET_OS_ANDROID
      cd Android
      SKIP_BRIDGE_ANDROID_BUILD_DISABLED=1 gradle --no-daemon :app:testDebugUnitTest :app:lintDebug :app:assembleDebug
      python3 /checks/check_android_package.py /workspace/.build/Android/app/outputs/apk/debug/app-debug.apk --dexdump /opt/android-sdk/build-tools/36.0.0/dexdump'
APK="$WORK/.build/Android/app/outputs/apk/debug/app-debug.apk"
[[ -s "$APK" ]] || { echo 'Android build did not produce an APK.' >&2; exit 1; }
cp "$APK" "$ROOT/dist/android/longlivehealth.apk.new"
chmod 644 "$ROOT/dist/android/longlivehealth.apk.new"
mv "$ROOT/dist/android/longlivehealth.apk.new" "$ROOT/dist/android/longlivehealth.apk"
echo "Android package: $ROOT/dist/android/longlivehealth.apk"
