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

Android development packages are served at `https://health.zhuoyin.info/longlivehealth.apk`.
The APK is packaged in the API image by the combined Docker build.

## Deploy images

Build on Desk through `./build_docker.sh`, which runs the image checks. Tag the
tested API image with an immutable release name, transfer it with `docker save`
and `docker load`, and set that exact tag in NAS's Home Service YAML. Both
deployment files use `pull_policy: never`; loading the image is required before
starting the service. Compare `docker image inspect --format '{{.Id}}'` on both
hosts to verify the transferred image. The current NAS release uses
`longlivehealth-backend-api:recovery-hrv-20261029`.

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

### Browse query audit — 2026-10-22

`browsecomplete39` fixes the daily SQL projection dropping kcal, and generates
range data for both standard and hospital-specific lab histories. Decimal-text
values are converted for display without rewriting originals. Exact unitless
hospital values (including numeric valueString) use a separate “Not reported”
unit series; qualitative and comparator results remain historical rows.
Daily cache version is wellness-day-v6. Docker validation passed 165 backend
tests (two existing ignored), including persisted daily records through SQL and
clinical chart ranges. This backend release does not require an iOS update.

The operator diagnostic runs the same Browse query methods sequentially and
prints counts rather than clinical values. It includes all catalog lab items:

```sh
docker exec --user 1002:1003 longlivehealth /app/longlivehealth --data-dir /data audit-browse --username admin --date 2026-10-22 --timezone America/New_York
```

Choose the audit date explicitly. Daily history covers 43 days; measurements use
30 days except VO₂ Max's complete history, matching the current iOS requests.
A zero chart count must be compared with stored records, query errors and
non-numeric historical results; it does not by itself mean no data exists.

### Complete daily history — Build 39

`sleephistory39` adds the authenticated full daily-history query used by Browse.
The real sleep audit returned 1,487 day/source chart groups across six sources,
with original records starting in 2017 and valid asleep groups starting in 2020.
Older in-bed-only records are retained without being interpreted as sleep.
Backend validation passed 166 tests (two existing ignored).

The current Browse period-query image is `longlivehealth-backend-api:period40b`. Its Docker build runs formatting, Clippy, and the backend tests. The `audit-browse` operator command checks full daily histories, oldest/latest measurement windows for all six periods, sleep sessions, and each laboratory metric without printing health values.

Report identity deployment: NAS API `longlivehealth-backend-api:import41`, Desk OCR `longlivehealth-backend-ocr:import41`. Desk compose project name is explicitly `longlivehealth` with `docker/docker-compose.ocr-desk.yaml`. A synthetic PNG was processed by the real Qwen3.8 engine and correctly returned the report title and Function Health source. Service keys remain in each service's existing TOML; no key is printed in validation logs.

PDF extraction hotfix `pdfpin41`: OCR tasks hold a temporary hard link to their processing original for the entire multi-page run. Automatic source naming or a user source correction may relocate the archive; cleanup of its old path cannot invalidate subsequent rendering. A three-page regression test explicitly removes the old path after page 1 and verifies all pages finish. Task failures now log the job ID and safe error category.


Report marker mapping deployment: NAS API `longlivehealth-backend-api:markers43`, image `sha256:3cf5067d2e03e299377fa8fb0d22f0cee0519fdcfebcca3bae00f5721459fb21`. The 118-item report catalog derives `resolved_metric_id` for existing and new observations without mutating stored evidence. The current imported report's 126 name/unit pairs all resolve. Docker formatting, Clippy and 172 backend tests passed (2 existing ignored); public server status returned OK after deployment. The GPU OCR image remains `import41`.

## Report drafts — 2026-10-22

The `drafts44` API release stores pending/rejected laboratory observations in
`/data/docker/data/longlivehealth/draft.sqlite`. The main `database.sqlite`
observation table contains confirmed results. PDF originals, report source/title,
OCR outputs and task state retain their existing storage. Startup relocates existing
unconfirmed observations; it does not scan original Health files or re-run OCR.

Keep both databases in backups. Drafts survive restarts. Confirmation commits the
main copy before removing the older draft; interrupted cleanup is retried at startup.
Export/restore also preserves drafts. The iOS Build 44 Confirm all button is supported
without another app update.

Deployment verified: API image `sha256:f0e1d077529029bd05745fbcdb3200f4a5ceb92feebf2c1cb2d7563169621361`
matched on Desk and NAS. All 126 existing pending observations moved to the draft
store with identical full-row content digests; the official observation table had
zero rows. Public `POST /api/v1/server/status` returned `ok`. Validation: 174 backend
tests passed (2 existing ignored), 13 archive tests passed, Docker fmt/clippy/build
passed. Desk OCR was not redeployed. Evidence is in `build/ios/validation/drafts-*`.

## Browse identity and administrative-field filtering — 2026-10-22

Image `longlivehealth-backend-api:browse49` (`sha256:b2f28e36c3f3dac36e515e3bf32a2279f9b51205fda053a0ba09e3c139ba7147`) is deployed through the existing home-service Compose service. The OCR container is unchanged: filtering is enforced in the API before OCR candidates enter draft.sqlite and in review validation, not merely through an OCR prompt.

