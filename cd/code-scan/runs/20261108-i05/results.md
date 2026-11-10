# I-05 filtering pilot

Luna/low screened 20 Rust modules at source revision `65fe14329cc16ce534cc07d1bf55c6a7a2b83272` through `codex exec --output-schema`.

| Outcome | Modules |
| --- | ---: |
| Candidate | 3 |
| Needs context | 2 |
| No signal | 14 |
| Rejected citation | 1 |

- The 19-call parallel batch took 16.3 seconds and reached eight concurrent calls. The initial smoke call took 6.8 seconds; median call duration was 5.2 seconds.
- CLI usage: 308,651 input tokens, including 66,304 cached; 3,382 output tokens; 200 reasoning tokens were also reported. Served model identity and billed cost were not reported.
- Native schema validation passed for all 20 responses. Exact source citations passed for 19; the controller retained the rejected response as unresolved. No tools or code mutations occurred in the filtering agents.

## Candidates and context

| Task | Filter result |
| --- | --- |
| [Systemd Dora startup](i05-020.json) | Repeated readiness polling with a fixed sleep. |
| [Dataflow demo](i05-002.json) | Timer loop flagged; scheduled heartbeat/sampling deadlines may explain it. |
| [Watchdog sinks](i05-004.json) | Timed maintenance loop flagged; sink semantics need examination. |
| [Watchdog main](i05-003.json) | Needs the engine's deadline/wait logic. |
| [Fault notification](i05-011.json) | Needs callers and their timeout/retry behavior. |

## Screening gaps

- [Activity demo](i05-001.json) returned no signal despite a loop that wakes every 100 ms to recheck a stop flag. This is a suspected missed candidate.
- [Docker startup](i05-013.json) and [graph monitoring](i05-014.json) accepted source comments claiming approved exceptions; systemd startup escalated a similar claim. Approval provenance was absent from the inputs.
- [Monitor-core](i05-012.json) returned a schema-valid response with inaccurate line references; the controller rejected it.

Coverage is limited to literal timer/wait matches: 20 selected modules, 6,696 source lines, from an inventory of 273 tracked Rust files. Unselected code and other languages remain unreviewed. Recall and precision are unmeasured; confirmation and fixes have not run.

The next design question is how to represent approved exceptions and route ambiguous no-signal responses. This pilot preserves the original decisions for evaluation.

[Summary JSON](summary.json) contains timings, usage, provenance, outcomes and manual spot-check observations. The source application remained unchanged; full CI and ARM64 validation were not run for this development-tooling pilot.
