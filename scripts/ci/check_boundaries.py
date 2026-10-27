#!/usr/bin/env python3
"""Check explicit forbidden Cargo dependency paths; read-only, standard library only."""
import argparse
from collections import deque
import fnmatch
import json
from pathlib import Path
import subprocess
import sys


def violations(metadata, config):
    if set(config) != {"rules"} or not isinstance(config["rules"], list) or not config["rules"]:
        raise ValueError("config requires a nonempty rules list and no other keys")
    members = set(metadata["workspace_members"])
    packages = [p for p in metadata["packages"] if p["id"] in members]
    by_name = {p["name"]: p for p in packages}
    if len(by_name) != len(packages):
        raise ValueError("ambiguous workspace package names")
    errors = []
    for rule in config["rules"]:
        if not isinstance(rule, dict) or set(rule) - {"from", "forbid", "kinds"}:
            raise ValueError("unknown rule fields")
        source = rule.get("from")
        patterns = rule.get("forbid")
        kinds = rule.get("kinds", ["normal", "build"])
        if not isinstance(source, str) or source not in by_name:
            raise ValueError(f"unknown source package: {source!r}")
        if not isinstance(patterns, list) or not patterns or any(not isinstance(p, str) or not p for p in patterns):
            raise ValueError("forbid must be a nonempty list of patterns")
        if not isinstance(kinds, list) or not kinds or any(k not in ("normal", "build", "dev") for k in kinds):
            raise ValueError("invalid dependency kinds")
        queue = deque([(source, [source])])
        visited = {source}
        while queue:
            name, path = queue.popleft()
            for dep in by_name[name]["dependencies"]:
                if (dep.get("kind") or "normal") not in kinds:
                    continue
                target = dep["name"]
                next_path = path + [target]
                if any(fnmatch.fnmatchcase(target, pattern) for pattern in patterns):
                    errors.append(" -> ".join(next_path))
                # Traverse only actual workspace path dependencies. An external
                # package can have the same name as an unrelated workspace member.
                package = by_name.get(target)
                local = package and dep.get("path") and Path(dep["path"]).resolve() == Path(package["manifest_path"]).resolve().parent
                if local and target not in visited:
                    visited.add(target)
                    queue.append((target, next_path))
    return sorted(set(errors))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--rules", type=Path, required=True)
    parser.add_argument("--metadata", type=Path)
    args = parser.parse_args()
    try:
        config = json.loads(args.rules.read_text())
        if args.metadata:
            metadata = json.loads(args.metadata.read_text())
        else:
            result = subprocess.run(["cargo", "metadata", "--no-deps", "--offline", "--format-version", "1"], cwd=args.repo, check=True, capture_output=True, text=True)
            metadata = json.loads(result.stdout)
        found = violations(metadata, config)
    except (ValueError, KeyError, TypeError, OSError, subprocess.CalledProcessError) as exc:
        print(f"boundary check failed: {exc}", file=sys.stderr)
        return 2
    for path in found:
        print(f"forbidden dependency: {path}")
    if not found:
        print("Dependency boundaries passed (workspace paths and direct dependencies).")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