The clinical projection excludes specimen-expiration/status fields by reviewed LOINC codes and exact normalized labels. Original FHIR/PDF evidence is retained. Thirty additional LOINC codes map to the same existing IDs used by report results, preserving source attribution, nonnumeric/bounded results, missing-unit history, and duplicate evidence. Different vitamin-D forms and historical eGFR formulas retain separate, unambiguous titles.

Audited 112 distinct clinical name/code/unit combinations (308 records). Fourteen workflow records came from Apple Health, not the PDF report. Existing main/draft report candidates contained no administrative fields. Deployed Browse has 150 laboratory items and no case-insensitive duplicate names. Details: `docs/validation/browse49-audit.md`.

The operator command now supports `audit-browse --catalog-only` (with its usual username/date/timezone arguments) for a fast value-free catalog check. Docker fmt, Clippy and tests passed: 178 passed, 2 existing ignored. Public server status returned HTTP 200 after startup; startup still takes about 34 seconds for existing archive-orphan recovery.


## Apple Health append archives — schema 8

Prepare and test the new image before stopping the API. Stop the API and run
`python3 src/archive/pack_health_raw.py --data-dir /absolute/data/root` with the
service UID/GID. The migration uses `server.lock` to refuse a running server.
It converts schema 7 to 8 without changing accounts, projections, record IDs,
connections, sync batches, coverage, or iPhone anchors. No resync is needed.

The resumable staging index is committed only after category bytes are durable.
Copies use bounded buffers and preserve JSON byte-for-byte; there is no extra
full-file checksum pass. Final index replacement and schema publication are one
SQLite transaction. Old per-record Apple JSON files are deleted only afterward.
An interrupted cleanup resumes by running the same command. Do not restart the
old image during an incomplete migration: its orphan cleanup does not know the
staging archive files. Keep the API stopped and rerun the migrator instead.
After completion start only the schema-8 image, verify counts, coverage and a
sample frame download, then verify public HTTPS readiness.

Frames contain `[8-byte little-endian JSON byte length][JSON bytes]`. Category
names outside the normalized filename alphabet use `encoded-<SHA256>.frames`.
`health_originals.raw_offset` points to the length prefix; `raw_length` excludes
it. Standalone non-Apple originals use NULL for both columns. Ordinary chart
queries still read SQLite. Raw downloads and maintenance seek to indexed frames;
exports copy each category's committed snapshot prefix once. Apple deletions
retain old raw frames and append tombstones; deleted values stay out of charts.


The 2026-10-22 cutover converted 4,090,902 originals into 79 category files,
retaining all 4,079,886 active records. NAS served the new image over HTTPS after
per-category frame checks. Remaining duplicate JSON files were isolated outside
`/data/docker/data/longlivehealth` in the private temporary directory
`/data/docker/data/longlivehealth-retired-raw58` for background deletion; this
folder is not part of the live archive or its Docker mount. The one-off cleanup
container removes it after completion. No iOS anchor reset or App update is needed.

2026-10-25: API `ocrvalidation66` returns all confirmed report history and per-item OCR validation errors. Desk OCR `reportdates65` uses laboratory-page-v4; a real-page comparison recovered all 11 printed collection dates, versus 0/11 with v3. Backend validation passed 186 tests (2 ignored). iOS Build 61 includes history placeholders and explicit OCR failure details.

2026-10-28: API `healthunlimited67` and iOS Build 62 remove Health series, individual upload payload and raw read byte ceilings. Verified single-record 41 MiB upload/replay/readback, 186 backend tests, 41 Swift core tests and 27 native tests.

## Apple Health Browse coverage and Android delivery — 2026-10-29

Current image: `longlivehealth-backend-api:apple-browse-apk-20261029`. It includes the shared 221-type Apple Health Browse catalog and the Android APK. Public download: `https://health.zhuoyin.info/longlivehealth.apk`; no APK volume is mounted. Verified served SHA-256: `8446480daad40e2262eb258b3c9968f911b7e273dd12f723a469f63d9f444dbc`. Fresh Android login fields are blank.

`index-apple-quantities` repaired 576,928 historical projections in bounded batches with optimistic row checks. Original raw frames and sync identities were preserved. See [the per-type audit](../validation/apple-health-browse-audit.md).

## ARM64 Android Build 3 — 2026-10-29

Current API image: `longlivehealth-backend-api:android-jni-20261029`. The fixed `/longlivehealth.apk` download is now ARM64-only. Its SHA-256 is `556889b39812c9f69352cdd1661bc084f0b36612aa113e90bb30e689e0dddcaf`. The app build checks every native DEX method against the packaged ARM64 library before publication. [Crash investigation](../validation/android-browse-crash.md).

## 2026-11-05 unified shared inference

Current architecture: NAS runs only the Long Live Health backend_api business service. Its private `/data/config.toml` has one `[llm]` endpoint/model/key for both built-in Rust OCR and Analytics. Desk local_llm owns Strata; no Long Live Health OCR container, model image, GPU reservation or OCR host port is needed. Keep the existing Caddy ingress and API data mount unchanged. Stop local_llm during resource-intensive builds and restart it afterwards. Existing report results and raw archives are preserved; changing the model does not rescan confirmed reports automatically.
