# Android client and shared implementation

Status (2026-10-29): installable Android development client; emulator acceptance checks passed. Z Fold 8 hardware acceptance remains pending at the user's request. Google Health cloud OAuth is deferred; Health Connect is implemented.

## Shared code and platform boundaries

`src/shared/Core`, `src/shared/Application`, and `src/shared/UI` own the API/authentication/session renewal, archive, byte-sized sync batching, report review, metric text, chart windows/scales/source priorities, and application screens. Both platform builds consume these canonical Swift sources. Generated Android copies exist only in `build/android/client`.

The iOS target continues to use Apple SwiftUI, Charts, HealthKit, Keychain and system glass materials. Android uses Skip Fuse (native Swift with Compose presentation), native range/point chart renderers, Android Keystore, BiometricPrompt, Health Connect, WorkManager, foreground transfer service, document pickers, camera/PDF preview, notifications, on-device dictation and calendar review.

Browse and Data use a persistent two-pane layout above 700 logical points. Below that width the selected detail occupies one pane. Folding does not reconstruct the selected metric or discard report/navigation state. Native Android icons replace unsupported SF Symbols; iOS symbols are unchanged. Exact physical-device appearance and accessibility still require hardware review.

## Health Connect

Supported reads: sleep stages, exercise sessions, steps, heart rate samples, resting heart rate, active calories, weight, height, body fat, VO₂ max, oxygen saturation, respiration, HRV RMSSD, glucose, temperature and distance. RMSSD is a separate metric from Apple's SDNN. Existing NAS records from Apple Health and reports remain browsable.

History is paged; mixed record types use the shared 8 MiB batch builder. A changes token is obtained before scanning history. Cursors advance only after server acknowledgement. Pending uploads retain stable IDs for replay; expired provider tokens restart a history scan. Deletes append tombstones. Device metadata distinguishes watch/phone sources within one provider; Health Connect does not expose a unique hardware serial, so identical models from one provider cannot always be distinguished.

Historical and background permissions are requested when supported. A scheduled worker supplies background delivery within Android's scheduling limits; a foreground service protects user-initiated work. Reviewed glucose is written only through the explicit confirmation action. Backend sleep projection translates provider stage enums without fabricating sleep when stages are unknown.

## Builds and distribution

`./build_android.sh` builds inside Docker and exports `dist/android/longlivehealth.apk`. `prepare` only stages generated sources. The persistent `longlivehealth-android-signing` volume preserves development signing identity. Swift dependencies are recorded in `src/android/Package.resolved`. The current APK includes only ARM64, compresses bundled native libraries, and is development-signed, not a Play Store release.

`./build_docker.sh TAG` builds Android before the backend images. The API image contains the APK under `/app/downloads`; `GET /longlivehealth.apk` streams this fixed public artifact without exposing user files. `./build_ios.sh` remains the separate macOS/Xcode entrypoint.

## Verified

- Docker Android entrypoint, native compilation and Android lint passed.
- 47 shared Core tests passed.
- 18 iOS AppModel simulator tests passed, including the import/foreground-refresh race regression; iOS test build passed.
- 190 backend tests passed, 2 ignored; Clippy passed.
- Real Health Connect API on an Android 36 emulator: 60-day-old heart rate samples, sleep stages and weight, changes/deletes, and distinct watch/phone sources. Synthetic records are removed in `finally`.
- Android UI: session restoration after process restart, Browse periods/source toggles, fold/unfold detail retention, tab state, report scrolling with 126 markers, Confirm All and Report Info editor.
- Android permission flow and Sync Now reached `Up to date` against the synthetic server.
- Camera: capture two pages, review, delete one page and upload one PDF automatically without Retry. Explicit imports now wait for foreground refresh rather than remain idle in the queue.
- Combined Docker build succeeded. Desk OCR's running deployment was not replaced.

Tests: `tests/android/ui_smoke.py` and `tests/android/instrumentation/HealthConnectIntegrationTest.kt`. UI tests require the disposable synthetic fixture and Android emulator container; they must never point at patient data.

## Remaining hardware acceptance

Z Fold 8 Health Connect data-provider coverage, fingerprint/device credential prompts, long background runs under Samsung battery management, camera quality and fold-hinge/display behavior. Emulator evidence does not establish these. Google Health cloud login remains outside the current scope until an OAuth project is available.
