#!/usr/bin/env python3
"""Persistent, flock-serialized Git worktree pool. Python 3, Git, Linux/macOS."""
import argparse
import fcntl
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import uuid


def run(*args, cwd=None, data=None):
    p = subprocess.run(args, cwd=cwd, input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if p.returncode:
        raise RuntimeError(p.stderr.decode(errors='replace').strip())
    return p.stdout


def git(root, *args, data=None):
    return run('git', '-C', str(root), *args, data=data)


def save(path, state):
    tmp = path.with_suffix('.tmp')
    tmp.write_text(json.dumps(state, indent=2) + '\n')
    os.replace(tmp, path)


def dirt(path):
    return git(path, 'status', '--short', '--untracked-files=all', '--ignored').decode()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--repo', default='.', help='Any checkout in the target repository')
    sub = p.add_subparsers(dest='command', required=True)
    sub.add_parser('init')
    sub.add_parser('status')
    a = sub.add_parser('acquire', aliases=['acq'])
    a.add_argument('--owner', required=True, help='CLI/session/task identifier')
    a.add_argument('--branch', required=True, help='New feature branch name')
    a.add_argument('--discard-dirty', action='store_true', help='Only after user authorizes the displayed cleanup')
    r = sub.add_parser('release')
    r.add_argument('--token', required=True)
    r.add_argument('--pr', required=True, help='Published GitHub PR URL; verified with gh')
    args = p.parse_args()
    common = Path(git(args.repo, 'rev-parse', '--path-format=absolute', '--git-common-dir').decode().strip()).resolve()
    records = git(args.repo, 'worktree', 'list', '--porcelain', '-z').split(b'\0')
    main_root = Path(os.fsdecode(records[0][len(b'worktree '):])).resolve()
    if git(main_root, 'rev-parse', '--is-bare-repository').strip() == b'true':
        raise RuntimeError('A non-bare main checkout is required')
    base = common / 'agent-pool'
    base.mkdir(exist_ok=True)
    with (base / 'lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        statefile = base / 'state.json'
        state = json.loads(statefile.read_text()) if statefile.exists() else {'slots': []}
        def safe(slot):
            path = Path(slot['path']).resolve()
            expected = (base / 'worktrees' / str(slot['id'])).resolve()
            if path != expected or path == main_root or not path.is_relative_to(base):
                raise RuntimeError('Unsafe pool path; refusing mutation')
            if path.exists() and Path(git(path, 'rev-parse', '--show-toplevel').decode().strip()).resolve() != path:
                raise RuntimeError('Pool directory is not a worktree root')
            return path
        if args.command == 'init':
            for i in range(1, 6):
                if any(s['id'] == i for s in state['slots']):
                    continue
                slot = {'id': i, 'path': str(base / 'worktrees' / str(i)), 'lease': None}
                path = safe(slot)
                if path.exists():
                    raise RuntimeError(f'Unregistered path {path}; inspect manually')
                git(main_root, 'worktree', 'add', '--detach', str(path), 'HEAD')
                state['slots'].append(slot)
                save(statefile, state)
        elif args.command in ('acquire', 'acq'):
            if len(state['slots']) != 5:
                raise RuntimeError('Run init first; pool must contain five slots')
            git(main_root, 'check-ref-format', '--branch', args.branch)
            if subprocess.run(['git', '-C', str(main_root), 'show-ref', '--verify', '--quiet', 'refs/heads/' + args.branch]).returncode != 1:
                raise RuntimeError('Branch already exists or could not be checked; use a new branch')
            free = [s for s in state['slots'] if s['lease'] is None]
            if not free:
                raise RuntimeError('Pool exhausted; leases never expire. Wait for an explicit release.')
            slot = next((s for s in free if not dirt(safe(s))), free[0])
            path = safe(slot)
            dirty = dirt(path)
            if dirty:
                print(f'WARNING: refreshing {path} will discard these tracked, untracked or ignored files:\n{dirty}', file=sys.stderr)
                if not args.discard_dirty:
                    raise RuntimeError('Cleanup blocked. Show warning to user and obtain approval before --discard-dirty.')
            head = git(main_root, 'rev-parse', 'HEAD').decode().strip()
            patch = git(main_root, 'diff', '--binary', 'HEAD', '--')
            untracked = git(main_root, 'ls-files', '--others', '--exclude-standard', '-z').split(b'\0')
            if git(main_root, 'ls-files', '-u'):
                raise RuntimeError('Main checkout has unresolved conflicts; resolve before acquiring')
            token = str(uuid.uuid4())
            slot['lease'] = {'token': token, 'owner': args.owner, 'branch': args.branch, 'base': head, 'state': 'preparing'}
            save(statefile, state)  # Fail closed: interrupted refresh remains reserved, without expiry.
            try:
                if dirty:
                    git(path, 'reset', '--hard', 'HEAD')
                    git(path, 'clean', '-fdx')
                    if dirt(path):
                        raise RuntimeError('Cleanup left files (possibly a nested repository); inspect manually')
                git(path, 'checkout', '--detach', head)
                git(path, 'checkout', '-b', args.branch)
                if patch:
                    git(path, 'apply', '--binary', '-', data=patch)
                for raw in untracked:
                    if not raw:
                        continue
                    rel = Path(os.fsdecode(raw))
                    src, dst = main_root / rel, path / rel
                    if not dst.parent.resolve().is_relative_to(path):
                        raise RuntimeError(f'Unsafe destination: {rel}')
                    dst.parent.mkdir(parents=True, exist_ok=True)
                    if src.is_symlink():
                        dst.symlink_to(os.readlink(src))
                    else:
                        shutil.copy2(src, dst)
                if git(main_root, 'rev-parse', 'HEAD').decode().strip() != head or git(main_root, 'diff', '--binary', 'HEAD', '--') != patch or git(main_root, 'ls-files', '--others', '--exclude-standard', '-z').split(b'\0') != untracked:
                    raise RuntimeError('Main checkout changed during refresh; inspect the reserved slot')
                slot['lease']['state'] = 'active'
                save(statefile, state)
            except Exception:
                print(f'Refresh failed; slot remains reserved with token {token}. Inspect status and repair manually; never steal a lease.', file=sys.stderr)
                raise
            print(json.dumps(slot, indent=2))
            return
        elif args.command == 'release':
            slot = next((s for s in state['slots'] if s['lease'] and s['lease']['token'] == args.token), None)
            if slot is None:
                raise RuntimeError('Unknown lease token')
            path = safe(slot)
            lease = slot['lease']
            if lease['state'] != 'active':
                raise RuntimeError('Incomplete acquisition needs manual repair; cannot release')
            branch = git(path, 'branch', '--show-current').decode().strip()
            head = git(path, 'rev-parse', 'HEAD').decode().strip()
            pr = json.loads(run('gh', 'pr', 'view', args.pr, '--json', 'url,state,headRefName,headRefOid', cwd=path))
            if pr['url'] != args.pr or pr['state'] not in ('OPEN', 'MERGED') or pr['headRefName'] != branch or branch != lease['branch'] or pr['headRefOid'] != head:
                raise RuntimeError('PR must be published for the leased branch at its current HEAD')
            if git(path, 'status', '--porcelain', '--untracked-files=all'):
                raise RuntimeError('Worktree has unpublished local edits. Commit/push them or preserve them before release.')
            slot['last_pr'] = pr['url']
            slot['lease'] = None
            save(statefile, state)
        print(json.dumps(state, indent=2))


if __name__ == '__main__':
    try:
        main()
    except (RuntimeError, OSError, ValueError) as e:
        print(f'Error: {e}', file=sys.stderr)
        sys.exit(1)
