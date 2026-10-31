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
    args = parser.parse_args()
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
        elevated(['docker', 'build', '--network=none', '-t', image, str(stage)])
    environment = ['env', f'TMPDIR={output}', f'DORAMCAP_TEST_IMAGE={image}',
                   'DORAMCAP_TEST_SOCKET=/var/run/docker.sock', f'DORAMCAP_TEST_DORA_NODE={fixture}',
                   f'DORAMCAP_TEST_MCAP_RECORDER={recorder}']
    cases = ['public_dora_', 'direct_session_lifecycle_and_prerun_reset',
             'parallel_projects_preserve_readiness_and_shutdown_barriers',
             'active_experiment_excludes_same_and_different_repositories',
             'reset_failure_blocks_preparation_and_retries_after_partial_removal',
             'stable_project_keeps_retained_volumes_isolated_between_experiments',
             'shared_experiment_preserves_native_inputs_and_rejects_conflicts',
             'mixed_volume_policies_and_evidence_preservation',
             'incomplete_volume_scan_retains_volumes']
    for case in cases:
        log = output / f'{case}.log'
        try:
            with log.open('wb') as stream:
                elevated([*environment, executable, case, '--ignored', '--nocapture', '--test-threads=1'],
                         stdout=stream, stderr=subprocess.STDOUT)
        except subprocess.CalledProcessError:
            raise SystemExit(f'FAIL: {case}; evidence and owned resources retained; see {log}') from None
        print(f'PASS: {case}', flush=True)
    elevated(['docker', 'image', 'rm', image])
    print(f'PASS: shared-Dora workflow; evidence {output}')


if __name__ == '__main__':
    main()
