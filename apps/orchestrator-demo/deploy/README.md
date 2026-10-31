# Orchestrator Lab hosting

Run the web server unprivileged and the fixed Docker worker on the host, optionally
with the capabilities needed by BPF and OverlayFS. Both use the existing socket protocol.

```text
worker SOCKET IMAGE ARTIFACT_DIRECTORY REPOSITORY_ROOT
```

The worker uses the shared experiment workflow, including pre-run reset. It shares
repository admission with other callers. Web hosting owns no experiment slot.
A supported native hosting launcher remains separate work.
