# NAS API and Desk OCR

Long Live Health runs from prebuilt Docker images on two hosts:

| Component | Host | Deployment configuration | Persistent data |
| --- | --- | --- | --- |
| API and embedded web UI | NAS, `192.168.1.10` | `/data/docker/2_home_service.yaml`, service `longlivehealth` | `/data/docker/data/longlivehealth` mounted at `/data` |
| GPU OCR | Desk, `192.168.1.20` | `docker/docker-compose.ocr-desk.yaml`, project `longlivehealth` | `playground/backend_ocr` mounted read-only at `/data` |

The client address is **https://health.zhuoyin.info**. The NAS's existing Caddy
discovers the service through its `caddy.0.domain` and `caddy.0.port=8080`
labels on `download_network`. The API has no published host port and no GPU,
source-code, binary, or model mount. Its runtime image includes the web UI and
required system libraries. NAS does not build the image.

The API's owner-readable `/data/config.toml` sets `[ocr].url` to
`http://192.168.1.20:8001`. Its embedded OCR API key must match the embedded key
in Desk's OCR `config.toml`. Do not copy credentials into Compose, environment
variables, Git, or images. OCR's authenticated HTTP listener is published only
on Desk's LAN address; its inference engine remains inside the container.
Cross-host LAN transport is intentional for this deployment; the single-host
playground continues to use its internal Docker network.

## Deploy images

Build on Desk through `./build_docker.sh`, which runs the image checks. Tag the
tested API image with an immutable release name, transfer it with `docker save`
and `docker load`, and set that exact tag in NAS's Home Service YAML. Both
deployment files use `pull_policy: never`; loading the image is required before
starting the service. Compare `docker image inspect --format '{{.Id}}'` on both
hosts to verify the transferred image. The current NAS release uses
`longlivehealth-backend-api:metrics37`.

On Desk, from the repository root:

```sh
docker compose -p longlivehealth -f docker/docker-compose.ocr-desk.yaml up -d --no-build backend_ocr
```

On NAS:

```sh
cd /data/docker
docker compose -f 2_home_service.yaml config --quiet
docker compose -f 2_home_service.yaml up -d --no-build --no-deps longlivehealth
docker exec --user 1002:1003 longlivehealth /app/longlivehealth --data-dir /data check-config
docker exec --user 1002:1003 longlivehealth /app/longlivehealth --data-dir /data probe-providers
```

NAS uses `env_file: ./lsio.env`, matching the other Home Service containers.
The shared file supplies PUID 1002, PGID 1003, UMASK 002 and TZ US/Eastern.
There is no per-service environment block or fixed `user` override. The image's
entrypoint preserves data ownership and drops privileges to PUID/PGID before
starting the API. Desk OCR continues to use UID/GID 1000.

Leave `RESET_ADMIN_PASSWORD` unset for normal operation; its default is false.
Explicit `false` also preserves the existing administrator password. Only the
exact value `true` resets it to `admin`, requires an immediate password change,
and revokes existing administrator sessions. Any other value, including an empty
string, `TRUE`, or `1`, fails startup without resetting credentials. Remove an
intentional `true` override after recovery: leaving it set resets the password on
every start. A fresh database still creates the initial admin/admin account.

The disposable image regression check exercises unset, false, invalid values and
true, including changed-password preservation and session revocation:

```sh
python3 tests/docker/admin_reset.py --image longlivehealth-backend-api:identity24
```

## Move an existing archive

1. Pre-copy immutable raw files and configuration while the old API is running.
   Exclude the live SQLite database, WAL/SHM, lock, and temporary directory.
2. Stop the old API and disable its automatic restart. Complete a final copy of
   the **entire** data root while no process can write to it. Copy any WAL with
   its database; never copy a live SQLite database independently of its WAL.
3. Confirm copying completed and check SQLite integrity before starting the destination.
   Ordinary rsync validates transferred file contents; a separate full checksum
   scan of every raw file is not required for this deployment.
   Preserve accounts, password hashes, refresh sessions, report relationships,
   health anchors, and `raw/<user>/<source>/...` paths without reimporting data.
4. Change only the destination configuration's OCR URL. Start Desk's OCR-only
   deployment, then the NAS API. Check authenticated OCR reachability, API
   readiness, existing record counts, and HTTPS through Caddy.
5. Remove the old API container and the verified old data copy only after the
   destination passes verification. Never run both APIs against divergent
   writable copies. A rollback after new writes requires moving the current
   NAS data back while both APIs are stopped.

The iOS app stores its server address. After cutover, select
`https://health.zhuoyin.info` and sign in with the existing account. A new server
origin may require a fresh login even though the migrated accounts are unchanged.

## Initial migration validation — 2026-10-20

- Transferred the tested API image without rebuilding on NAS; both hosts reported
  `sha256:c6c3ddc2424e101ebab6a32e1a5f0decf011fd3599af511a034993203d788c62`.
- Copied 4,054,777 raw originals, then stopped Desk API cleanly and completed an
  ordinary incremental copy of the entire data directory. The additional full
  raw-content checksum scan was stopped at the operator's request.
