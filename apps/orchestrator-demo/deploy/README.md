# Native hosting

Run the unprivileged web server and a separately authorized host worker. The worker
accepts `SOCKET IMAGE ARTIFACT_DIRECTORY REPOSITORY_ROOT`; its OverlayFS mounts must
share the Docker daemon's mount namespace. Await experiment cleanup before stopping it.
