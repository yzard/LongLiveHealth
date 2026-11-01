# Server-owned Browse directory and shared response cache

Browse previously synthesized entries from a client-side measurement list and an unconditional Sleep entry before loading three independent server responses. This generated populated categories without a fetched or cached catalog. Classification rules also lived in the client.

`POST /api/v1/browse/catalog` now returns the account's complete categories and entries, with presentation metadata, from saved health and confirmed laboratory data. Classification, labels and ordering are owned by the backend. It includes Apple Health, Health Connect and manual weight records. Empty accounts produce empty directories. The mobile directory performs one request and caches the whole response; arbitrary server category IDs and titles require no client enum update. Native icon rendering remains a platform implementation detail. The superseded `wellness/sources.browse_metrics` projection and mobile seed list are removed.

Overview, Browse, Trends, Insights and Data share `ResponseCache`. Keys include the API path and canonical request body within a server/account-specific directory. Cached reads are revalidated against the server and can recover from connectivity failures. No seed data is written. Date windows, metrics and pages do not share keys. The prior separate Overview, preferences, reports, source-list and generic read snapshot files are removed as disposable caches. Report originals and pending upload drafts remain separate durable files. Sync coverage, jobs and security state use live requests, not cached error/progress state. HTTP 404 removes the matching response and deleted report preview; authentication failures cannot return cached results. Sync completion retains useful offline snapshots; subsequent reads revalidate.

The old per-type iOS pending-checkpoint replay and associated checkpoint models were removed. The current mixed, byte-sized durable upload block and acknowledged anchors remain. Two verified retired iPhone connections were removed from NAS, cascading only their coverage and replay receipts; 225 route records remained. Git had only main and origin/main, so there were no obsolete feature branches to remove.

Validation tracks account/parameter cache isolation, missing cache, arbitrary category metadata, all supported health definitions, absent/zero records, Health Connect and manual weight, laboratory classification, report deletion, Browse navigation and tab state. Existing measured data, account records and medical originals are outside the cleanup.

## Deployment and verification

- 55 shared core tests, 28 iOS native tests, Browse category/navigation and full-catalog UI checks, and Overview UI checks passed.
- Final Docker build passed formatting, Clippy, Android unit/lint/package checks, and 220 backend tests (2 existing ignored).
- NAS deployed `longlivehealth-backend-api:unified-catalog-20261031`; server status is OK and unauthenticated catalog access returns 401.
- Live account directory: 234 unique entries across 18 nonempty categories; all entries resolve to exactly one returned category.
- Removed 63 inactive API image tags on NAS after validating the new service. Running images were excluded.
- Android Build 31 is published at `/longlivehealth.apk`; public and local SHA-256 are `25d448dcc3fa764bfdb22a4da563296cbd9f6b72268f932f9b1b87a125663fe7`. No Android device is currently connected for installation.
- iOS Build 84 signed successfully after the user completed macOS keychain authorization and was installed on the connected iPhone on October 31, 2026.
