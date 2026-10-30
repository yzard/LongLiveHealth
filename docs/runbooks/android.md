# Android testing

Download the development APK from `https://health.zhuoyin.info/longlivehealth.apk`. Android may ask to allow installation from the browser used to download it. This is a directly installed development build, not a Google Play release.

Open Long Live Health and enter `https://health.zhuoyin.info` with your existing account. No account or server data reset is needed. In Data → Health Connect, choose Sync Now and grant the data access you want. Permit history/background access when offered if those features are wanted. Health Connect receives data from compatible providers; enabling it does not connect Google Health cloud OAuth.

Browse and Data show a list on the left and detail on the right when the display is wide enough. Existing Apple Health and report data are available from the same NAS account.

Developer builds run through `./build_android.sh` on a Docker host. Do not remove the `longlivehealth-android-signing` Docker volume: changing its key prevents an update over the existing installation. The published APK supports ARM64 phones only; x86 and x86_64 libraries are excluded. Canonical UI and business logic are in `src/shared`; Android-specific services are in `src/android`.

For emulator acceptance, use synthetic data with `tests/ios/simulator_fixture.py`, then run `tests/android/ui_smoke.py` on the Desk host with the `longlivehealth-android-test` emulator container. `login`, `navigation`, and `camera` exercise the corresponding flows. Health Connect instrumentation tests create and clean up synthetic provider records. Do not run these against a user's real account.

Hardware testing on Z Fold 8 is still required for biometric authentication, provider-specific data, battery-management behavior and physical camera/folding behavior. The user chose to complete emulator testing first.

The APK is copied into the API image at `/app/downloads/longlivehealth.apk` by the combined Docker build. `GET /longlivehealth.apk` streams that fixed file; the earlier `/downloads/longlivehealth.apk` URL remains an alias. No host artifact volume is required. Fresh installations have empty username and password fields and contain no user session. Users enter their own server and sign in separately; upgrades preserve their device-held session. Test credentials are supplied by external emulator automation, not packaged in the app.

The build explicitly compiles ARM64 after bridge generation and disables Gradle’s host-architecture native rebuild. Before staging an APK, `check_android_package.py` reads its DEX native declarations and verifies that every `health.app` JNI method is exported by the packaged ARM64 library. Missing implementations or any non-ARM64 library fail the build. Regression tests cover stale libraries, missing architectures and JNI name encoding.

## APK signing identity

The current Docker workflow builds `assembleDebug`. Android Gradle creates its development keystore on first use at `/root/.android/debug.keystore`; `build_android.sh` mounts the persistent Docker volume `longlivehealth-android-signing` there on Desk. Recreating the build container or workspace does not recreate the key. Deleting that volume, or building on another host without restoring it, can produce a different identity and prevent an in-place update of installed APKs. Preserve and securely back up the volume before moving or cleaning the build host.

On 2026-10-30, the keystore certificate and served Build 5 APK had the same SHA-256 signing certificate fingerprint: `14922a5f4e9d4fe87bfdcf60c23dd3a26829175bb18768d9f12b655193168e15`. This is the certificate fingerprint, not the APK file hash. The certificate was generated on 2026-10-29 and identifies `Android Debug`. This is a persistent development key, not a dedicated production release key. The APK contains the public certificate and signature, not the private key. Signing material must not be committed or copied into the API image; that image contains only the signed APK. Git ignores keystores and signing-property files.
