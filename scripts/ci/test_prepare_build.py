"""Exercise source-cache correctness with the real rsync executable."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('prepare_build', Path(__file__).with_name('prepare-build.py'))
prepare_build = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare_build)


class PrepareBuildTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.cache = self.root / 'cache'

    def snapshot(self, name, sources):
        snapshot = self.root / name
        snapshot.mkdir()
        manifest = {'files_sha256': {}, 'files_mode': {}}
        for relative, data in sources.items():
            path = snapshot / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(data)
            manifest['files_sha256'][relative] = prepare_build.digest(path)
            manifest['files_mode'][relative] = 0o644
            path.chmod(0o644)
        for relative, key in [('run-worker-job.sh', 'runner_sha256'), ('prepare-build.py', 'prepare_sha256')]:
            path = snapshot / relative
            path.write_text('runner helper')
            manifest[key] = prepare_build.digest(path)
        (snapshot / 'SOURCE.json').write_text(json.dumps(manifest))
        (snapshot / 'source.tar.gz').write_bytes(b'archive remains in snapshot')
        return snapshot

    def test_unchanged_edit_delete_and_rename_across_submissions(self):
        # Arrange
        first = self.snapshot('first', {'libs/a.rs': 'old', 'libs/gone.rs': 'gone'})
        prepare_build.prepare(first, self.cache)
        cached = self.cache / 'libs/a.rs'
        os.utime(cached, ns=(1_000_000_000, 1_000_000_000))
        identical = self.snapshot('identical', {'libs/a.rs': 'old', 'libs/gone.rs': 'gone'})
        # Act
        prepare_build.prepare(identical, self.cache)
        # Assert
        self.assertEqual(cached.stat().st_mtime_ns, 1_000_000_000)
        self.assertFalse((self.cache / 'source.tar.gz').exists())
        self.assertTrue((self.cache / '__pycache__').is_dir())
        self.assertEqual((self.cache / '__pycache__').stat().st_uid, os.getuid())
        # Arrange: same-size edit with an old source timestamp must still update.
        edited = self.snapshot('edited', {'libs/a.rs': 'new', 'libs/renamed.rs': 'gone'})
        os.utime(edited / 'libs/a.rs', ns=(1_000_000_000, 1_000_000_000))
        (self.cache / 'generated.tmp').write_text('stale generated output')
        # Act
        prepare_build.prepare(edited, self.cache)
        # Assert
        self.assertEqual(cached.read_text(), 'new')
        self.assertGreater(cached.stat().st_mtime_ns, 1_000_000_000)
        self.assertFalse((self.cache / 'libs/gone.rs').exists())
        self.assertTrue((self.cache / 'libs/renamed.rs').exists())
        self.assertFalse((self.cache / 'generated.tmp').exists())
        self.assertEqual((first / 'libs/a.rs').read_text(), 'old')

    def test_permission_only_change(self):
        # Arrange
        first = self.snapshot('first', {'run.sh': 'echo ok'})
        prepare_build.prepare(first, self.cache)
        cached = self.cache / 'run.sh'
        before = cached.stat().st_mtime_ns
        second = self.snapshot('second', {'run.sh': 'echo ok'})
        (second / 'run.sh').chmod(0o755)
        manifest = json.loads((second / 'SOURCE.json').read_text())
        manifest['files_mode']['run.sh'] = 0o755
        (second / 'SOURCE.json').write_text(json.dumps(manifest))
        # Act
        prepare_build.prepare(second, self.cache)
        # Assert
        self.assertEqual(cached.stat().st_mode & 0o777, 0o755)
        self.assertEqual(cached.stat().st_mtime_ns, before)

    def test_invalid_snapshot_preserves_cache(self):
        for corruption in ('content', 'extra', 'symlink', 'runner', 'mode'):
            with self.subTest(corruption=corruption):
                # Arrange
                snapshot = self.snapshot(corruption, {'src.rs': 'valid'})
                self.cache.mkdir(exist_ok=True)
                sentinel = self.cache / 'sentinel'
                sentinel.write_text('keep')
                if corruption == 'content':
                    (snapshot / 'src.rs').write_text('tampered')
                elif corruption == 'extra':
                    (snapshot / 'extra').write_text('unlisted')
                elif corruption == 'symlink':
                    (snapshot / 'link').symlink_to(snapshot / 'src.rs')
                elif corruption == 'runner':
                    (snapshot / 'run-worker-job.sh').write_text('tampered')
                else:
                    (snapshot / 'src.rs').chmod(0o755)
                # Act / Assert
                with self.assertRaises(ValueError):
                    prepare_build.prepare(snapshot, self.cache)
                self.assertEqual(sentinel.read_text(), 'keep')

    def test_destination_symlink_is_rejected(self):
        # Arrange
        snapshot = self.snapshot('snapshot', {'src.rs': 'valid'})
        outside = self.root / 'outside'
        outside.mkdir()
        self.cache.symlink_to(outside)
        # Act / Assert
        with self.assertRaises(ValueError):
            prepare_build.prepare(snapshot, self.cache)
        self.assertEqual(list(outside.iterdir()), [])


if __name__ == '__main__':
    unittest.main()
