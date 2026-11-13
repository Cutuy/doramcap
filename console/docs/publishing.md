# Publishing a Console release

Push a `console-v0.1.0` tag. The [release workflow](../../.github/workflows/console-release.yml)
builds on x86_64 and ARM64 runners, then:

1. Pushes images to `ghcr.io/cutuy/doramcap` and joins them into multi-arch digests.
2. Pins those digests into a copy of [console/release](../release).
3. Packages one archive per architecture. Packaging runs without registry credentials
   and rejects any image users cannot pull for both architectures.
4. Generates `install.sh` with both archive hashes and publishes the GitHub release as Latest.
5. Points each image's `latest` tag at that release's verified multi-arch digest.

GHCR packages must be public. Enable immutable releases in the repository settings.
