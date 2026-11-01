# Workout Route sync status investigation

The account coverage response contains one row per installation and record type. The mobile sync page displayed every installation's error as if it belonged to the current device.

Read-only NAS inspection on 2026-10-31 found two retired Apple Health installations with Workout Route errors last updated on October 15 and October 20. The current iPhone's October 31 14:43:29 UTC query succeeded (`no_visible_samples`, meaning no new visible samples for that anchored query), and its upload returned HTTP 200. The local read-failure file was empty and its route anchor had been committed. The archive contains 225 nondeleted route records. No anchors or historical evidence were deleted or reset.

The shared sync screen now filters both the error list and full category list by the current installation UUID and health platform. Missing installation identity does not fall back to other devices. Completion of an automatic sync refreshes the visible coverage as well as manual sync completion. Actual errors belonging to the current device remain visible.

Regression coverage includes retired installation errors, current success, current failure, cross-platform entries, case-insensitive UUIDs, and missing installation identity. The simulator test checks that unowned coverage cannot appear as this device's Workout Route failure.

Validation: 56 portable core tests passed, iOS simulator/device compilation succeeded, and `testDataToolsAndErrorFirstCoverage` passed on the simulator. iOS Build 83 was signed and installed on the connected iPhone. Android Build 30 passed the Docker build (unit tests, lint and packaging) and is available from the NAS APK endpoint; no Android phone was connected for installation. The combined backend image was deployed as `coverage-device-20261031`, server status is OK, and the public APK SHA-256 matches the built artifact (`e1931345b02ba32ac1bd0b86c5ef89e46ab42104d286c641ae7e4dd4d3babe5d`).
