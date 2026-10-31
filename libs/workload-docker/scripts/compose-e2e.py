#!/usr/bin/env python3
"""Run shared-Dora workflow tests using a prebuilt doramcap/dora:local image."""
import argparse
import json
import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--profile', default='ci')
    parser.add_argument('--socket', type=Path, default=Path('/var/run/docker.sock'))
    args = parser.parse_args()
    socket = args.socket.absolute()
    docker = ['docker', '--host', f'unix://{socket}']
    output = args.output.absolute()
    output.mkdir(parents=True, exist_ok=False)
    repository = ROOT if (ROOT / ".git").exists() else Path.cwd()
    main_checkout = Path(subprocess.check_output(
        ['git', 'worktree', 'list', '--porcelain'], cwd=repository, text=True,
    ).splitlines()[0].removeprefix('worktree '))
    credential = main_checkout / '.sudo-password'
    if not credential.is_file() or credential.stat().st_mode & 0o777 != 0o600:
        raise SystemExit('Provision main .sudo-password locally, mode 0600')
    subprocess.run(['git', 'check-ignore', '-q', '.sudo-password'], cwd=main_checkout, check=True)

    def elevated(command, **kwargs):
        with credential.open('rb') as secret:
            return subprocess.run(['sudo', '-S', '-p', '', '--', *command], stdin=secret, check=True, **kwargs)

    cargo = ['cargo', '--locked', '--profile', args.profile]
    subprocess.run([cargo[0], 'build', *cargo[1:], '-p', 'mcap-recorder', '--bins', '--examples'], cwd=ROOT, check=True)
    built = subprocess.run([cargo[0], 'test', *cargo[1:], '-p', 'workload-docker', '--test', 'docker',
                            '--no-run', '--message-format=json'], cwd=ROOT, check=True, capture_output=True, text=True)
    executable = next(item['executable'] for item in map(json.loads, built.stdout.splitlines()) if item.get('executable'))
    # Keep child-process restarts valid while Cargo replaces its outputs.
    stable_executable = output / 'native-tests'
    shutil.copy2(executable, stable_executable); executable = str(stable_executable)
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--no-deps', '--format-version', '1'], cwd=ROOT))
    target = Path(metadata['target_directory']) / ('debug' if args.profile == 'dev' else args.profile)
    fixture = target / 'examples/dora_attachment_fixture'
    recorder = target / 'mcap-recorder'
    image = f'doramcap-compose-test:shared-{os.getpid()}'
    with tempfile.TemporaryDirectory(prefix='compose-image-', dir=output) as temporary:
        stage = Path(temporary)
        filesystem = stage / 'rootfs'
        filesystem.mkdir()
        programs = [recorder, fixture, *map(Path, ['/bin/sh', '/bin/sleep', '/bin/true', '/bin/false', '/bin/rm', '/bin/touch', '/usr/bin/test'])]
        for source in programs:
            if source not in (recorder, fixture):
                destination = filesystem / 'bin' / source.name
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, destination)
            for line in subprocess.check_output(['ldd', str(source)], text=True).splitlines():
                match = re.search(r'(?:=>\s+)?(/\S+)', line)
                if match:
                    library = Path(match.group(1))
                    destination = filesystem / library.relative_to('/')
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(library, destination)
        (filesystem / 'tmp').mkdir()
        (stage / 'Dockerfile').write_text('FROM scratch\nCOPY rootfs /\nENV HOME=/tmp\nENTRYPOINT ["/bin/sh"]\n')
        subprocess.run([*docker, 'build', '--network=none', '-t', image, str(stage)], check=True)
    environment = ['env', f'TMPDIR={output}', f'DORAMCAP_TEST_IMAGE={image}',
                   f'DORAMCAP_TEST_SOCKET={socket}', f'DORAMCAP_TEST_DORA_NODE={fixture}',
                   f'DORAMCAP_TEST_MCAP_RECORDER={recorder}']
    cases = [
        ('public_dora_', 7),
        ('direct_session_lifecycle_and_prerun_reset', 1),
        ('parallel_projects_preserve_readiness_and_shutdown_barriers', 1),
        ('active_experiments_are_exclusive_per_repository_and_independent_between_repositories', 1),
        ('reset_failure_blocks_preparation_and_retries_after_partial_removal', 1),
        ('stable_project_keeps_retained_volumes_isolated_between_experiments', 1),
        ('reset_discards_fixed_workspace_without_launch_history', 1),
        ('shared_experiment_preserves_native_inputs_and_rejects_conflicts', 1),
        ('mixed_volume_policies_and_evidence_preservation', 1),
        ('incomplete_volume_scan_retains_volumes', 1),
    ]
    for case, expected in cases:
        log = output / f'{case}.log'
        try:
            with log.open('wb') as stream:
                elevated([*environment, executable, case, '--ignored', '--nocapture', '--test-threads=1'],
                         stdout=stream, stderr=subprocess.STDOUT)
        except subprocess.CalledProcessError:
            raise SystemExit(f'FAIL: {case}; evidence and owned resources retained; see {log}') from None
        results = re.findall(r'^test result: ok\. (\d+) passed;', log.read_text(), re.MULTILINE)
        if not results or int(results[-1]) != expected:
            raise SystemExit(f'FAIL: {case} did not run {expected} tests; see {log}')
        print(f'PASS: {case}', flush=True)
    subprocess.run([*docker, 'image', 'rm', image], check=True)
    print(f'PASS: shared-Dora workflow; evidence {output}')


if __name__ == '__main__':
    main()
