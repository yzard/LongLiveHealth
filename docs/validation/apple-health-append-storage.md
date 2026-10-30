# Apple Health append storage — 2026-10-22

## Contract

Apple Health originals use `raw/<user>/apple_health/<category>.frames`.
Each record is an unsigned 8-byte little-endian JSON byte length followed by the
unchanged UTF-8 JSON. Versions and deletion markers append; deleting a measurement
hides its SQLite projection without deleting prior frames. Account deletion
removes the complete user archive. Other import platforms keep standalone files.

The existing original index now records path, prefix offset and JSON length.
Raw downloads, hydration, detailed-series maintenance, exports and restores use
these locations. Charts continue to read SQLite. Exports include each shared
category once, ending at the snapshot's last committed frame.

Normal Apple batches use one conflict/tombstone lookup and two bulk write
statements. They flush each changed category once before committing. A blocking
filesystem job owns the SQLite transaction so cancellation cannot let a later
writer overtake an unfinished OS write. Retry truncates only uncommitted tails.
The iOS 1,000-record read/upload pipeline and persisted HealthKit anchors remain
unchanged; no full-history reimport is required.

## Verification

- Backend tests: 179 passed, 2 explicitly ignored benchmarks; Docker formatting,
  Clippy with warnings denied, tests and release build passed.
- Archive/migration tests: 17 passed, including interrupted conversion, missing
  originals, preserved committed prefixes, exact bytes, offline locking and retry.
- Additional framed-original integration verification covers 1,000-record writes,
  retained versions and tombstones, failed SQLite commit followed by recovery,
  replay, corrupted prefix rejection, export audit and complete archive restore.
- NAS image: `longlivehealth-backend-api:frames58`,
  `sha256:31d80f8380550d2012a08e6772250c3aad6ac0238758af6aebcdcfaf9a01fc70`.
  Image ID matched between Desk and NAS after transfer.

## Performance

NAS production logs before migration showed recent nonempty 1,000-record batches
at 13.4, 16.9, 24.9 and 20.5 seconds. Per-record staging took roughly 2.7–4.0 seconds;
the existing per-record database/publication loop held the writer for 9.3–21.1 seconds.

A controlled HTTP benchmark used fresh, isolated databases on the same NAS data
filesystem, one connection, 1,000 records per batch and 3,000 bytes of synthetic
raw padding per record. No real health records or credentials were copied.

| NAS image | Batch 1 | Batch 2 | Batch 3 |
| --- | ---: | ---: | ---: |
| browse49 | 5,182 ms | 4,010 ms | 4,273 ms |
| frames58 | 503 ms | 542 ms | 506 ms |

The median improvement in this isolated comparison is about 8.4×. It does not
predict exact timings against the much larger live database, across Wi-Fi, or
while HealthKit is reading protected data. Benchmark containers and data were
removed after the measurements.

## Migration

The offline converter preserves existing accounts, record IDs, query projections,
sync batches, coverage and client anchors. It packs original bytes in bounded
buffers, persists its staging index after flushing, atomically publishes schema 8,
and only then deletes old standalone Apple JSON files. It can resume after an
interruption. A bounded read-ahead pool and 1 GiB migration-only SQLite cache reduce
random disk stalls. There is no second full checksum pass over millions of files.

Live cutover verification passed: schema 8; 4,090,902 indexed originals;
79 category files totaling 17,894,465,260 bytes; 4,079,886 active records unchanged.
The last indexed frame of every category was read and its identity checked.
The API restarted with the tested image; public HTTPS returned 200 and the OCR
service availability probe passed (the OCR model was not loaded for this probe).

The remaining old JSON copies were moved out of the active data root into a
private, temporary retirement directory after committing the new index. All 79
category files were moved back to their indexed active paths before restarting
the API. Retirement cleanup runs independently in directory order, using bounded
parallel unlink operations. It removes only old JSON copies; original contents
remain in the new frames. The retirement directory is removed when cleanup ends.


## Uncapped sync requests — 2026-10-22

The deployed `frames60` removes both the 1,000-record rejection and the 40 MiB
sync-request limit (HTTP extraction and canonical digest). The existing per-record
32 MiB payload validation and bounded read-response limits are separate and remain.
The iOS client retains its existing page/pipeline behavior.

Regression coverage accepts 10,001 records, rejects an invalid final record without
partial writes, and sends a complete HTTP request above 40 MiB. Image checks passed:
formatting, Clippy, 180 backend tests (2 ignored benchmarks), and release build.

NAS isolated HTTP measurements, each request containing 10,000 fresh records:

| Raw padding per record | Request 1 | Request 2 | Request 3 |
| --- | ---: | ---: | ---: |
| 3,000 bytes | 2,336 ms | 2,475 ms | 2,314 ms |
| 5,000 bytes | 2,856 ms | 2,945 ms | 3,244 ms |

