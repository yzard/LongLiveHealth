# Android Browse native bridge crash — 2026-10-29

The connected Samsung SM_F971U1 running Android 17 reported repeated main-thread crashes in Build 2:

`UnsatisfiedLinkError: No implementation found for health.app.BrowseView.Swift_initState_healthDefinitions(long)`

The APK contained updated Kotlin declarations and an updated x86_64 app library, but its ARM64 app library predated the new Browse state. This was why emulator acceptance missed the failure. The generated Gradle native build used `--arch automatic`, selecting the Docker host architecture and retaining an older ARM64 artifact.

## Fix

- Explicitly build the ARM64 native app after generating the bridge, then disable Gradle's automatic native rebuild during final assembly.
- Restrict APK packaging to `arm64-v8a`, as requested. No x86, x86_64 or ARMv7 libraries ship.
- Validate the final APK's DEX native declarations against its actual ARM64 exported symbols before staging the artifact. The check supports JNI escaping and overloaded signatures and rejects unexpected architectures or an empty declaration set.
- Add regression tests for stale native libraries, architecture restrictions and JNI name encoding.
- Increment Android versionCode to 3 and retain the signing identity and device session during upgrade.

## Build evidence

Android compilation and lint passed. All 851 app-native declarations in the final APK have implementations. Three package-validation regression tests passed. The old APK was rejected by the new checks. Build 3 was installed over Build 2 on the connected phone without clearing app data.

APK SHA-256: `556889b39812c9f69352cdd1661bc084f0b36612aa113e90bb30e689e0dddcaf`.

The API image `longlivehealth-backend-api:android-jni-20261029` serves the ARM64-only Build 3 at `/longlivehealth.apk`. After unlocking the real phone, Browse and the Heart Rate detail loaded real server data, and Back returned to the Browse list. The app process remained alive (PID 20012). At the end of this check, Android exit history contained no new crash: its latest crash was the old Build 2 failure at 20:38:18, before the upgrade. The phone locked again before search and repeated tab-switch verification could finish; those checks and explicit folded/unfolded coverage remain unverified. An initial connection EOF alert was dismissed before the data loaded; this observation does not establish its cause.
