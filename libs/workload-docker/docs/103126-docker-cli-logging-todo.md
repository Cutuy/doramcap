# Docker CLI logging

Production Compose calls use `compose::command::compose` and log their lifecycle.

- [ ] Route direct Docker calls in test/dev harnesses through a shared logged
  runner, including `tests/docker.rs`, `tests/shared_ipc.rs` and `scripts/compose-e2e.py`.
