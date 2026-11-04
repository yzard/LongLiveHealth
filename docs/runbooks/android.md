# Android testing

Download the development APK from `https://health.zhuoyin.info/longlivehealth.apk`. Android may ask to allow installation from the browser used to download it. This is a directly installed development build, not a Google Play release.

Open Long Live Health and enter `https://health.zhuoyin.info` with your existing account. No account or server data reset is needed. In Data → Health Connect, choose Sync Now and grant the data access you want. Permit history/background access when offered if those features are wanted. Health Connect receives data from compatible providers; enabling it does not connect Google Health cloud OAuth.

Browse and Data show a list on the left and detail on the right when the display is wide enough. Existing Apple Health and report data are available from the same NAS account.

Developer builds run through `./build_android.sh` on a Docker host. Do not remove the `longlivehealth-android-signing` Docker volume: changing its key prevents an update over the existing installation. The published APK supports ARM64 phones only; x86 and x86_64 libraries are excluded. Canonical UI and business logic are in `src/shared`; Android-specific services are in `src/android`.

For emulator acceptance, use synthetic data with `tests/ios/simulator_fixture.py`, then run `tests/android/ui_smoke.py` on the Desk host with the `longlivehealth-android-test` emulator container. `login`, `navigation`, and `camera` exercise the corresponding flows. Health Connect instrumentation tests create and clean up synthetic provider records. Do not run these against a user's real account.

Physical fold/unfold navigation has been checked on Z Fold hardware. Biometric authentication, provider-specific data, battery-management behavior and actual hardware photo capture still need dedicated end-to-end checks.

The APK is copied into the API image at `/app/downloads/longlivehealth.apk` by the combined Docker build. `GET /longlivehealth.apk` streams that fixed file; the earlier `/downloads/longlivehealth.apk` URL remains an alias. No host artifact volume is required. Fresh installations have empty username and password fields and contain no user session. Users enter their own server and sign in separately; upgrades preserve their device-held session. Test credentials are supplied by external emulator automation, not packaged in the app.

The build explicitly compiles ARM64 after bridge generation and disables Gradle’s host-architecture native rebuild. Before staging an APK, `check_android_package.py` reads its DEX native declarations and verifies that every `health.app` JNI method is exported by the packaged ARM64 library. Missing implementations or any non-ARM64 library fail the build. Regression tests cover stale libraries, missing architectures and JNI name encoding.

## APK signing identity

The current Docker workflow builds `assembleDebug`. Android Gradle creates its development keystore on first use at `/root/.android/debug.keystore`; `build_android.sh` mounts the persistent Docker volume `longlivehealth-android-signing` there on Desk. Recreating the build container or workspace does not recreate the key. Deleting that volume, or building on another host without restoring it, can produce a different identity and prevent an in-place update of installed APKs. Preserve and securely back up the volume before moving or cleaning the build host.

On 2026-10-30, the keystore certificate and served Build 5 APK had the same SHA-256 signing certificate fingerprint: `14922a5f4e9d4fe87bfdcf60c23dd3a26829175bb18768d9f12b655193168e15`. This is the certificate fingerprint, not the APK file hash. The certificate was generated on 2026-10-29 and identifies `Android Debug`. This is a persistent development key, not a dedicated production release key. The APK contains the public certificate and signature, not the private key. Signing material must not be committed or copied into the API image; that image contains only the signed APK. Git ignores keystores and signing-property files.

## Mobile presentation

Shared pages use `mobileNavigationTitle` in `src/shared/UI/MobilePresentation.swift`. It selects compact inline Android titles while retaining native iOS title behavior. The shared Android theme uses scroll-away navigation bars: titles and actions leave on upward scroll and return on downward scroll. Browse search remains available below the collapsing bar. Each foldable pane owns its own navigation and scroll behavior; do not pin a separate heading outside its scroll viewport. `MetricDetailScreen` owns the common Browse detail layout; avoid adding separate chart titles in individual metrics.

Android colors, typography, navigation icons, input fields and segmented controls live in `src/android/app/src/main/kotlin/HealthDesign.kt`, applied once by `PresentationRootView`. This layer decorates the existing navigation containers rather than replacing them, so tab, form and fold-pane state remain owned by the shared views. Keep iOS materials in the iOS implementation.

Before accepting a presentation release on hardware, check light and dark modes, folded and expanded layouts, longer category/report titles, larger system text, search entry/clear, chart periods, and tab/back navigation with a selected metric. Check report editors without saving changes to real reports. Build/lint success does not replace this visual and interaction check.


## 2026-10-30 hardware presentation audit

Build 14 was installed over the existing Z Fold installation, preserving its account and archive. The actual external display was 1248×1972 and the unfolded internal display was 2448×1848; both were checked in dark mode. Private screenshots and UI trees are local ignored artifacts under `build/android/design-audit`, not repository assets.

