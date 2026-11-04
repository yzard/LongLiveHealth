import fcntl
import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('build_artifacts', ROOT / 'src/development/build_artifacts.py')
artifacts = importlib.util.module_from_spec(spec)
spec.loader.exec_module(artifacts)


def model_layout(layout):
    directory = layout / 'blobs/sha256'
    directory.mkdir(parents=True)

    def blob(value):
        data = value if isinstance(value, bytes) else json.dumps(value).encode()
        digest = hashlib.sha256(data).hexdigest()
        (directory / digest).write_bytes(data)
        return {'digest': 'sha256:' + digest, 'size': len(data)}

    layers = [blob(b'model'), blob(b'license')]
    config = blob(
        {
            'config': {'Labels': {'org.shared-model.sha256': artifacts.MODEL_SHA256}},
            'rootfs': {'diff_ids': [row['digest'] for row in layers]},
        }
    )
    manifest = blob({'config': config, 'layers': layers})
    (layout / 'index.json').write_text(json.dumps({'manifests': [manifest]}))
    (layout / 'oci-layout').write_text(json.dumps({'imageLayoutVersion': '1.0.0'}))
    return layers


class BuildArtifactTests(unittest.TestCase):
    def test_existing_model_is_reused_without_any_docker_download_call(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            layout = root / 'shared/model'
            layers = model_layout(layout)
            artifacts.validate_model(layout)
            script = '''set -euo pipefail
source "$1"
BUILD_PROJECT_ROOT="$2"
BUILD_DOCKER_BUILDER=unused
docker() { echo Unexpected-model-download >&2; return 99; }
prepare_shared_model "$3"
'''
            args = [
                'bash',
                '-c',
                script,
                'model-test',
                str(ROOT / 'src/development/build_lifecycle.sh'),
                str(ROOT),
                str(layout.parent),
            ]
            valid = subprocess.run(args, capture_output=True, text=True)
            self.assertEqual(valid.returncode, 0, valid.stderr)
            self.assertIn('Using local shared', valid.stdout)
            (layout / 'blobs/sha256' / layers[0]['digest'].split(':')[1]).unlink()
            invalid = subprocess.run(args, capture_output=True, text=True)
            self.assertNotEqual(invalid.returncode, 0)
            self.assertNotIn('Unexpected-model-download', invalid.stderr)

    def test_cleanup_removes_private_work_and_targets_preserving_active_android_artifacts_and_data(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            work = root / 'build/docker/work.test'
            (work / 'android/gradle').mkdir(parents=True)
            target = root / 'build/backend_api/target'
            (target / 'debug').mkdir(parents=True)
            (target / 'CACHEDIR.TAG').write_text('Signature: 8a477f597d28d172789f06886806bc55\n')
            protected = [
                'build/android/client/artifact.apk',
                'dist/android/app.apk',
                'build/backups/archive',
                'playground/backend_api/database.sqlite',
            ]
            for relative in protected:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'preserve')
            artifacts.clean(root, work)
            self.assertFalse(work.exists())
            self.assertFalse(target.exists())
            self.assertTrue(all((root / name).read_bytes() == b'preserve' for name in protected))

    def test_active_target_and_symlink_fail_before_deleting_diagnostics(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            work = root / 'build/docker/work.test'
            work.mkdir(parents=True)
            target = root / 'build/backend_api/target'
            (target / 'debug').mkdir(parents=True)
            (target / 'CACHEDIR.TAG').write_text('Signature: 8a477f597d28d172789f06886806bc55\n')
            with (target / 'debug/.cargo-lock').open('w+b') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                with self.assertRaises(BlockingIOError):
                    artifacts.clean(root, work)
            self.assertTrue(work.exists())
            shutil.rmtree(target)
            target.symlink_to(root / 'playground')
            with self.assertRaises(ValueError):
                artifacts.clean(root, work)
            self.assertTrue(work.exists())

    def test_real_exit_trap_cleans_on_success_retains_failure_and_propagates_cleanup_error(self):
        for failure, expected in [('', 0), ('build', 7), ('cleanup', 9)]:
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary).resolve()
                (root / 'src/development').mkdir(parents=True)
                shutil.copy(ROOT / 'src/development/build_artifacts.py', root / 'src/development/build_artifacts.py')
                shutil.copy(ROOT / 'src/development/android_cleanup.py', root / 'src/development/android_cleanup.py')
                binary = root / 'bin'
                binary.mkdir()
                docker = binary / 'docker'
                docker.write_text(
                    '#!/bin/bash\nif [[ "$1 $2" == "buildx rm" && "$TEST_FAILURE" == cleanup ]]; then exit 9; fi\n'
                )
                docker.chmod(0o755)
                script = '''set -euo pipefail
source "$1"
start_build "$2"
echo compiled > "$BUILD_WORK_DIR/scratch"
mkdir -p "$2/dist"
echo artifact > "$2/dist/program"
[[ "$TEST_FAILURE" != build ]] || exit 7
'''
                env = dict(os.environ, PATH=str(binary) + os.pathsep + os.environ['PATH'], TEST_FAILURE=failure)
                result = subprocess.run(
                    ['bash', '-c', script, 'build-test', str(ROOT / 'src/development/build_lifecycle.sh'), str(root)],
                    env=env,
                    capture_output=True,
                    text=True,
                )
                self.assertEqual(result.returncode, expected, result.stderr)
                self.assertEqual(bool(list((root / 'build/docker').glob('work.*'))), bool(failure))
                self.assertEqual((root / 'dist/program').read_text().strip(), 'artifact')


if __name__ == '__main__':
    unittest.main()
