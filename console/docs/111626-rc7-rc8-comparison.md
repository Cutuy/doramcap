# Console rc7 → rc8

Created 11/16/26. Follow-up to the [rc7 field report](111426-rc7-ux-report.md).
[rc8](https://github.com/Cutuy/doramcap/releases/tag/console-v0.0.1-rc8)
(`ecfcaec9`) was installed publicly and exercised through Chromium on 11/15/26,
on the same ARM64 AWS VM: 2 vCPU, 904 MiB RAM. rc7 used ROS 2 chatter;
rc8 used the bundled playground. Both enabled Data recorder. The ROS 2 CPU
comparison remains unmeasured.

Numbers match the original report's findings.

| Finding | rc7 | rc8 |
| --- | --- | --- |
| 1. Setup comparison | Commands and signals absent; shutdown fix showed “0 changed”. | `command`, `entrypoint`, `stop_signal` compared. **Gap:** `stop_grace_period` absent; changing it flagged an unrelated hidden mount. |
| 2. CPU overhead | 96–98 core %, with `talker` at 0.4%. | Playground: 111.0 core % versus plain Compose 105.3%; worker ~100% in both. Different workload. |
| 3. Readiness probes | Three system apps ran `docker exec` every 200 ms. | **Fixed:** CPU monitor, Data recorder and live bridge have no healthchecks. |
| 4. Saved choices | New run dropped Data recorder. | **Fixed:** New run, refresh and Run again retain apps; Run again restores file policy. |
| 5. Generated names | Compose prefixes visible. | **Still present** in folders and logs. |
| 6. File changes | Customer and Console files mixed. | **Improved:** User/System associations and run-file labels; shared category remains. |
| 7. Run comparison | No per-service exits or CPU. | **Still missing.** |
| 8. Machine condition | Equal-looking rounded values marked Changed. | **Fixed:** exact bytes and uptime seconds expose differences. |
| 9. MCAP indexing | Streaming reads worked; indexed reader rejected files. | **Unchanged:** both downloads streamed to their footers; neither is indexed. |
| 10. Journal noise | Warning per changed file. | **Fixed:** aggregate summaries and coverage warnings. |
| 11. Log list | System and customer logs mixed. | **Still present.** |
| 12. Release notes | One line at publication. | Substantive release notes. |
| Installation | Public install and all ten doctor checks passed. | **Issue:** `umask 077` produced `0700` release directories; launcher exited 126. Manual permission repair restored access; doctor passed. |

CPU figures are ten-second samples; 100% is one core. EC2 steal was 0.7% with
Console and 0.9% for the control. [rc8 live run](111526-playground-run.png).

## Additional rc8 coverage

| Area | Observed result |
| --- | --- |
| File policies | Keep retained a download; Write back copied to host; Discard left evidence without a download or host file. |
| Stop and restart | Normal Stop → Result in 4.31 s. Service stop wrote files back and cleaned up; restart retained History, recordings and the killed-worker warning. Old session returned HTTP 401. |
| Failure and cancellation | Exit 7 named the worker; startup Stop produced Cancelled. Reports survived and cleanup was Verified; recording was Unavailable / Unknown, respectively. |
| Input and evidence | Stale Review blocked Start; corrected YAML recovered. Preview showed 16/18 host changes; full export contained all 48 changes across categories. |

rc8 comparison now shows the command, entrypoint, signal and mount changes:

![rc8 command comparison on AWS ARM64](111526-playground-compare.png)