The audit exercised the five tabs, Browse categories/search and metric charts/history, report list/review/info/result editor/import menu, manual weight/events, source aliases, sync coverage, activity imports, settings/password/users, comparisons, daily logs, cycle history, long-term reports, archive questions, reminders, specialty records, nutrition subpages and training. Editing checks cancel without saving real health or account changes. Routes that require missing user data or external provider permission are not equivalent to populated end-to-end validation.

Fixes include one shared inline title policy, native detail back buttons instead of an extra back row, proportional table digits, the explicit upload icon, consistent Insights naming, compact fallback report titles, and distinct loading/error/empty HRV states. Camera capture and original-document preview share an inset-aware native layout with centered titles, rounded controls and the same spacing. The Browse clear-search control uses a bundled glyph rather than an unsupported SF Symbol. Dynamic menus include their placeholder in the same `ForEach` collection to avoid a native picker index crash. Chart drag observers allow vertical parent-list scrolling while retaining selection and horizontal period navigation.

Regression commands in `tests/android/ui_smoke.py` include `selectors` (comparison/association native picker crash), `chart-scroll` (vertical gestures start over the plot and reach history), and `navigation` (session retention, source toggles, fold state, report scrolling, tab return, confirmation and editor). Seed and expand the synthetic report before `navigation`; do not execute its confirmation flow against real accounts.

A separate server issue was observed: `wellness/hrv` returns 413 for the current 90-day archive because the existing query/aggregation limits are 1,000 windows and 16 MiB. The presentation change prevents that failure from also claiming there are no records; it does not remove the backend limits. Health Connect's coverage screen also currently includes server coverage from Apple Health installations. These are data/API follow-ups, not successful data checks.

The native report layout is attached before configuring the window insets controller; camera-launch regression testing caught an initialization-order crash during this audit. An earlier Skip preference-collection exception was also recorded during the session; its reproducibility has not been established. Do not interpret presentation checks as proof that every framework crash is eliminated.

Original-document acceptance also exposed FoundationNetworking using `/data/local/tmp`, which is not writable by the application. Android startup now assigns `TMPDIR` to its private cache before initializing Swift services. `original-preview` verifies download, native preview and return to the report against the fixture.

Short reports also exposed duplicate Android lazy-list keys across processing sections. Report rows now include their section in the identity, and the original-preview test uses a short report so those rows are visible. The PNG fixture was corrected to have valid PNG chunks for Android decoding.

Final artifact: Android Build 14, SHA-256 `728035c2fbe870c729d7e8ce819cfcc2010a5cd8629ab2552517d96a1f419bae`. The phone package and public API-image download were verified. Shared report identity changes passed an unsigned iOS build. Camera import and short-report original preview passed emulator regressions, including image decoding after replacing the old corrupt fixture cache. Final native-page screenshots on the phone remain pending because the device locked; the last installed changes were checked on the emulator. Light mode and enlarged system text were not part of this hardware pass. The temporary USB stay-awake setting was restored to its original value of 0.

Follow-up on 2026-10-30: Build 14 native camera and original-PDF layouts were checked on the unlocked folded phone (1248×1972). Camera entry and cancellation returned to the report list. An existing 11-page PDF rendered correctly; the slider reached page 11 and Back/Done returned to Report Info without changing report data. Final native-page verification on the unfolded display and actual hardware camera capture remain untested.

Expanded-screen follow-up on 2026-10-30: the device reported OPENED with its physical 2448×1848 internal display. Build 14 camera entry/cancellation and the existing 11-page original PDF were checked on that display. Titles, system insets, three equal-height camera controls and page slider remained visible without clipping. PDF aspect ratio was preserved; the slider rendered page 6, then Back/Done restored the report and its Data split-pane navigation. No new layout defect was found. This closes the pending expanded native-page layout check; no new photos were captured or imported during the hardware audit.

## Build workspace cleanup

After an APK is exported successfully, the build removes its private Swift and Gradle workspaces. The success hook also removes the recognized old `build/android/client` and `research` compilation directories.

Use `dist/android/longlivehealth.apk` for installation. New emulator containers should mount `dist/android/` at `/artifacts` and install `/artifacts/longlivehealth.apk`. An existing container with an older bind path keeps its APK under `dist/android/emulator/<container-id>/`; a small link preserves its recorded source path. The old compiler caches are removed without stopping that container. See the [Android cleanup verification](../reviews/2026-10-31-android-cleanup.md).

The signing volume and shared Qwen model remain intact. Cleanup refuses to remove a workspace still mounted for compilation, or one whose paths or artifact destinations are unexpected.
