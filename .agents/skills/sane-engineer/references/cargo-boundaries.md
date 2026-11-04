# Cargo dependency boundaries

The checker uses `cargo metadata --no-deps --offline --format-version 1` and
Python's standard library. It reports forbidden direct dependencies and transitive
paths through workspace packages, including dependencies declared under target
conditions and optional dependencies. It does not resolve paths through external
packages. Default kinds are normal and build; dev dependencies are opt-in.

Rules name a workspace source package and forbidden package-name glob patterns:

```json
{
  "rules": [
    {"from": "orchestrator", "forbid": ["workload-*", "bollard"]},
    {"from": "workload-lifecycle", "forbid": ["workload-docker", "bollard"]}
  ]
}
```

These rules illustrate the doramcap boundaries agreed during the motivating review.
They are not automatically applied to other repositories. A lifecycle contract may
depend on generic execution; a backend may depend on both. A concrete workflow can
live in the backend crate or a consuming application according to its actual scope.

Optional rule field: `"kinds": ["normal", "build", "dev"]`. Kinds apply to every
edge traversed for that rule. A test-only dependency can be permitted while production
imports are forbidden. Match dependency package names, including renamed imports.

Exit codes: 0 pass; 1 boundary violation; 2 malformed input, missing source package,
metadata failure or ambiguous workspace package names. Rules require at least one
entry; unknown keys and invalid kinds fail instead of silently weakening the check.
Unmatched forbidden patterns are allowed to prohibit future additions.

Run against a saved Cargo metadata JSON with `--metadata path.json` for fixtures.
Run `python3 scripts/test_boundaries.py` for behavioral checks. The helper is read-only.
It cannot detect conceptual leakage through public fields, comments, re-exports or
external dependency internals; review those separately.

For CI, copy this helper into repository tooling and commit repository-owned rules.
Use an existing script/job and run both a passing real graph and a deliberately
forbidden fixture before declaring enforcement complete. Follow repository policy
for dependency adoption; this helper itself needs no third-party Python packages.
