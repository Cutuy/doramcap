# Publishing a Console release

Build each architecture natively with the Node version in `.node-version`.
Copy [console/release](../release) and replace each `:local` image with its
published digest; packaging rejects unpinned images. Template project, graph and
app input paths are relative to the playground; runtime data and access credentials
are generated locally. Build `system-monitor` with `--features bpf,task-bpf,dora`.
Publish all images to ECR Public before publishing release archives.

```sh
node console/scripts/package.mts --output /tmp/console-amd64 --version 0.1.0 \
  --node /path/to/node --node-license /path/to/node/LICENSE \
  --playground /path/to/release/playground --config /path/to/release/console.pbtxt
node console/scripts/installer.mts --base-url https://downloads.example.com/0.1.0 \
  --version 0.1.0 --amd64-sha256 AMD64_HASH --arm64-sha256 ARM64_HASH \
  --output /tmp/install.sh
```

Upload both archives and the generated installer to the versioned R2 download
prefix. The installer embeds both archive hashes; serve it over HTTPS. Publish
checksums and signatures through your release process. Account credentials stay
in publisher tooling. The CLI needs no AWS or Cloudflare account.
