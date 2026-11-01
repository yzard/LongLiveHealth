# Reference ranges and sleep goal verification — 2026-10-31

## Fixed

- Overview previously clipped sleep at local midnight. It now groups complete observed sleep sessions and assigns them to the local wake date. Awake/in-bed intervals are excluded, overlapping stages are unioned, and incomplete/conflicting sessions do not become valid totals. Other calendar-day aggregation consumers retain their existing contract.
- Seven/thirty-day goal labels previously required every individual night to meet the goal even though the displayed number is an average. Goal status now compares that displayed average with the saved personal goal.
- The compact SQL health projection omitted Health Connect sleep stages. It now preserves stages for Health Connect sleep records, with an end-to-end storage/query regression test.
- The NLA Lp(a) PDF URL returned 404. Its citation now targets the official page for the same 2024 scientific statement. Catalog version: `2026-10-31.1`.

## Validation

- All 147 configured population/context rules exercised with 749 representative and boundary values, including inclusive/exclusive cutoffs. Tests also cover missing demographics, unsupported units/context, measurement-date age and source-specific report intervals.
- Backend: 213 passed, 2 intentionally ignored; formatting and Clippy passed.
- Shared Swift core: 51 passed; iOS and Android builds passed.
- Cross-midnight sleep test covers duplicate stages, awake intervals, in-bed-only sources, partial sessions and the repeated daylight-saving hour.
- Android physical device, existing profile: Albumin's above-range status and expanded guide checked; resting heart rate's in-range status checked. Mayo Clinic and AHA links opened the correct source content in the phone browser. Automated requests to some source sites returned bot-protection 403 responses; these were not treated as broken links.
- Physical Overview after deployment: today's sleep and the seven-day average meet the saved goal; the thirty-day average remains below it. Values and personal profile are deliberately omitted from this repository report.
- Both clients updated: Android 23 and iOS 76 installed. NAS API image: `longlivehealth-backend-api:range-sleep-fix-20261031`. Existing OCR runtime was unchanged. A brief 502 during container replacement cleared after restart; the refreshed day endpoint returned 200.

Automated validation covers every configured numerical rule; real-device UI validation samples the normal, above-range, personal-goal and source-link paths. It does not claim that every individual Browse page was manually opened.
