# Gallery hosting

The gallery manages one Compose generation per app through the shared Rust
workflow. Viewers share that generation; it stops when their sessions expire.
Each app has a separate repository scope; different apps may run concurrently.
Docker Engine access and independently built app images are required.

## Start

From the checkout:

```sh
npm ci --prefix apps/project-explorer/frontend
npm run build --prefix apps/project-explorer/frontend
cargo build --locked -p project-explorer-server --bins
install -d -m 700 /absolute/path/gallery-data
target/debug/project-explorer-server \
  --workspace "$PWD" --data /absolute/path/gallery-data \
  --assets "$PWD/apps/project-explorer/frontend/dist" \
  --origin http://127.0.0.1:8096
```

Run Caddy in another terminal. Host networking lets it reach the manager and app
containers on Linux:

```sh
docker run --rm --name doramcap-gallery-caddy --network host --read-only \
  --cap-drop ALL --cap-add NET_BIND_SERVICE --tmpfs /config --tmpfs /data \
  -v "$PWD/apps/project-explorer/server/caddy:/etc/caddy:ro" \
  caddy:2 caddy run --config /etc/caddy/Caddyfile
```

Open <http://127.0.0.1:8096/#/apps>.

## Configuration

Registrations live in `apps/*/deploy/registration.pbtxt` and load at startup.
Use `--enable-app ID` for opt-in registrations. App build instructions and host
requirements belong in each app's `deploy/` directory; see the
[registration schema](../proto/registration.proto) and
[app deployment guide](docker-hostable-apps.md).

Viewer heartbeats default to 15 seconds, with 120-second expiry. Configure these
with `--heartbeat-seconds` and `--idle-seconds`; heartbeat must be shorter than
expiry. Browsing the gallery alone does not keep an app running.

Caddy uses `GALLERY_BIND` (address and port), `GALLERY_LISTEN` (interface) and
`GALLERY_API` (manager address). For Tailscale access, set the first two to the
host's Tailscale address and match `--origin` to the public URL. Restrict access
through your Tailscale policy: permitted peers can launch enabled apps.

For a URL prefix, set `GALLERY_PREFIX=/gallery` on Caddy and use
`/etc/caddy/Caddyfile.prefix`. Open `/gallery/#/apps`; `--origin` contains only
scheme and host, including port.

## Restarting after interruption

Keep the data directory private and persistent. Finalized outputs, logs and
recording status remain there; only declared results are downloadable.

Each app is a separate repository scope. Its next launch stops and verifies old
writers, discards leftover overlay data, and removes owned containers and networks
before preparing a new run. Unexpected reset failures block that launch. Other
apps may run concurrently. Save any old upper data you need before launching again.

## Public HTTPS

Use [Caddyfile.public](../caddy/Caddyfile.public), replacing its example hostname
in both blocks. Set the manager's `--origin` to that hostname's HTTPS origin.
Point its DNS A record at the VM and allow inbound TCP 80/443. Keep the manager
on loopback; mount persistent, private Caddy `/data` and `/config` directories so
certificates survive restarts. The private loopback entry redirects to HTTPS.

The public template serves anonymous viewers and preserves the gallery's existing
app concurrency, viewer leases and operator-managed result retention.
