# Integrated Insights: cardiovascular prevention and training recovery

Reviewed 2026-11-05. This is a targeted evidence review and implementation specification, not a systematic-review publication.

## Existing feature inventory

Insights previously presented links to archive questions, reminders, specialty records, nutrition, training planning, HRV windows and behavior associations, followed by a lipid-analysis request and past model-generated analyses. These are tools and individual analyses rather than a combined assessment. Behavior associations (failed calibration) and the separate HRV-window research tool have been removed, including their endpoints. Nutrition, training planning and reminders now live under Data → Manual Imports. Existing AI reviews and archive questions remain secondary Insights actions. Specialty record details remain available through Browse. Previously stored derived results remain exportable. The main screen now presents **Cardiovascular Review** and **Training & Recovery** using the same Swift view and authenticated API on iOS and Android.

## Cardiovascular prevention

Use the **PREVENT base 10-year total cardiovascular disease equation**, independently implemented from the publicly published CDC formula. It estimates first total-CVD events, including ASCVD and heart failure. It is not an ASCVD-only percentage, a diagnosis, or the probability of an imminent event. Do not apply ASCVD-only treatment thresholds to this total-CVD output.

Sources:

- [AHA PREVENT scope](https://professional.heart.org/en/guidelines-and-statements/about-prevent-calculator).
- [Khan et al., development and validation, Circulation](https://pubmed.ncbi.nlm.nih.gov/37947085/), DOI 10.1161/CIRCULATIONAHA.123.067626.
- [CDC published base equation and coefficients](https://wwwn.cdc.gov/KDSS/detail.aspx?Qnum=Q811&topic=1).
- [AHA October 2026 update](https://professional.heart.org/-/media/PHD-Files/Guidelines-and-Statements/PREVENT/PREVENT-Update-FAQ-101426.pdf): HbA1c add-on/full equations were corrected; base equations are unaffected. This implementation does not use those optional equations.

The model needs age, biological sex, total and HDL cholesterol, systolic pressure, kidney function, smoking, diagnosed diabetes, antihypertensive medication and statin status. These inputs must be explicit. A missing medication record does not mean the person takes no medication. A single glucose result does not establish a diabetes diagnosis.

Eligibility: age 30–79, no established cardiovascular disease, compatible units and inputs within the implementation's accepted domains. CKD-EPI 2021 creatinine eGFR must be confirmed. Do not infer the method from an unspecified eGFR label. Total cholesterol 130–320 mg/dL, HDL 20–100 mg/dL, systolic pressure 90–200 mmHg and eGFR 15–140 are accepted. Rather than silently winsorizing unsupported inputs, show why a percentage cannot be calculated. Age is recomputed at assessment time.

Input policy: the latest compatible labs within 365 days and explicitly paired blood-pressure daily systolic mean within 90 days. These freshness windows are application choices, not AHA recommendations. They can combine measurements from different dates; expose values, dates and sources for review. A daily mean is not a substitute for a standardized multi-day home-BP protocol. Conflicting latest results are not averaged or silently selected. Censored or qualitative results (for example, `<10`) remain visible as reported evidence with source, date and unit; they are not changed into exact numbers. A newer non-exact result is not silently replaced by an older exact result. Preserve laboratory collection timestamps, specimen/assay separation, and report reference assessment.

ApoB, LDL, non-HDL, triglycerides, Lp(a), hs-CRP, HbA1c and VO₂ Max provide complementary evidence. They are not extra coefficients in the base equation. Prefer the original report's applicable reference assessment; otherwise use the existing cited reference-guide engine. Do not treat an unknown reference interval as normal. Display the underlying history and the inputs even when the formal risk estimate is unavailable. Numerous heart-rate samples alone cannot establish low atherosclerotic risk.

## Training and recovery

Sources:

- [Buchheit 2014](https://www.frontiersin.org/journals/physiology/articles/10.3389/fphys.2014.00073/full), DOI 10.3389/fphys.2014.00073: interpretation of heart-rate monitoring requires measurement conditions and training context.
- [Kiviniemi et al. 2007](https://pubmed.ncbi.nlm.nih.gov/17849143/): randomized HRV-guided endurance training study in a small, specific population. It does not validate a universal consumer-device readiness score.
- [Plews et al. 2014](https://pubmed.ncbi.nlm.nih.gov/24334285/): measurement frequency and compliance matter for HRV monitoring.

Compare the last seven completed local calendar days with the preceding 28 days. Require at least three recent and 14 baseline observed days per metric/source. Use the median of daily values. Do not fill absent data with zero or assume no workout record means a rest day. Keep device sources separate and do not merge SDNN, RMSSD and Recovery HRV. Unknown acquisition protocols limit interpretation; Recovery HRV is descriptive only.

Combine resting HR, HRV, sleep against the saved goal, observed workout duration and active energy. Show VO₂ Max history as longer-term fitness context, not a same-day clearance test. The initial review cues (resting HR +10%, HRV −10%, activity +20%, sleep below the personal goal) are **application heuristics, not paper-derived clinical thresholds or validated readiness rules**. Show a combined review signal only when multiple domains agree, and expose missing data and the method. Do not report an injury probability or an acute:chronic workload safety boundary.

The first version does not include daily fatigue, soreness, illness, stress or session RPE. These can contradict wearable changes and must be listed as missing context. A future questionnaire should capture these with a timestamp and a defined scale. Absence of a review signal is not exercise clearance.

## Data and UI contract

`POST /api/v1/insights/get` takes the viewer's IANA timezone. It returns both assessments, provenance, evidence plots, missing inputs, method references, data revision and computation time. Computation uses normalized SQLite data and checks for an archive revision change. Both mobile clients share the same implementation and account-isolated response cache. No model-generated numeric risk, synthetic defaults or unconfirmed OCR observations enter the assessment.

Risk Context is an autosaved overlay. Current diagnoses, smoking, medicines and eGFR method use an explicit not-specified state. Reference Settings remains the source of birth date and biological sex. An actual preference change refreshes the assessment; ordinary tab return does not reset the view.

## Verification

Tests cover sex-specific equation intercepts, missing/invalid inputs and established-CVD exclusion, source and HRV-type isolation, disjoint calendar windows across daylight saving, missing days, empty archives and invalid timezones. Existing Trends tests protect its directional filtering. Builds and additional runtime checks are reported separately; this document does not imply a clinical validation study or external certification.

Verification completed for this change:

- Backend full regression: 7 library tests and 245 integration tests passed; two existing tests were ignored. After the final refinements, all five Insights tests and the complete-history Trends regression were rerun successfully. Clippy passed with warnings treated as errors.
- iOS simulator and unsigned device builds passed. The dedicated UI test verified missing-risk presentation, the method/source sheet, separate device evidence and scrolling; its screenshot was inspected.
- Android ARM64 build, unit tests, lint and native-method verification passed. Physical Android/iPhone installation was not performed in this deployment task.
- NAS was inspected read-only to confirm the existing cardiovascular laboratory names. No confirmed reports or production settings were changed.

### Cleanup and NAS deployment verification

The cleanup build passed 7 backend library tests, 240 integration tests (2 existing ignored benchmarks), strict Clippy, 76 Swift core tests, iOS simulator/device builds, the integrated Insights UI test, and Android build/unit/lint checks. Removed endpoints are absent from the 73-route caller inventory. Full-data export/restore still preserves previously archived research results.

Live NAS checks cover authentication, retired-route 404 responses, invalid-timezone rejection, evidence agreement with Browse history, explicit missing risk context, source-separated recovery windows, and concurrent Insights/Browse/status requests. Confirmed observation contents and revisions are fingerprinted before and after the read-only assessment checks; temporary test sessions are revoked. The published APK is checked against the freshly built artifact by SHA-256.

Deployed image: `longlivehealth-backend-api:insights-reviewed-20261105`. Final live checks passed, including censored-result preservation. The shared LLM service was restored after the build. No personal medical context was filled in automatically, and no phone installation was performed.


### Android physical-device follow-up — 2026-11-06

Installed Android build 60 on the connected Samsung foldable, retaining the account. Physical checks used the folded screen. Verified cardiovascular findings, six explicit missing-risk confirmations, the unchanged Risk Context editor, evidence disclosure, all 17 plottable cardiovascular history cards, six recovery comparison signals, five device-separated fitness histories, missing recovery context, method sheets, browser links, and return/tab scroll-state preservation. AHA PREVENT and Buchheit PubMed links opened successfully. No medical profile answers or confirmed report results were changed.

Fixed two issues found during the checks: calendar-only collection dates now retain their original date instead of shifting to the previous day in New York, and measurement disclosure rows align left on Android. The API preserves the original `sampled_at` field alongside the timestamp; both clients use the shared formatter. Date-only behavior is covered across New York/Tokyo, and exact timestamps still use the viewer timezone.

Backend strict checks passed: 7 library and 240 integration tests, 2 existing ignored benchmarks. Swift core tests: 77 passed. iOS simulator/device compilation and build 106 development signing passed. Physical iPhone installation and UI testing remain deferred at the user's request; test-runner provisioning requires an Xcode account. The opt-in live iOS test compiles but has not run against the phone.

NAS image is now `longlivehealth-backend-api:insights-device-20261106`. Live checks matched nine laboratory numeric inputs to Browse and preserved the non-exact Lp(a) result and original collection date. All 126 confirmed observations retained identical revisions and contents. Concurrent Insights/Browse/status requests completed in approximately 1.98/2.49/0.001 seconds. The published APK matches the installed artifact: SHA-256 `d3f31c7d954ef9b3d3965d93d54e5963ee44168b67bb67c3941c9a63f9255b61`.

An earlier build 59 startup logged a Compose layer exception. It did not recur during subsequent navigation or the build 60 verification; no claim is made that its underlying framework cause has been diagnosed. Device diagnostics are retained privately outside the repository.


### iPhone physical-device follow-up — 2026-11-06

Installed and verified build 107 on the paired iPhone 16 Pro Max. After the owner completed Xcode account and device automation authorization, the opt-in live UI test passed on the physical phone. It checked cardiovascular findings, source disclosure, opening the AHA link in the external browser and returning, measurement provenance, the unchanged Risk Context sheet, recovery method/source disclosure, scrolling through recovery charts, and Browse/Insights tab return. Screenshots were inspected privately. Fixed truncation of long measurement titles by allowing vertical wrapping in the shared view, and confirmed the complete eGFR title in the device screenshot. Date-only bloodwork collection dates remained unchanged.

The initial UI test incorrectly treated an offscreen disclosure as tappable. Its scrolling helper now checks the visible viewport before tapping, and the final test passed. No account context, medical answers, or health data were edited. The owner temporarily selected Never for Auto-Lock during testing and should restore their preferred interval afterward. Android remains on build 60; the title-wrapping change is in shared source but has not been repackaged for Android in this iPhone-only follow-up.

## November 6 — dashboard redesign

Read-only Apple Health beta inspection covered the Sleep, Readiness, Vitals,
Heart and Fitness summary tiles and each pane's upper and lower content.
Private screenshots remain under `/tmp/llh-insights-device`, outside the repository.

| Domain | Reference hierarchy | Long Live Health implementation |
| --- | --- | --- |
| Sleep | Main result, segmented visual, stages, detailed comparisons | Recorded duration, personal-goal ring, original stage timeline, per-source history and method |
| Readiness | Headline, factors, detailed contributing evidence | Three evidence pillars (sleep/vitals/activity), baseline review, separate source/protocol comparisons |
| Vitals | Observed readings positioned within a personal band | Per-metric reading and observed baseline band, dates, full source history |
| Heart | Prominent heart/value, small history, supporting metrics | Heart/value graphic, explicitly labeled daily reading, time series, HRV and resting-rate evidence |
| Fitness | Main activity visual, supporting metrics, recent history | Actual activity reading and mini history, workout and VO2 evidence in detail |
| Cardiovascular | Same summary-to-detail design language | Risk value or missing-input state, first actionable finding; full PREVENT evidence, context and sources in detail |

The mobile UI is shared SwiftUI with server-owned dashboard data. Details open
above the retained dashboard; returning does not request or discard the dashboard.

Apple's published sleep weights are duration 50, bedtime consistency 30 and
interruptions 20. Readiness inputs and 0–10 categories are published, but the
cited support documents do not provide the complete numerical mapping. No
Apple-equivalent score is fabricated. Sleep-stage classification comes from
imported records, not a new accelerometer classifier.
Sources: https://support.apple.com/en-ie/108906 and
https://support.apple.com/en-ie/128112.

Vitals bands are explicitly descriptive source-specific observed min–max values,
not clinical reference intervals and not Apple's proprietary Typical bands.
At least 14 observed baseline days are required. Old device sources remain in
details; each preview identifies its date. Sleep selects the longest complete
session on the most recent recorded wake date, without adding overlapping devices.

Validation: the simulator opened and dismissed all six detail pages successfully.
Android compilation exposed private property bridge restrictions; state and
environment properties were corrected for the shared native bridge.

Physical-device follow-up found Android's bounded-width lazy stack expanding
beyond the narrow display. The six-card dashboard now uses the same simple
full-width vertical layout as Browse, and Android category symbols reuse its
existing licensed Material PNG assets. The Insights GET operation retries up to
three coherent snapshots on a revision conflict during sync; other failures are
not retried or hidden. Production audit retained all 126 confirmed observations.

Screenshot review caught a first-presentation sheet showing an empty panel despite
successful navigation assertions. Selection now uses one identifiable sheet item
instead of separate data and visibility state. Device and simulator checks now
assert the selected detail title on every presentation. Android screenshot review
also replaced unsupported alert and blood-work symbols with supported renderers.

Final physical validation: iOS build 111 opened all six correctly titled details,
method sheets, and retained its dashboard after tab return. Android build 64 was
installed with the existing signing identity; all six cards/details were checked
on the physically OPENED and CLOSED Z Fold displays, including screenshots,
scrolling and Back. Unsupported symbols and narrow-screen overflow are resolved.
The temporary Android charging-awake override was restored to its original zero.
The user-set iPhone Auto-Lock interval remains for the owner to restore.

NAS runs `insights-dashboard4-20261106`, including Android 64. The final image
passed 7 library and 241 integration tests (2 explicitly ignored) and Clippy.
An earlier restart test hit its asynchronous data-directory lock race; a complete
unchanged rerun passed. Android lint/package checks passed after recovering a
Maven Central rate-limited test dependency from Google's mirror with SHA-1
verification in the disposable build only. Deployment audit again retained all
126 confirmed observations and checked the six domains and cited lab inputs.


### Insights as the application home (2026-11-06)

Both clients now have three tabs: Insights, Browse and Data. A new app process
opens Insights. The former Overview table/customization and Trends cards are
removed. Data and daily Browse details were split out of the mixed Overview
source file. Shared formatting and chart code now have metric/Insights names.
Android's native tab icons were updated to match the new positions.

Removed public routes: `wellness/day`, `wellness/trends`, `trends/get`, and the
obsolete `wellness/report`. Complete longitudinal calculations used by Insights
moved into `insights/history`; Browse daily history and the coach's internal daily
aggregation remain. Obsolete favorite/layout fields are excluded from returned
preferences and rejected in new writes. Reading persisted pre-upgrade preferences
preserves sleep goals, reference context and source settings. Raw health archives
are not rewritten.

Validation: 71 Swift Core tests, 12 browser client tests, 7 Rust library tests and
239 integration tests passed (2 ignored); Clippy and Android lint/package checks
passed. iPhone build 112 opened all six details and method sheets and preserved
the dashboard on tab return. Android build 65 passed cold-start, six-card/detail,
Back and three-tab retained-scroll checks on the physically folded Z Fold.
The final Android-only icon correction was rebuilt and visually verified.

NAS deployment `insights-tabs-final-20261106` passed a live six-domain and
nine-laboratory-input audit. The four retired routes returned 404, obsolete
preference keys were absent, the saved 420-minute sleep goal was preserved,
and all 126 confirmed observation payloads/revisions remained unchanged.
Final Android rebuild was rechecked against the deployed API: all six detail
screens, Back, Browse/Data switching and retained Insights scroll passed. Public
APK SHA-256 matches the installed build artifact. Temporary Android stay-awake
was restored to the original value (0). Desk local-llm.service is active again.

## Recovery on 2026-11-14

The Mac working tree contained the integrated six-card Insights dashboard and
three-tab clients, but those changes had not been committed. Desk builds from
`5319b28` included newer OCR work while still using the earlier five-tab UI.
The recovered Mac state is preserved in commit `9e63bc4`, then merged with
`5319b28`. Desk's uncommitted OCR/LOINC changes were also copied into the recovery
without modifying Desk's active working tree.

Both clients use Insights, Browse, Data, with Insights selected at launch.
Android native tab icons use the same three-position order. The latest OCR
report revision invalidation now refreshes Insights instead of the removed
Trends screen. Android smoke checks reject the retired Overview/Trends tabs.

Recovery verification: 72 shared Swift tests, 9 browser client tests, 4 Android
package checks, the 69-route mobile API inventory, and an iOS Simulator build
passed. Android package compilation and physical-device installation have not
been performed for this recovery. The local Mac has no Rust toolchain or Docker
runtime, so backend compilation was not rerun in this recovery session.
