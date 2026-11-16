# Console rc7 vs rc8

Created 11/14/26; updated 11/16/26. Public ARM64 releases tested through the SSH
tunnel with Chromium on the same Ubuntu 24.04 AWS VM: 2 vCPU, 904 MiB RAM, no swap.
[rc7](https://github.com/Cutuy/doramcap/releases/tag/console-v0.0.1-rc7) used ROS 2
Jazzy chatter, a 200 Hz `/scan` publisher and rosbag2;
[rc8](https://github.com/Cutuy/doramcap/releases/tag/console-v0.0.1-rc8)
(`ecfcaec9`, tested 11/15/26) used the bundled playground. Both enabled Data recorder.
CPU figures cover different workloads; rc7's ROS 2 overhead has not been remeasured.

| Area | rc7 | rc8 |
| --- | --- | --- |
| Installation | Public installer completed in 3.4 s; all ten `doctor` checks passed. | **Permission issue:** `umask 077` left release directories at `0700`; normal-user launcher exited 126. Permissions repaired manually, then all ten checks passed. |
| 1. Setup comparison | Ignored `command`, `entrypoint` and `stop_signal`; the ROS shutdown fix showed “0 changed”. | **Improved:** all three fields appear. `stop_grace_period` is still missing; changing 1 s → 5 s instead flagged an unrelated hidden live-bridge mount. |
| 2. CPU cost | Host used 96–98% of 200% capacity while `talker` used 0.4%. | **Playground sample:** host busy CPU 111.0% versus plain Compose 105.3%; worker ~100% in both. Ten-second samples; EC2 steal 0.7% / 0.9%, respectively. |
| 3. Readiness probes | CPU monitor, Data recorder and live bridge ran `docker exec` healthchecks every 200 ms throughout the run. | **Fixed:** Docker inspection found no healthchecks on these apps. |
| 4. Saved app choices | New run dropped Data recorder; Run again retained it. | **Fixed:** New run, refresh and Run again retained the expected apps; Run again also restored file policy. |
| 5. Generated names | Compose prefixes leaked into writable folders and logs. | **Still present.** |
| 6. File changes | Mixed customer writes, Console run files and system-app layers; exposed hundreds of ROS Python bytecode writes. | **Improved:** User/System associations and run-file labels are visible; system records share the category. Bounded preview showed 16/18 host changes; full export had all 48 changes across categories and a completion record. |
| 7. Run comparison | No per-service exits or CPU; killed and clean runs differed only by duration and recording size. | **Still present.** |
| 8. Machine condition | Rounded disk space and uptime looked equal but were marked Changed. | **Fixed:** small disk differences use exact bytes; uptime uses seconds. |
| 9. Recordings | Both Console MCAPs decoded with a streaming reader; indexed reader rejected them. Customer rosbag2 MCAP wrote back successfully. | **Still index-free:** independent `@mcap/core` streaming read reached both footers (480 / 954 messages); indexed reader rejected both. See [writer design](../../libs/storage-persistence/docs/101826-mcap-writer-design.md). |
| 10. Journal noise | Logged every file change at warning level. | **Fixed:** aggregate filesystem summaries and coverage warnings replace per-file warnings. |
| 11. Log list | Mixed system apps and customer services in start order. | **Still present.** |
| 12. Release notes | One-line note at release; expanded after publishing. | Substantive notes accompanied the installed ARM64 release. |
| File policies | `./bags` write-back delivered the customer's rosbag2 recording. | **Verified:** Keep retained a downloadable file without host writes; Write back copied it to the host; Discard preserved change evidence without a download or host file. |
| Stop and restart | Killed-service banner exposed `ros2 run` losing signals; direct node binary exited 0 in 0.28 s. Cleanup Verified, no containers left. | Stop → Result in 4.31 s, Completed / Saved / cleanup Verified. Service stop wrote files back and cleaned up; restart preserved History, recordings and exit-137 warning. Old session returned HTTP 401. |
| Failure and cancellation | Not assessed in this report. | Exit 7 named the worker; startup Stop produced Cancelled. Both retained reports and verified cleanup. Recording was Unavailable / Unknown, respectively; cancelled file changes were explicitly unavailable. |
| Input errors | Malformed YAML stopped at Review with Compose's parse error. | Stale Review blocked Start; malformed YAML stopped at Review, and corrected input recovered. |

Additional rc8 coverage: Fault Watchdog connected four sources with no active faults,
drops or overflow. The local SPA used sample data; run/Stop, saved apps, History deep
links, downloads and 390 px mobile History worked. Its recording decoded fully.

Evidence: [rc7 setup](111426-rc7-ux-report.svg), [Review](111426-rc7-review.png),
[killed-service result](111426-rc7-result.png), [live CPU](111426-rc7-run.png),
[comparison](111426-rc7-compare.png); rc8 [AWS comparison](111526-playground-compare.png)
and [AWS live run](111526-playground-run.png).