- SQLite's full integrity check returned `ok`. Before starting NAS API, both
  database files had SHA-256
  `05293b97c0db1e859e40d02a5d9503ce41f43e7d66ed99e71246ac2bd719e0cd`.
- The destination retained 4,063,978 health records, 4,054,777 original references,
  one user and one session, matching the stopped source database.
- HTTPS returned 200 with the Long Live Health page; a protected API request
  without credentials returned 401. The API ran as UID/GID 1000:1003 with no
  container restarts.
- NAS reached Desk OCR at port 8001. An unauthenticated extraction request
  returned 401; an authenticated malformed request returned 422. Provider probe
  succeeded. These checks validate connectivity and authentication, not model
  inference accuracy; they do not load the GPU model.
- After NAS readiness was confirmed, removed the stopped Desk API container and
  its old `playground/backend_api` data directory. Desk retains only the OCR
  runtime for this deployment; NAS is the authoritative API data store.

## Identity replacement — 2026-10-20

The identity replacement deployed `longlivehealth-backend-api:identity24`, image
`sha256:3954a76a4b416ef90dbc470ca17e70c605a20cf3a466b4385bf3740c7de657e7`,
verified identical on Desk and NAS. HTTPS and the authenticated OCR provider
probe passed; host port bindings remain empty. The admin-reset image regression
passed all six cases. Desk OCR mounts the renamed repository's
`/home/zyin/dev/longlivehealth/playground/backend_ocr` directory.

Before deployment, read-only database checks found zero manual health records,
zero old-brand source IDs, zero exports, and only `apple_health` raw-source
directories. No stored rows or original files needed renaming. Existing accounts
and health records were preserved. The new iOS identity signs in to the same NAS
account; its first Health sync obtains new local anchors and the API deduplicates
records using their existing Health identities.

## Interval step totals — 2026-10-21

Built `longlivehealth-backend-api:overview29` through the root Docker entrypoint,
including formatting, clippy, and 153 passing backend tests (two existing ignored
tests). Desk and NAS image IDs match:
`sha256:6697ea0de69eac5f74182c7e62ebd2f503399aa4aa18da4ef1875c57c9955a77`.
This release adds overlap-only device priority for combined daily step totals;
per-source chart values remain available. Older daily-view caches recompute on
read. NAS Compose uses this image tag; data mounts and the shared environment
file are unchanged.

After startup, the public HTTPS server-status endpoint returned `ok`; configuration
validation and the authenticated Desk OCR availability probe passed. Build 29
passed 30 Swift core tests and the focused Browse/Overview UI regression, and was
signed and installed wirelessly on the paired iPhone.

## Daily summary admission — 2026-10-21

NAS logs showed `wellness/day` returning 429 during concurrent Health sync.
Aggregation admitted every CPU chunk with a fail-fast shared semaphore, discarding
the summary on transient contention. The `dailyqueue30` API image adds bounded
async waiting for these chunks (four waiters, ten-second maximum); uploads keep
their existing admission behavior. All 154 backend tests passed, including busy
CPU, queue-bound and cancellation coverage. Desk/NAS image IDs match:
`sha256:927663771c769a5e380a1a6ec396cfe5b4f661c47e9fdff054ac35a6864e6baa`.
The iOS Build 30 client requires no update for this server fix.

After startup, public HTTPS server status returned HTTP 200 / `ok`. Existing
startup ownership adjustment and original-index recovery took several minutes.

### Browse performance update — Build 31

The `browse31` image includes the clinical-history index selection fix and unified
device-series response (all sources, manual weight, server-normalized deduplicated
chart points, full-history VO₂ Max). The rollout changes only the API image.
Data volumes, the Desk OCR service, Caddy routing and environment settings stay intact.

The `storage31` follow-up removes recursive ownership checks/changes at startup.
File permission and read-only storage failures are reported by the API when the
operation occurs, with messages displayed by iOS. Original-index recovery is a
separate startup step; removing permission traversal does not remove that recovery.

Validation: 156 backend tests passed (2 pre-existing ignored), disposable-volume
permission/identity check passed, and the deployed public status endpoint returned
200. This restart reached readiness in about 29 seconds, without recursive chown;
original-index recovery still runs. Build 31 was verified installed on the paired iPhone.

### Heart detail and synchronization — Build 32

`heartsamples32` fixes Apple source-name-only 409s without creating new original
revisions for unchanged measurements. It adds compact heart subsample projections
and all-period min/max display ranges. The paired iPhone runs Build 32. Historical
subsamples are rebuilt by `index-heart-samples`, in batches of 100 projections,
without changing originals. Numeric Browse ranges preserve source/unit separation.

### Daily history and calorie charts — Build 37

The deployed `metrics37` image is
`sha256:2e88b1973d6a2d0814c8bac7693ecfeee913175565bd27f7fd3060eabd77e5b6`,
verified identical on Desk and NAS. Daily chart units no longer depend on today's
selected source or available samples. Active/resting energy use source-separated
kcal totals with midnight proration. Daily caches recompute under wellness-day-v5.
The Docker build passed 163 backend tests (two ignored); public HTTPS readiness
returned `ok`. No original health records were changed by this display fix.
