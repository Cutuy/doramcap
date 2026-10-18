#!/usr/bin/env python3
"""Run real Dora nodes and verify finite-session shutdown preserves every sample.

Requires prebuilt --features dora binaries and Dora CLI 1.1.0.
Uses only Python's standard library; MCAP checks support uncompressed chunks,
which is the recorder's configured format.
"""
import argparse
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile


def read_messages(path):
    content = path.read_bytes()
    magic = b"\x89MCAP0\r\n"
    assert content[:8] == magic and content[-8:] == magic, "incomplete MCAP"
    messages = []

    def records(buffer):
        offset = 0
        while offset < len(buffer):
            opcode = buffer[offset]
            length = struct.unpack_from("<Q", buffer, offset + 1)[0]
            data = buffer[offset + 9:offset + 9 + length]
            offset += 9 + length
            assert len(data) == length, "truncated record"
            if opcode == 6:  # Chunk: timestamps, uncompressed size, CRC, encoding.
                encoding_len = struct.unpack_from("<I", data, 28)[0]
                assert data[32:32 + encoding_len] == b"", "expected uncompressed chunk"
                size = struct.unpack_from("<Q", data, 32 + encoding_len)[0]
                records(data[40 + encoding_len:40 + encoding_len + size])
            elif opcode == 5:  # Message
                _, sequence, log_time, publish_time = struct.unpack_from("<HIQQ", data)
                payload = json.loads(data[22:])
                assert sequence == payload["sequence"] & 0xFFFFFFFF
                assert log_time == publish_time == payload["capture_unix_ns"]
                messages.append(payload)

    records(content[8:-8])
    return messages


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dora", default="dora")
    parser.add_argument("--bin-dir", type=Path, default=Path(__file__).resolve().parents[1] / "target/debug")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="doramcap-dora-test-") as directory:
        work = Path(directory)
        # Isolate from concurrent Cargo builds replacing the same target binary.
        for binary in ("system-monitor", "mcap-recorder"):
            shutil.copy2(args.bin_dir / binary, work / binary)
        output = work / "session.mcap"
        descriptor = work / "dataflow.yml"
        descriptor.write_text(json.dumps({"nodes": [
            {"id": "monitor", "path": str(work / "system-monitor"),
             "args": "--backend synthetic --samples 10 --dora", "outputs": ["snapshots"]},
            {"id": "recorder", "path": str(work / "mcap-recorder"), "args": str(output),
             "inputs": {"snapshots": {"source": "monitor/snapshots", "queue_size": 64}}},
        ]}))  # JSON is a YAML subset.
        subprocess.run([args.dora, "run", str(descriptor), "--stop-after", "15s"],
                       check=True, timeout=30)
        messages = read_messages(output)
        assert len(messages) == 10, f"expected 10 final samples, recorded {len(messages)}"
        assert [m["sequence"] for m in messages] == list(range(10))
        assert all(m["source"] == "synthetic" for m in messages)
        assert messages[0]["dictionary_full"] and messages[0]["dictionary"]
        for i, message in enumerate(messages):
            assert message["records"], "missing numeric accounting records"
            assert all(record["runtime_ns"] == (i + 1) * 50_000_000
                       for record in message["records"])
        assert all(a["capture_unix_ns"] < b["capture_unix_ns"]
                   for a, b in zip(messages, messages[1:]))
        print("PASS: 10 Dora samples recorded with original sequence, timestamps, and JSON payloads")


if __name__ == "__main__":
    main()
