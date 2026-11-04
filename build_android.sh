#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODE="${1:-build}"
[[ $# -le 1 && ( "$MODE" == build || "$MODE" == prepare ) ]] || { echo 'Usage: build_android.sh [build|prepare]' >&2; exit 2; }
if [[ "$MODE" == build ]]; then
    if [[ -z "${BUILD_WORK_DIR:-}" ]]; then
        source "$ROOT/src/development/build_lifecycle.sh"
        start_build "$ROOT"
    fi
    WORK="$BUILD_WORK_DIR/android/client"
else
    WORK="$ROOT/build/android/client"
fi
mkdir -p "$WORK/Sources/HealthApp" "$WORK/Android" "$ROOT/dist/android"
# Compilation and dependency caches belong to this invocation, not the live emulator workspace.
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
TOOLCHAIN_INPUT="$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$ROOT/docker/android.Dockerfile")"
TOOLCHAIN_CACHED="$(docker image inspect longlivehealth-android-toolchain:6.3.2 --format '{{index .Config.Labels "org.longlivehealth.toolchain-input"}}' 2>/dev/null || :)"
if [[ "$TOOLCHAIN_CACHED" != "$TOOLCHAIN_INPUT" ]]; then
    docker buildx build --builder "$BUILD_DOCKER_BUILDER" --load -f "$ROOT/docker/android.Dockerfile" \
        --label "org.longlivehealth.toolchain-input=$TOOLCHAIN_INPUT" -t longlivehealth-android-toolchain:6.3.2 "$ROOT"
fi
mkdir -p "$BUILD_WORK_DIR/android/gradle"
# The named signing volume preserves the development identity across clean workspaces.
docker run --rm -i -v "$WORK:/workspace" -v "$BUILD_WORK_DIR/android/gradle:/root/.gradle" \
    -v longlivehealth-android-signing:/root/.android -v "$ROOT/src/tools:/checks:ro" \
    -e BUILD_UID="$(id -u)" -e BUILD_GID="$(id -g)" \
    -w /workspace longlivehealth-android-toolchain:6.3.2 bash -seu <<'ANDROID_BUILD'
trap 'chown -R "$BUILD_UID:$BUILD_GID" /workspace /root/.gradle' EXIT
skip export --no-ios --debug --debug-info line-tables --host-build transpile --module HealthApp --swift /usr/bin/swift --arch aarch64
# Build ARM64 explicitly rather than reusing the Docker host architecture.
mapfile -t modules < <(find .build/plugins/outputs -maxdepth 6 -path "*/skipstone/HealthApp/build.gradle.kts")
[[ ${#modules[@]} == 1 ]] || { echo "Expected one generated HealthApp Gradle module" >&2; exit 1; }
native="$(dirname "${modules[0]}")/build"
SKIP_BRIDGE=1 TARGET_OS_ANDROID=1 skip android build --dir "$native/jni-libs" --package-path /workspace --configuration debug --product HealthApp --scratch-path "$native/swift" --arch aarch64 -Xcc -fPIC -Xswiftc -DSKIP_BRIDGE -Xswiftc -DTARGET_OS_ANDROID
cd Android
SKIP_BRIDGE_ANDROID_BUILD_DISABLED=1 gradle --no-daemon :app:testDebugUnitTest :app:lintDebug :app:assembleDebug
python3 /checks/check_android_package.py /workspace/.build/Android/app/outputs/apk/debug/app-debug.apk --dexdump /opt/android-sdk/build-tools/36.0.0/dexdump
ANDROID_BUILD
APK="$WORK/.build/Android/app/outputs/apk/debug/app-debug.apk"
[[ -s "$APK" ]] || { echo 'Android build did not produce an APK.' >&2; exit 1; }
cp "$APK" "$ROOT/dist/android/longlivehealth.apk.new"
chmod 644 "$ROOT/dist/android/longlivehealth.apk.new"
mv "$ROOT/dist/android/longlivehealth.apk.new" "$ROOT/dist/android/longlivehealth.apk"
echo "Android package: $ROOT/dist/android/longlivehealth.apk"

REPORT_ROOT="$ROOT/build/validation/${BUILD_WORK_DIR##*/}/android"
for report in reports test-results; do
    if [[ -d "$WORK/.build/Android/app/$report" ]]; then
        mkdir -p "$REPORT_ROOT"
        cp -a "$WORK/.build/Android/app/$report" "$REPORT_ROOT/"
    fi
done
