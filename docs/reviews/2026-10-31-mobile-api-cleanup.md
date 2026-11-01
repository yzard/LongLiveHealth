# Mobile API cleanup — 2026-10-31

The API route inventory now maps every public business route to its maintained mobile caller. Shared Swift application/UI code supplies the iOS and Android contract. The public APK download is retained for initial installation.

Removed unused routes: reference/catalog, reference/evaluate, observations/history, metrics/browse, health/list, health/raw/list, health/raw/{raw_id}/download, health/aggregate, wellness/clinical-age, wellness/series, files/list, files/get and jobs/get. Removed the duplicate /downloads/longlivehealth.apk alias. Web views now use the same catalog, chart and coverage routes as mobile clients.

Blood pressure chart/history queries use HKCorrelationTypeIdentifierBloodPressure. Separate scalar query aliases are rejected; ingestion still accepts raw systolic and diastolic samples so their data remains complete. The combined chart resolves both components in one server response.

The build entrypoints run tests/tools/test_mobile_api_contract.py. It fails for a public route without a mobile caller or for a mobile call without a server route. The generated inventory lists all 75 retained routes and callers.

Validation: 227 backend tests passed, two optional benchmarks ignored; 59 portable Swift tests passed; 12 frontend unit tests passed. Android 36 built in Docker and passed the paired blood pressure, one-row history and ECG switching UI checks. iOS 88 rebuilt with the corrected project version; signature verification passed and device app inventory confirms installed bundle version 88.

Deployment: NAS now runs longlivehealth-backend-api:mobile-api-20261031. Live status returned 200; retired business routes returned 404. The live Browse catalog contains exactly one Blood Pressure entry. Public Android 36 APK SHA256 matches the built artifact: c2aab091f9c82052769b16c5c992624a85616274b80d2b42af54382c0d78e037.

Limitations: the full Compose/browser integration run could not start because both Docker Hub and its alternate image registry timed out while pulling Caddy. Android 36 emulator checks passed, but physical installation is pending because adb sees no device. The previously temporary keep-awake setting must be restored to 0 on reconnect. The previous iOS artifact reported bundle version 86 despite the intended 87/88 labels. This was corrected in project.yml, rebuilt, signed and installed; device inventory now confirms 88.
