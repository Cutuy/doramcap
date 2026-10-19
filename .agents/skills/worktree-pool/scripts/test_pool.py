#!/usr/bin/env python3
"""Release behavior exercised in disposable Git repositories."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).with_name('pool.py')
spec = importlib.util.spec_from_file_location('pool', SCRIPT)
pool = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pool)


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name) / 'repo'
        self.repo.mkdir()
        self.git('init', '-b', 'main')
        self.git('config', 'user.email', 'test@example.com')
        self.git('config', 'user.name', 'Test')
        (self.repo / '.gitignore').write_text(
            'target/\napps/*/frontend/dist/\nnode_modules/\n.env\n')
        self.git('add', '.gitignore')
        self.git('commit', '-m', 'Initial')
        self.command('init')
        self.slot = json.loads(self.command(
            'acq', '--owner', 'test', '--branch', 'codex/test').stdout)
        self.worktree = Path(self.slot['path'])
        self.token = self.slot['lease']['token']

    def git(self, *args, root=None):
        return subprocess.run(
            ['git', '-C', str(root or self.repo), *args],
            capture_output=True, check=True)

    def command(self, *args, check=True):
        return subprocess.run(
            [sys.executable, str(SCRIPT), '--repo', str(self.repo), *args],
            capture_output=True, check=check)

    def release(self):
        return self.command('release', '--token', self.token, '--main', check=False)

    def lease(self):
        state = json.loads((self.repo / '.git/agent-pool/state.json').read_text())
        return state['slots'][0]['lease']

    def write(self, root, rel):
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('preserve or clean')
        return path

    def test_release_cleans_outputs_and_preserves_other_files(self):
        for rel in pool.BUILD_OUTPUTS:
            self.write(self.worktree, rel + '/artifact')
        cache = self.write(self.worktree, 'node_modules/package/cache')
        secret = self.write(self.worktree, '.env')
        main_artifact = self.write(self.repo, 'target/artifact')
        result = self.release()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNone(self.lease())
        for rel in pool.BUILD_OUTPUTS:
            self.assertFalse((self.worktree / rel).exists())
        for path in (cache, secret, main_artifact):
            self.assertEqual(path.read_text(), 'preserve or clean')

    def test_uncommitted_changes_block_cleanup(self):
        artifact = self.write(self.worktree, 'target/artifact')
        self.write(self.worktree, 'source.rs')
        self.assertNotEqual(self.release().returncode, 0)
        self.assertTrue(artifact.exists())
        self.assertEqual(self.lease()['token'], self.token)

    def test_unintegrated_commits_block_cleanup(self):
        artifact = self.write(self.worktree, 'target/artifact')
        self.write(self.worktree, 'source.rs')
        self.git('add', 'source.rs', root=self.worktree)
        self.git('commit', '-m', 'Feature', root=self.worktree)
        self.assertNotEqual(self.release().returncode, 0)
        self.assertTrue(artifact.exists())
        self.assertIsNotNone(self.lease())

    def test_symlink_blocks_cleanup_and_preserves_external_cache(self):
        artifact = self.write(self.repo, 'target/artifact')
        (self.worktree / 'target').symlink_to(artifact.parent, target_is_directory=True)
        self.assertNotEqual(self.release().returncode, 0)
        self.assertTrue(artifact.exists())
        self.assertIsNotNone(self.lease())

    def test_symlinked_ancestor_blocks_cleanup(self):
        (self.worktree / 'apps').symlink_to(self.repo, target_is_directory=True)
        with self.assertRaisesRegex(RuntimeError, 'symlink'):
            pool.clean_build_outputs(self.worktree)
        self.assertIsNotNone(self.lease())

    def test_tracked_output_blocks_cleanup(self):
        artifact = self.write(self.worktree, 'target/fixture')
        self.git('add', '-f', 'target/fixture', root=self.worktree)
        self.git('commit', '-m', 'Tracked fixture', root=self.worktree)
        commit = self.git('rev-parse', 'HEAD', root=self.worktree).stdout.strip().decode()
        self.git('cherry-pick', commit)
        self.assertNotEqual(self.release().returncode, 0)
        self.assertTrue(artifact.exists())
        self.assertIsNotNone(self.lease())

    def test_cleanup_failure_keeps_lease_for_retry(self):
        artifact = self.write(self.worktree, 'target/artifact')
        argv = ['pool.py', '--repo', str(self.repo), 'release',
                '--token', self.token, '--main']
        with patch.object(sys, 'argv', argv), patch.object(
                pool.shutil, 'rmtree', side_effect=PermissionError('Test failure')):
            with self.assertRaises(PermissionError):
                pool.main()
        self.assertEqual(self.lease()['token'], self.token)
        self.assertTrue(artifact.exists())
        self.assertEqual(self.release().returncode, 0)
        self.assertIsNone(self.lease())


if __name__ == '__main__':
    unittest.main()
