#!/usr/bin/env python3
"""Run prebuilt policy probes with timeouts; mutate only a unique test cgroup subtree."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile


def cross_language(binary):
    from google.protobuf import descriptor_pb2, descriptor_pool, message_factory, text_format
    with tempfile.TemporaryDirectory(prefix='policy-wire-') as directory:
        directory = Path(directory)
        descriptor = directory / 'policy.bin'
        subprocess.run([binary, '--descriptor', str(descriptor)], check=True, timeout=10)
        descriptors = descriptor_pb2.FileDescriptorSet.FromString(descriptor.read_bytes())
        pool = descriptor_pool.DescriptorPool()
        for item in descriptors.file:
            pool.Add(item)
        desc = pool.FindMessageTypeByName('pbtxt_config.v1.Fragment')
        if hasattr(message_factory, 'GetMessageClass'):
            cls = message_factory.GetMessageClass(desc)
        else:
            cls = message_factory.MessageFactory(pool).GetPrototype(desc)
        source = directory / 'input.pbtxt'
        wire = directory / 'wire.bin'
        source.write_text('app { id: "camera" config { [type.googleapis.com/resource_policy.v1.Policy] { placement { cpus: "4,5-7,9-11" } rt_priority: 0 } } }')
        expected = text_format.Parse(source.read_text(), cls(), descriptor_pool=pool)
        subprocess.run([binary, '--wire', str(source), str(wire)], check=True, timeout=10)
        decoded = cls.FromString(wire.read_bytes())
        assert decoded == expected
        policy_desc = pool.FindMessageTypeByName('resource_policy.v1.Policy')
        policy_cls = message_factory.GetMessageClass(policy_desc) if hasattr(message_factory, 'GetMessageClass') else message_factory.MessageFactory(pool).GetPrototype(policy_desc)
        policy = policy_cls.FromString(decoded.app[0].config.value)
        assert policy.HasField('rt_priority')
        assert policy.rt_priority == 0
        source.write_text(text_format.MessageToString(decoded))
        subprocess.run([binary, '--wire', str(source), str(wire)], check=True, timeout=10)
        assert cls.FromString(wire.read_bytes()) == expected
    print('PASS Python/Rust pbtxt and binary interoperability, explicit zero presence')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--enable-root-cpuset', action='store_true',
                        help='approved test-only enable/restore of the root cpuset controller')
    args = parser.parse_args()
    binary = str(args.binary.resolve())
    cross_language(binary)
    assert os.geteuid() == 0, 'native tests require root'
    cpus = sorted(os.sched_getaffinity(0))
    assert len(cpus) >= 2, 'two CPUs required for watchdog separation'
    # Keep watchdog execution on a housekeeping CPU, separate from RT workloads.
    os.sched_setaffinity(0, {cpus[0]})
    root = Path('/sys/fs/cgroup') / f'resource-policy-validation-{os.getpid()}'
    control = Path('/sys/fs/cgroup/cgroup.subtree_control')
    enabled_root = False
    created_root = False
    try:
        if 'cpuset' not in control.read_text().split():
            if not args.enable_root_cpuset:
                raise RuntimeError('cpuset delegation unavailable; host setup requires separate approval')
            control.write_text('+cpuset')
            enabled_root = True
            print('Temporarily enabled root cpuset for approved native tests', flush=True)
        root.mkdir()
        created_root = True
        # Controller delegation below this point affects only our test subtree.
        assert 'cpuset' in (root / 'cgroup.controllers').read_text().split()
        (root / 'cgroup.subtree_control').write_text('+cpuset')
        for priority in (0, 1, 30):
            subprocess.run([binary, str(root), f'app-{priority}', str(cpus[-1]), str(priority)], check=True, timeout=15)
        (root / 'cpuset.cpus').write_text(str(cpus[-1]))
        subprocess.run([binary, str(root), 'rejected', str(cpus[0]), 'reject'], check=True, timeout=10)
        # The dedicated probe drops identities after exec, inside private CI directories.
        subprocess.run([binary, '--denied', '-', str(cpus[-1])], check=True, timeout=10)
        contenders = []
        try:
            for priority in (0, 1, 30):
                contenders.append(subprocess.Popen([binary, '--contend', str(priority), str(cpus[-1])]))
            for process in contenders:
                assert process.wait(timeout=10) == 0
        finally:
            for process in contenders:
                if process.poll() is None:
                    process.kill()
                process.wait()
        print(f'PASS native resource-policy on {os.uname().machine} {os.uname().release}')
    finally:
        try:
            # Timeout kills the probe; remove empty owned leaves, never recursively delete.
            if created_root:
                for child in root.iterdir():
                    if child.is_dir():
                        child.rmdir()
                root.rmdir()
        finally:
            try:
                if enabled_root:
                    control.write_text('-cpuset')
                    assert 'cpuset' not in control.read_text().split()
                    print('Restored original root cpuset controller setting', flush=True)
            finally:
                os.sched_setaffinity(0, cpus)



if __name__ == '__main__':
    main()
