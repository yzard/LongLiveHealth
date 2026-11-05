# Lifecycle shared by the combined build and standalone Android build.
start_build() {
    BUILD_PROJECT_ROOT="$1"
    mkdir -p "$BUILD_PROJECT_ROOT/build/.locks" "$BUILD_PROJECT_ROOT/build/docker"
    exec 9>"$BUILD_PROJECT_ROOT/build/.locks/docker.lock"
    python3 - <<'PY'
import fcntl
try:
    fcntl.flock(9, fcntl.LOCK_EX | fcntl.LOCK_NB)
except BlockingIOError:
    raise SystemExit('Another combined/Android build is active')
PY
    BUILD_WORK_DIR="$(mktemp -d "$BUILD_PROJECT_ROOT/build/docker/work.XXXXXX")"
    BUILD_DOCKER_BUILDER="longlivehealth-$(id -u)-$$"
    export BUILD_WORK_DIR BUILD_DOCKER_BUILDER
    export PYTHONDONTWRITEBYTECODE=1 TMPDIR="$BUILD_WORK_DIR/tmp" XDG_CACHE_HOME="$BUILD_WORK_DIR/cache"
    mkdir -p "$TMPDIR" "$XDG_CACHE_HOME"
    trap 'finish_build "$?"' EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    docker buildx create --name "$BUILD_DOCKER_BUILDER" --driver docker-container
}

finish_build() {
    local result="$1"
    trap - EXIT INT TERM
    if [ "$result" -eq 0 ]; then
        if docker buildx rm "$BUILD_DOCKER_BUILDER"; then
            python3 "$BUILD_PROJECT_ROOT/src/development/build_artifacts.py" clean \
                --project "$BUILD_PROJECT_ROOT" --work "$BUILD_WORK_DIR" || result=$?
        else
            result=$?
        fi
    fi
    if [ "$result" -ne 0 ]; then
        echo "Build or cleanup failed; diagnostics retained: $BUILD_WORK_DIR" >&2
        echo "Builder: $BUILD_DOCKER_BUILDER" >&2
    fi
    exit "$result"
}
