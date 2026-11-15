#!/usr/bin/env bash
set -euo pipefail
if [ "$#" -gt 1 ]; then
    echo 'Usage: ./build_docker.sh [test-tag]' >&2
    exit 1
fi
PROJECT_ROOT="$(cd "$(dirname "$0")" && pwd)"
IMAGE_REPOSITORY=longlivehealth-backend-api
if [ "$#" -eq 0 ]; then
    RELEASE_DATE="$(date +%Y%m%d)"
    IMAGE_TAGS=(--tag "$IMAGE_REPOSITORY:$RELEASE_DATE" --tag "$IMAGE_REPOSITORY:latest")
else
    TEST_TAG="$1"
    if [[ ! "$TEST_TAG" =~ ^[a-zA-Z0-9_][a-zA-Z0-9_.-]{0,127}$ || "$TEST_TAG" == latest || "$TEST_TAG" =~ ^[0-9]{8}$ ]]; then
        echo 'Use a test tag other than latest or YYYYMMDD' >&2
        exit 2
    fi
    IMAGE_TAGS=(--tag "$IMAGE_REPOSITORY:$TEST_TAG")
fi
source "$PROJECT_ROOT/src/development/build_lifecycle.sh"
start_build "$PROJECT_ROOT"
cd "$PROJECT_ROOT"
python3 "$PROJECT_ROOT/tests/tools/test_apple_catalog.py"
python3 "$PROJECT_ROOT/tests/tools/test_mobile_api_contract.py"
python3 "$PROJECT_ROOT/tests/development/playground.py"
python3 -m unittest discover -s "$PROJECT_ROOT/tests/development" -p '*.py'
python3 "$PROJECT_ROOT/tests/run_playground.py"
python3 "$PROJECT_ROOT/tests/build_ios.py"
node --test "$PROJECT_ROOT/tests/frontend/"*.mjs
node --check "$PROJECT_ROOT/src/frontend/app.mjs"
node --check "$PROJECT_ROOT/src/frontend/presentation.mjs"
"$PROJECT_ROOT/build_android.sh"
docker buildx build --builder "$BUILD_DOCKER_BUILDER" --load --file docker/backend_api.Dockerfile "${IMAGE_TAGS[@]}" .
