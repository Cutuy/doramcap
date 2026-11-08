# Tooling

Shared TypeScript helpers for repository tools. Run with the pinned Node runtime;
install the locked dependencies with `npm --prefix libs/tooling ci --ignore-scripts`.

| Module | Contract |
| --- | --- |
| [CLI](cli.mts) | Commander arguments/help and failure reporting after cleanup. |
| [Processes](process.mts) | Execa argv execution, inherited or complete captured output, explicit failures. |
| [Filesystem](fs.mts) | SHA256, canonical paths and owned temporary directories; Node built-ins only. |
| [Globs](glob.mts) | Case-sensitive minimatch groups including dotfiles. |
| [Archives](archive.mts) | Stream exact bytes and modes through tar-stream and gzip. |

```ts
import { capture, run } from './libs/tooling/process.mts';

const revision = capture('git', ['rev-parse', 'HEAD']).stdout.trim();
run('git', ['show', '--stat', revision]);
```

CI snapshot policy stays in [scripts/ci](../../scripts/ci/README.md). Shared modules
use explicit imports so worker preparation can run the filesystem helpers before
installing npm dependencies. No build step is required.

Run `npm --prefix libs/tooling test` and `npm --prefix libs/tooling run typecheck`.
