# Console and open source workload tools

Reviewed: 2026-11-11. Draft for the hosted website. Competitor entries cover open source editions.

| Front | Console | [Dockge](https://github.com/louislam/dockge/tree/1.5.0) | [Komodo](https://komo.do/) | [Portainer CE](https://github.com/portainer/portainer/tree/2.45.3) | [Coolify](https://coolify.io/docs/) | [Testcontainers Node](https://node.testcontainers.org/features/compose/) |
| --- | --- | --- | --- | --- | --- | --- |
| Primary job | Recorded experiments | Compose stack operations | Builds and fleet deployment | Container administration | Application hosting | Test environments |
| Interface | Web UI, CLI | Web UI | Web UI, API, CLI | Web UI, API | Web UI, API | TypeScript library |
| Reach | Local Linux host | Multiple Docker hosts | Multiple Docker/Swarm hosts | Docker, Swarm, Kubernetes | Multiple connected servers | Configured container runtime |
| Operating model | Per-project run controller | Server with agents | Core, database, agents | Server, optional agents | Hosting control plane over SSH | Library in the test process |
| Automation | Ordered startup, capture, teardown | Compose lifecycle controls | Builds, Git sync, procedures | YAML/Git stack deployment | Git deployments, previews | Code, profiles, readiness strategies |
| Observability | Live CPU, recordings, machine facts | Logs, terminals | Host metrics, logs, terminals | Container stats, logs, terminals | Resource metrics, logs | Container logs and SDK access |
| Saved state | Run history, recordings, filesystem evidence | Compose and environment files | Resource configuration, execution history | Manager and stack backups | Database backups | Disposable containers and volumes |
| Change analysis | Two-run setup, machine, result differences | Manual inspection | Stack-file change detection | Manual inspection | Deployment history | Assertions written in test code |
| License | [PolyForm Noncommercial](../../LICENSE) | MIT | GPL-3.0 | zlib | Apache-2.0 | MIT |

Console keeps reviewed inputs, workload lifecycle, recordings, machine context,
and run comparison together. Its strength is helping teams investigate
differences between executions with the relevant evidence attached.

Console's current comparison covers captured facts and basic results. Machine
capture observes configuration; filesystem behavior follows the selected policy.
[Scope and qualification](110426-machine-capture-comparison.md).

Sources: Dockge 1.5.0, Komodo 2.3.4, Portainer CE 2.45.3, Testcontainers Node
12.2.1; Coolify's current documentation.
[Komodo stack changes](https://komo.do/docs/deploy/compose) ·
[Portainer statistics](https://docs.portainer.io/user/docker/containers/stats) ·
[Portainer backup scope](https://docs.portainer.io/faqs/getting-started/what-does-portainers-backup-include) ·
[Coolify Compose deployments](https://coolify.io/docs/applications/builds/docker-compose) ·
[Coolify backups](https://coolify.io/docs/databases/backups) ·
[Console lifecycle and filesystem policies](../../libs/workload-docker/README.md).
