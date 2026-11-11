![Run log capture options](111126-run-log-capture-options.svg)

# Run log capture: architecture options

Created November 11, 2026. Decision pending. PR #314 remains draft.

The choice is who owns application output before it becomes a run file: the host
journal or the run's workflow. Resolve the capture contract before selecting a
rate-limit setting or changing installation.

This reopens the journal decision in [the existing design](111026-run-logs.md).
That document retains the previous agreement and the measured results. This
options document proposes no host changes or implementation plan.

## Behavior to preserve

- Preserve explicit project logging, including empty blocks. Review identifies
  project-managed logs and their exclusion from Console collection.
- Associate captured stdout and stderr with one run and one program.
- Keep logs with the run's files once #316 milestone 4 provides that storage.
- Preserve available output on failure and finish capture before deleting its
  source. Report capture and storage failures separately from program failure.

The decision is whether Console promises the output still available from the
host journal, or captures application output within the run's storage budget.
The latter requires explicit behavior for storage exhaustion and recovery.

## Measured facts

The default journal path retained all sampled low-rate and burst messages. Under
saturation, a quiet container retained 5/60 lines per stream in one run and 14/60
in a repeat. All quiet markers after the noisy exit were missing in both runs.
Both containers' entries identify the same Docker service and writer.

On the tested systemd 255 host, per-service zero rate-limit values inherited the
global defaults. An isolated Docker daemon confirmed that this setting still
lost output. Disabling the journal's default rate limit changes host policy.
The [qualification record](111026-run-logs.md#journal-load-qualification) contains
counts, platform versions and primary source links.

## Options

| Option | Capture behavior | Host changes | Main tradeoff |
| --- | --- | --- | --- |
| **A. Keep the host journal as configured** | Export entries that survive rate limits and retention. | None. | Smallest change; a noisy container can cost other containers their output. |
| **B. Disable the journal's default rate limit** | Keep the existing journal/export architecture and remove default rate suppression. | A journal configuration change affecting the host; explicit per-unit limits still apply. | Small code change with broad operational scope. Retention and storage failures remain. |
| **C. Let each workflow capture into run files** | Capture application output through native Docker and systemd facilities, then retain it with the run. | None required for journal policy. | More lifecycle work: start capture, drain it, handle storage failure and recover before cleanup. |

A and B retain the previously agreed journal architecture. C changes that
decision and needs a separate implementation plan. For C, Docker supports
[following container stdout/stderr](https://docs.docker.com/reference/cli/docker/container/logs/),
and systemd supports
[file destinations for service output](https://www.freedesktop.org/software/systemd/man/255/systemd.exec.html#StandardOutput=).
Those are platform capabilities; C's end-to-end capture behavior is unqualified.
The plan must address Docker rotation, buffering, controller restart and disk
exhaustion before claiming complete capture.

## Responsibilities and lifetime

- **Workflow:** identifies each program, owns capture and drains output before
  removing containers or other log sources. It preserves partial evidence when
  capture or cleanup fails.
- **Capture source:** the host journal in A/B; backend-native output handling in
  C. Explicit project logging retains project ownership in every option.
- **Run storage:** owns saved files and their retention under #316. Saving files
  does not restore output already dropped by a source.
- **Console:** shows ownership in Review and saved files with the run. The
  selected architecture must define how incomplete capture is reported.

## Recommendation and decision

If preserving application output is a must-have and host policy should stay
outside Console, **choose C** and plan that work with run-file retention. It puts
capture, failure reporting and cleanup under the same workflow owner.

Choose A if best-effort journal output is the intended product contract. Choose B
if journal unification matters enough to accept a host-wide policy requirement.

First decide the capture contract and acceptable host scope. Implementation
details, configuration changes and any new dependencies wait for that choice.

Applicable rules: D-01/D-02 (behavior and simplicity), D-03/D-06 (ownership and
lifetime), D-08 (retention), D-11/D-12 (capacity and host scope), and I-06/D-13
(explicit limits and measured evidence).
