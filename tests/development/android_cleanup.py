import fcntl
import hashlib
import importlib.util
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('android_cleanup_source', ROOT / 'src/development/android_cleanup.py')
cleanup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cleanup)


def fixture(root):
    cache = root / 'build/android'
    artifacts = root / 'dist/android/emulator'
    artifacts.parent.mkdir(parents=True)
    (artifacts.parent / 'longlivehealth.apk').write_bytes(b'current exported APK')
    for name in ('client', 'research'):
        path = cache / name
        (path / '.build/compiler').mkdir(parents=True)
        (path / '.build/compiler/cache').write_bytes(b'rebuildable compiler state')
        (path / 'Package.swift').write_text('// generated workspace')
    source = cache / 'client' / cleanup.APK_DIRECTORY
    source.mkdir(parents=True)
    (source / 'app-debug.apk').write_bytes(b'live emulator APK')
    (source / 'output-metadata.json').write_text('{}')
    return cache, artifacts, source


class AndroidCleanupTests(unittest.TestCase):
    def test_inactive_workspaces_are_removed_and_final_apk_is_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            cache, artifacts, _ = fixture(Path(temporary).resolve())
            workspaces = cleanup.recognized_workspaces(cache, artifacts)
            cleanup.apply_plan(cache, artifacts, workspaces, [], os.getuid(), os.getgid())
            self.assertFalse((cache / 'client').exists())
            self.assertFalse((cache / 'research').exists())
            self.assertEqual((artifacts.parent / 'longlivehealth.apk').read_bytes(), b'current exported APK')

    def test_live_directory_inode_and_restart_path_survive_idempotent_cleanup(self):
        with tempfile.TemporaryDirectory() as temporary:
            cache, artifacts, source = fixture(Path(temporary).resolve())
            inode = source.stat().st_ino
            digest = hashlib.sha256((source / 'app-debug.apk').read_bytes()).hexdigest()
            mount = [{'source': str(source.relative_to(cache)), 'container': 'a' * 12}]
            for _ in range(2):
                workspaces = cleanup.recognized_workspaces(cache, artifacts)
                cleanup.apply_plan(cache, artifacts, workspaces, mount, os.getuid(), os.getgid())
                self.assertTrue(source.is_symlink())
                self.assertEqual(source.stat().st_ino, inode)
                self.assertEqual(hashlib.sha256((source / 'app-debug.apk').read_bytes()).hexdigest(), digest)
                self.assertFalse((cache / 'client/.build/compiler').exists())
                self.assertFalse((cache / 'research').exists())
            cleanup.apply_plan(cache, artifacts, ['client'], [], os.getuid(), os.getgid())
            self.assertFalse((cache / 'client').exists())
            self.assertTrue((artifacts / ('a' * 12) / 'app-debug.apk').exists())

    def test_workspace_mount_is_rejected_and_recorded_bind_paths_are_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            cache, artifacts, source = fixture(Path(temporary).resolve())
            container = {
                'Id': 'a' * 64,
                'Mounts': [{'Type': 'bind', 'Source': str(cache / 'client')}],
                'HostConfig': {},
            }
            with self.assertRaises(ValueError):
                cleanup.artifact_mount_plan(cache, ['client', 'research'], [container])
            self.assertTrue((cache / 'research/Package.swift').exists())
            container['Mounts'] = []
            container['HostConfig']['Binds'] = [str(source) + ':/artifacts:ro']
            self.assertEqual(
                cleanup.artifact_mount_plan(cache, ['client'], [container]),
                [{'source': str(source.relative_to(cache)), 'container': 'a' * 12}],
            )

    def test_artifact_conflicts_and_symlink_parents_fail_before_deletion(self):
        with tempfile.TemporaryDirectory() as temporary:
            cache, artifacts, source = fixture(Path(temporary).resolve())
            destination = artifacts / ('a' * 12)
            destination.mkdir(parents=True)
            mount = [{'source': str(source.relative_to(cache)), 'container': 'a' * 12}]
            with self.assertRaises(ValueError):
                cleanup.apply_plan(cache, artifacts, ['client', 'research'], mount, os.getuid(), os.getgid())
            self.assertTrue((cache / 'research/Package.swift').exists())
            self.assertTrue(source.is_dir())
            destination.rmdir()
            original_build = cache / 'client/.build'
            external = Path(temporary).resolve() / 'external'
            original_build.rename(external)
            original_build.symlink_to(external, target_is_directory=True)
            with self.assertRaises(ValueError):
                cleanup.apply_plan(cache, artifacts, ['client', 'research'], mount, os.getuid(), os.getgid())
            self.assertTrue((cache / 'research/Package.swift').exists())
            self.assertTrue((external / 'compiler/cache').exists())

    def test_unknown_workspace_cannot_escape_cleanup_scope(self):
        with tempfile.TemporaryDirectory() as temporary:
            cache, artifacts, _ = fixture(Path(temporary).resolve())
            with self.assertRaises(ValueError):
                cleanup.apply_plan(cache, artifacts, ['../../backups'], [], os.getuid(), os.getgid())
            self.assertTrue((cache / 'research/Package.swift').exists())

    def test_failed_path_link_restores_the_original_mounted_directory(self):
        with tempfile.TemporaryDirectory() as temporary:
            cache, artifacts, source = fixture(Path(temporary).resolve())
            inode = source.stat().st_ino
            mount = [{'source': str(source.relative_to(cache)), 'container': 'a' * 12}]
            with patch.object(Path, 'symlink_to', side_effect=PermissionError('synthetic link failure')):
                with self.assertRaises(PermissionError):
                    cleanup.apply_plan(cache, artifacts, ['client', 'research'], mount, os.getuid(), os.getgid())
            self.assertEqual(source.stat().st_ino, inode)
            self.assertFalse(source.is_symlink())
            self.assertTrue((cache / 'research/Package.swift').exists())

    def test_build_lock_prevents_cleanup_while_a_build_is_active(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            path = root / 'build/.locks/docker.lock'
            path.parent.mkdir(parents=True)
            with path.open('a+b') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                with self.assertRaises(BlockingIOError):
                    with cleanup.build_lock(root):
                        self.fail('An active build lock must not be bypassed')


if __name__ == '__main__':
    unittest.main()
