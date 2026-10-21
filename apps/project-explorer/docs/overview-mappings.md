# Overview mapping contract

[Canonical JSON](../overview.json) · [JSON Schema](../overview.schema.json)

`overview.json` is the versioned source for the overview narrative, capabilities,
third-party integrations, and use cases. React renders this manifest; mappings are
not duplicated in component code. The static site serves exact copies of the JSON
and schema for local inspection. Catalog names, summaries and dependency lists
continue to come from the repository.

| Field | Contract |
|---|---|
| `version` | Schema version, currently `1`; unsupported versions fail validation |
| `id` | Stable lowercase slug, unique within its collection; used in deep links |
| `components` | Exact generated IDs such as `apps/provenance-demo` or `crates/activity-tracing` |
| `evidence` | Existing repository Markdown paths supporting the mapping |
| `capabilities` | Use-case references to declared capability IDs |
| `integrations` | References to declared third-party integration IDs |
| `group` | Integration presentation group: `execution` or `communication` (default) |
| `status` | Explicit maturity; illustrative use cases describe composition possibilities |
| `limits` | Capability-specific boundaries and future scope |

## Editing

1. Update the canonical JSON and its evidence links in the same change.
2. Keep component descriptions in their README; use the manifest for editorial context.
3. Every use-case component and integration must belong to one of its capabilities.
4. Keep status claims supported by the cited evidence. A schema check establishes
   referential integrity; validating end-to-end behavior requires integration tests.

## CI checks

From the repository root:

```sh
npm --prefix apps/project-explorer/frontend ci
npm --prefix apps/project-explorer/frontend run build
```

Generation runs Ajv against the strict JSON Schema, then verifies duplicate IDs,
component/capability/integration references, evidence documents, and membership.
Any failure exits nonzero before emitting the overview.

Deep links use `#/overview?useCase=perception`, `?capability=memory`, or
`?integration=dora`. The first use case in manifest order is selected when `useCase` is omitted.
The selected use case can accompany a capability or integration.
Unknown IDs produce a recoverable message.

Linux and CUDA appear in the execution substrate; Dora, MCAP and OpenTelemetry
appear under communication and evidence. CUDA is platform-dependent, and OTel is
optional. The selected mapping detail precedes the compact architecture stack.
