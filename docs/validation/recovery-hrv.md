# Recovery HRV — 2026-10-29

Recovery HRV is a separate shared Browse measurement on iOS and Android. Its persisted record type remains `HKQuantityTypeIdentifierHeartRateVariabilityRMSSD`; SDNN and Health Connect RMSSD remain separate. Keeping the shipped type and payload preserves sync anchors, queued uploads and semantic hashes.

The API projects the original HKQuantity description into milliseconds after calculating its semantic digest. Explicit milliseconds and seconds are supported; missing, malformed, negative and nonfinite quantities remain unplottable historical records. Originals are unchanged. This uses the precision present in the archived description and does not manufacture additional precision.

`index-recovery-hrv` reads only this type's referenced originals and updates SQLite projections in transactions of at most 100 records, with optimistic original/projection checks and revision invalidation. No archive reset or resync is required. Stop the API before running this operator command because the CLI owns the data-root lock.

NAS deployment: `longlivehealth-backend-api:recovery-hrv-20261029`. All 7,697 existing Recovery HRV records were projected and verified to have numeric values in milliseconds. Compose domain, storage and OCR deployment were preserved.

Validation: 192 backend tests passed, 2 ignored; clippy passed. Regression covers raw-unit validation, old projection backfill, idempotence, same-version replay, user isolation, chart values and invalid historical records. Shared core: 47 tests passed, plus the updated sync-name regression. iOS simulator and device builds passed; signed Build 65 installed on the paired iPhone. Android Docker build/lint passed; emulator verified search, chart, milliseconds, source legend, explanation and Historical Values with synthetic data. Physical Android hardware remains untested.

Android APK: https://health.zhuoyin.info/downloads/longlivehealth.apk
SHA256: `ea6da681d84fd6f3c85bc63d457fab77a6d4e5a9e25a82410be2d827de0e7a76`
