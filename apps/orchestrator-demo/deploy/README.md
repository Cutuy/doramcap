# Orchestrator Lab hosting

The intended native hosting model runs the web server unprivileged and a separate,
optionally elevated Docker worker. A supported native launcher is not currently provided.

For gallery integration, prefer those two host processes with shared singleton,
idle-stop and proxy handling. The worker already invokes the shared Compose
workflow for experiments. Keeping it on the host gives its OverlayFS mounts the
same namespace as the Docker daemon. Shutdown must let the active experiment
finish cleanup before terminating the worker.

The Compose profile here is pending and has no active gallery registration. Its
containerized worker needs additional mount-namespace/capability qualification.
A native-process gallery adapter belongs in the hosting PR; these assets alone
do not enable it.
