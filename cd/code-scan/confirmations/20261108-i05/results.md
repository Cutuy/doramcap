# I-05 candidate confirmation

Sol/high reviewed the three screening candidates at source revision `65fe14329cc16ce534cc07d1bf55c6a7a2b83272`, using native JSON Schema output and at most two concurrent CLI calls.

| Candidate | Outcome | Evidence |
| --- | --- | --- |
| Watchdog MCAP sink | Confirmed violation | Maintenance timeouts call nonblocking I/O completion progress independently of flush deadlines. [Decision](i05-004.r2.json) |
| Dataflow demo timer | Refuted | Timers schedule sampling, telemetry, delayed delivery and run expiry; arrival and shutdown have notification branches. [Decision](i05-002.r1.json) |
| Systemd Dora readiness | Unresolved | Polling is present, but the claimed approval could not be verified. [Decision](i05-020.r2.json) |

- Five model calls across two rounds; peak concurrency two. Sum of batch wall times: 96.3 seconds, excluding context preparation and publication.
- Tokens: 139,018 input, 0 cached input, 6,705 output. Reported reasoning tokens: 1,907.
- Standard API-equivalent estimate: **$0.3451** for confirmation; **$0.3717** including screening. [Rates](https://developers.openai.com/api/docs/models/gpt-6.1-sol). Actual billed cost and served model identity were not reported.
- All five responses pass native schema and exact source-span validation. Source artifacts match the frozen Git revision. The quote validator now accepts exact multiline spans; the initial response was revalidated without a model retry.
- The controller supplied requested storage internals and a bounded approval-history search. Systemd is parked for missing exception evidence; repeating the request would not add information.

The MCAP finding is confirmed from static behavior and contracts. Runtime impact was not measured. One of three candidates survived confirmation; the unresolved case prevents a final precision figure. No fixer or production tests ran, and production code remains unchanged.

[Summary JSON](summary.json) contains usage, timing, provenance, verdict routing and validation. Initial context was selected by the controller; context requests were resolved through explicit JSON follow-ups.
