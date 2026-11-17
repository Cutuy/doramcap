# Holoscan lifecycle probe

Build and run the official SDK 5 `first_pipeline` through the real Holoscan CLI,
Docker and host OverlayFS. The graph passes ten integers through three CPU
operators, requests cooperative stop and checks its termination. It needs no
recorded data, model or display.

The [Python source](first_pipeline/applications/first_pipeline/first_pipeline.py)
is unchanged from [NVIDIA's example](https://github.com/nvidia-holoscan/holoscan-sdk/blob/358c772f2d2e88d117651a961d6f6bf9b650cf8d/examples/first_pipeline/python/first_pipeline.py),
under Apache-2.0. The local metadata, CMake and Dockerfile form a test wrapper:
CMake copies the source into the native build directory; the CLI then runs that
artifact. The fixture pins the x86 CUDA base digest, SDK Core 5 wheel and CLI 5
wheel. Resolved Python dependency versions remain visible in the image build log.

## Run

Use the repository's pinned Node runtime and an isolated CLI 5.0.0 installation.
Every request selects `finite` explicitly and uses a fresh absolute output path
without whitespace, commas or colons.

```sh
node scripts/ci/holoscan/probe.mts --phase preview --mode finite \
  --cli /absolute/path/to/venv/bin/holoscan --output /tmp/holoscan-preview
```

The preview records host prerequisites, CLI version, catalog and native build/run
dry runs. It records unavailable Docker/GPU inspection as failed commands.
`preview: passed` describes the CLI previews only.

Run the capability-required test executable with sudo after the preview. Read
the password through stdin from the main checkout's ignored, mode-0600 file,
as [AGENTS.md](../../../AGENTS.md#capability-required-tests) specifies:

```sh
sudo -S -p '' /absolute/path/to/pinned/node scripts/ci/holoscan/probe.mts \
  --phase real --mode finite --cli /absolute/path/to/venv/bin/holoscan \
  --output /tmp/holoscan-real < /absolute/path/to/main/.sudo-password
```

The real phase builds the small image and application, then runs a plain CLI
baseline, the same build artifact through an overlay root, a read from an extra
folder whose overlay retains its name, and an Engine stop. Overlay launches add
one CPU, ownership/cidfile options and journald logging. Containers use the native
NVIDIA runtime; the example itself uses host-memory integer operations.

The test records raw command logs, container inspections, Engine event JSONL,
Engine log tails and host journal output in the output directory. `report.json`
keeps execution failures and cleanup facts separately. A failed assumption stops
the scenarios. Cleanup removes this invocation's containers and unmounts its
host overlays, including after interruption. The uniquely named test image and
build cache remain available for inspection; remove that exact image when done.

This qualifies lightweight lifecycle mechanics. The two application builds,
recorded replay, GPU kernels and recovery acceptance in PR #264 remain separate
qualification work.
