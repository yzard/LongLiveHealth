#!/usr/bin/env bash
set -euo pipefail
if [ "$#" -gt 1 ]; then
    echo 'Usage: ./build_docker.sh [tag]' >&2
    exit 1
fi
PROJECT_ROOT="$(cd "$(dirname "$0")" && pwd)"
IMAGE_TAG="${1:-local}"
if [[ ! "$IMAGE_TAG" =~ ^[a-zA-Z0-9_][a-zA-Z0-9_.-]{0,127}$ ]]; then
    echo 'Invalid Docker tag' >&2
    exit 2
fi
MODEL_CACHE_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/shared-model-layers/qwen3.8-27b-nvfp4-ninfer-f0b43ad4"
source "$PROJECT_ROOT/src/development/build_lifecycle.sh"
start_build "$PROJECT_ROOT"
cd "$PROJECT_ROOT"
prepare_shared_model "$MODEL_CACHE_ROOT"
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
docker buildx build --builder "$BUILD_DOCKER_BUILDER" --load --file docker/backend_api.Dockerfile --tag "longlivehealth-backend-api:$IMAGE_TAG" .
docker buildx build --builder "$BUILD_DOCKER_BUILDER" --build-context "model=oci-layout://$SHARED_MODEL_LAYOUT" --load --file docker/backend_ocr.Dockerfile --tag "longlivehealth-backend-ocr:$IMAGE_TAG" .