The second set exceeds 40 MiB. These measurements use a separate initially empty
database on the NAS filesystem, not the live multi-million-record database.
The benchmark containers and data were removed. NAS now runs
`longlivehealth-backend-api:frames60`, image
`sha256:f265a844c01aa4a4ba7f5f94a6fe9e3b7ab73a15a4c5b256481e109e39f14aa6`;
public HTTPS returned 200 after restart. Retirement cleanup also completed and
removed the old duplicate JSON directory and temporary cleanup containers.


## Mixed-type 10,000-record blocks - 2026-10-22

The iOS Build 58 producer fills blocks across HealthKit type boundaries, including
characteristics, activity summaries, CDA documents, and selected medication snapshots.
The block size is 10,000 total records; no total-byte cutoff or per-type flush remains.
Queries use remaining capacity, and a final partial/coverage-only block is sent.
One block uploads while the next is prepared. Pending requests include all local
anchor/snapshot writes. Acknowledgement precedes those writes; a partial local
commit retries the same request. Existing old-format pending requests drain first.

The API accepts coverage arrays and validates each record's type against them;
all records and type coverage share one SQLite transaction and replay identity.
Single-type requests remain supported for installed apps and their durable retries.
Archive appends still go to the corresponding category file.

Validation: 181 backend tests passed (2 ignored benchmarks), 41 portable Swift tests,
and 26 native simulator tests, including old and mixed checkpoint recovery. NAS
isolated HTTP tests sent 10,000 mixed records per request (one body mass, three sleep,
9,996 heart rate, 3,000 padding bytes each): 2,889 / 2,264 / 2,230 ms. Temporary test
containers/data were removed. Deployed API image `longlivehealth-backend-api:mixed61`,
SHA256 `1563431ac9cb17bf12bac302e6f4e849e9045accd26cf698558e68297e95ed41`;
public HTTPS returned 200 after restart. Device installation status is recorded in
the iOS runbook.


## Pending ECG 409 recovery - 2026-10-22

An installed pre-mixed checkpoint contained 71 ECG records in three requests
(32 / 32 / 7). NAS returned 409 while draining it, before mixed-block collection.
Read-only comparison with indexed originals found identical secure HealthKit
archives but adjacent floating-point differences in ECG voltage/time series and
precise timestamps. The projected content digest treated these as changed values.

The exceptional same-version path now reads the indexed original and accepts a
representation-only replay when the secure archive is byte-identical and every
JSON field is equal except floating numbers at most one IEEE-754 ULP apart.
Integers, structure, source metadata and the secure archive remain exact. Changed
measurements still fail; no stored originals or projections are rewritten.
Comparison/parsing runs on the existing CPU executor. Normal digest matches keep
the fast path.

182 backend tests passed (2 ignored), with a regression that accepts the adjacent
representation and rejects changed voltage, time, and archive data. An isolated NAS
database seeded from these 71 originals accepted all three actual pending requests;
the total stayed 71. The isolated database was then removed. Deployment image:
`longlivehealth-backend-api:ecg63`, SHA256
`9c938f6b5dd9504e53468c849819e862e9942f6e485dc3b28d8c5683fc8e6e88`.
Public HTTPS returned 200 after restart. No iOS update or checkpoint reset is needed.


## Medication permission 400 - 2026-10-22

Enabling selected medication access produced a 6,812-record block: one vision
prescription, 6,810 medication dose events and one medication snapshot. Of these,
6,008 source versions were fractional; the integer-only transport field rejected
the complete block before persistence. No dose records existed on NAS before the
fix. The new medication decoder retains the original numeric version and uses
positive IEEE-754 bits internally for correctly ordered SQLite revision checks.
Raw/projection content and batch identity retain the source values.

183 backend tests passed (2 ignored), including fractional update ordering, replay,
older-version rejection, raw preservation, invalid versions and same-version
content conflict. The complete actual pending batch passed in an isolated NAS
container: 6,774 active records plus 38 deletion markers, first request 2,755 ms;
replay 741 ms with unchanged count. The first diagnostic assertion incorrectly
counted deletions as active records; it was corrected and both passes rerun.
Isolated data and diagnostic copies were deleted after verification.

Deployed `longlivehealth-backend-api:medication64`, SHA256
`d1604fb4640b38d3ec2d1086fa3ea8e148529f9e1c73569349d4162ea5209d76`.
Public HTTPS returned 200. No iOS update, permission reset or cursor reset needed.

## Workout route archive limits — 2026-10-28

Removed the iOS 24 MiB series and 32 MiB single-payload ceilings and the matching backend payload/read ceilings. The 10,000-record mixed batching policy remains. Series callbacks append original values directly, avoiding repeated JSON encoding only to count bytes. Raw frame reads verify their prefix and actual file extent before allocating. Regression coverage uploads, replays and reads back a single 41 MiB payload without truncation.
