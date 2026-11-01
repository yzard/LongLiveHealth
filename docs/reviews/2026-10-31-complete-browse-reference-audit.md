# Browse reference audit — 2026-10-31

All 232 entries in the live NAS Browse catalog were captured through their real database/query and production reference-assessment paths. Independent verification checked returned bounds, boundary inclusivity, classifications, age at measurement, reference sex and original-report source ownership. No numeric verification failures were found. No personal measurements, dates of birth, source aliases or credentials are included here; detailed captures remain in ignored operator build artifacts.

84 entries have assessed numeric data, 99 have numeric data without an applicable verified guide or sufficient context, and 49 have no numeric range buckets (including 16 non-scalar record categories). Missing context and incompatible units remain unassessed. They do not imply normal results. Qualitative records and incomplete numeric results remain accessible in Historical Values.

The iOS simulator visited all 232 actual captured entry pages across the main audit and the final focused run. The latter verified all 16 record categories plus the corrected Sleep reference display. Android physical-page verification covered all 232 entries. Captured page text was compared with the actual API bounds, classifications, original-report source/alias and every rendered scientific citation. The final controls pass covers laboratory, daily, scalar and paired-component charts. Missing guides and non-numeric views remain explicitly distinguished from assessed values. Physical historical-list checks are recorded separately; they exposed and prompted a fix for repeated row keys in nested daily history.

Fixes include conditional guide visibility without fabricated context, observation-owned printed reference units and citations, paired blood-pressure period filtering, sleep-duration formatting, and newest-first historical pagination and shared mobile row sorting. Source-record queries now constrain the record type before retrieval and enumerate related IDs before primary-key lookups. On the same live NAS archive, a read-only 51-row Workout Route query took 171.98 seconds with the source-wide nested-loop plan and 0.003 seconds after the related-ID-first change. This measures SQL retrieval, not full HTTP response time. The full backend regression suite passed 231 tests, with 2 explicitly ignored. SQLite documents the join-order control used here in its [optimizer overview](https://www.sqlite.org/optoverview.html#manual_control_of_query_plans_using_cross_join).

| Metric | Result | Explanation |
|---|---|---|
| 25-hydroxyvitamin D3 | Assessed | Applicable published or observation-owned original-report interval. |
| AB Screen | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| ABO group | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| ABORh | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Absolute basophil count | Assessed | Applicable published or observation-owned original-report interval. |
| Absolute eosinophil count | Assessed | Applicable published or observation-owned original-report interval. |
| Absolute lymphocyte count | Assessed | Applicable published or observation-owned original-report interval. |
| Absolute monocyte count | Assessed | Applicable published or observation-owned original-report interval. |
| Absolute neutrophil count | Assessed | Applicable published or observation-owned original-report interval. |
| Absolute NRBC | Assessed | Applicable published or observation-owned original-report interval. |
| Activated Partial Thromboplastin Time | Assessed | Applicable published or observation-owned original-report interval. |
| Active Energy | Not assessed | Energy expenditure depends on body size, activity and recording completeness; no universal daily normal interval. |
| Activity Move Mode | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Activity Rings | Not assessed | No verified interval |
| Alanine aminotransferase | Assessed | Applicable published or observation-owned original-report interval. |
| Albumin | Assessed | Applicable published or observation-owned original-report interval. |
| Albumin / globulin ratio | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Albumin, random urine without creatinine | Not assessed | Spot urine concentration depends on dilution. Do not apply albumin/creatinine-ratio or 24-hour thresholds to mg/dL. |
| Alkaline phosphatase | Assessed | Applicable published or observation-owned original-report interval. |
| Allergy Record | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Amylase | Assessed | Applicable published or observation-owned original-report interval. |
| Anion gap | Assessed | Applicable published or observation-owned original-report interval. |
| Antinuclear antibodies screen | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Apolipoprotein B | Assessed | Applicable published or observation-owned original-report interval. |
| Appearance - urine | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Apple Exercise Time | Not assessed | No universal clinical interval established for this measurement protocol. |
| Apple Stand Hour | Not assessed | No verified interval |
| Apple Stand Time | Not assessed | No universal clinical interval established for this measurement protocol. |
| Arachidonic acid/EPA ratio | Not assessed | This is a numeric fatty-acid ratio. No transferable reference interval has been verified for this specimen and assay; use the original laboratory interval. |
| Aspartate aminotransferase | Assessed | Applicable published or observation-owned original-report interval. |
| Atrial Rate | Not assessed | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Audio Exposure Event | Not assessed | No verified interval |
| Bacteria - urine | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Basophils % | Assessed | Applicable published or observation-owned original-report interval. |
| Beat-to-beat Measurements | Not assessed | No verified interval |
| Bilirubin - urine | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Biological Sex | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Blood Pressure | Assessed | Applicable published or observation-owned original-report interval. |
| Blood Type | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Blood urea nitrogen | Assessed | Applicable published or observation-owned original-report interval. |
| Body Mass Index (BMI) | Assessed | Applicable published or observation-owned original-report interval. |
| BUN / creatinine ratio | Assessed | Applicable published or observation-owned original-report interval. |
| Calcium | Assessed | Applicable published or observation-owned original-report interval. |
| Carbon dioxide | Assessed | Applicable published or observation-owned original-report interval. |
| Cardio Recovery | Not assessed | No universal clinical interval established for this measurement protocol. |
| Celiac - Deamidated gliadin (IgA) | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Celiac - Deamidated gliadin (IgG) | Not assessed | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Celiac - Tissue transglutaminase (IgA) | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Celiac - Tissue transglutaminase (IgG) | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Chloride | Assessed | Applicable published or observation-owned original-report interval. |
| Clinical Note Record | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Color - urine | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Cortisol | Not assessed | No unique applicable rule: age, reference sex, specimen, fasting status, or measurement context is missing or unsupported |
| Creatinine | Assessed | Applicable published or observation-owned original-report interval. |
| Creatinine-based estimated glomerular filtration rate (eGFR) | Not assessed | Equation is unspecified. A single eGFR does not establish chronic kidney disease; assess persistence and albuminuria. |
| Date Of Birth | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| DHEA sulfate | Assessed | Applicable published or observation-owned original-report interval. |
| Dietary Carbohydrates | Not assessed | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Cholesterol | Not assessed | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Energy Consumed | Not assessed | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Fat Total | Not assessed | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Fiber | Not assessed | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Protein | Not assessed | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Sodium | Not assessed | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Sugar | Not assessed | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Water | Not assessed | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Direct bilirubin | Assessed | Applicable published or observation-owned original-report interval. |
| Distance Cycling | Not assessed | No universal clinical interval established for this measurement protocol. |
| Distance Walking Running | Not assessed | No universal clinical interval established for this measurement protocol. |
| ECG | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| eGFR (formula not specified) | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| eGFR (historical CKD-EPI, Black population adjustment) | Assessed | Applicable published or observation-owned original-report interval. |
| eGFR (historical CKD-EPI, non-Black population adjustment) | Assessed | Applicable published or observation-owned original-report interval. |
| eGFR (historical MDRD) | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Environmental Audio Exposure | Not assessed | Hearing guidance uses sound level together with cumulative exposure duration. An isolated dB value cannot be assigned a safe exposure interval. |
| Environmental Sound Reduction | Not assessed | Hearing guidance uses sound level together with cumulative exposure duration. An isolated dB value cannot be assigned a safe exposure interval. |
| Eosinophils % | Assessed | Applicable published or observation-owned original-report interval. |
| Estimated Workout Effort Score | Not assessed | No universal clinical interval established for this measurement protocol. |
| Estradiol | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Ferritin | Assessed | Applicable published or observation-owned original-report interval. |
| Fitzpatrick Skin Type | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Flights Climbed | Not assessed | No universal clinical interval established for this measurement protocol. |
| Follicle stimulating hormone | Assessed | Applicable published or observation-owned original-report interval. |
| Food | Not assessed | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Gamma-glutamyl transferase | Assessed | Applicable published or observation-owned original-report interval. |
| Globulin | Assessed | Applicable published or observation-owned original-report interval. |
| Glucose | Assessed | Applicable published or observation-owned original-report interval. |
| Glucose - urine | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| HbA1c (NGSP) | Assessed | Applicable published or observation-owned original-report interval. |
| HDL cholesterol | Assessed | Applicable published or observation-owned original-report interval. |
| HDL large | Not assessed | Measurement context required: assay: quest_ion_mobility |
| Headphone Audio Exposure | Not assessed | Hearing guidance uses sound level together with cumulative exposure duration. An isolated dB value cannot be assigned a safe exposure interval. |
| Heart rate | Not assessed | Resting and exercise readings cannot share a normal interval. Use resting heart rate for the adult resting guide. |
| Height | Not assessed | Adult height is not a clinical in-range/out-of-range measurement. |
| Hematocrit | Assessed | Applicable published or observation-owned original-report interval. |
| Hemoglobin | Assessed | Applicable published or observation-owned original-report interval. |
| High-sensitivity C-reactive protein | Not assessed | Measurement context required: acute inflammation: absent |
| Homocysteine | Assessed | Applicable published or observation-owned original-report interval. |
| HRV SDNN | Not assessed | Recording duration, rhythm, activity and recovery protocol determine HRV interpretation. No protocol-independent clinical normal interval is applied. |
| Hyaline casts - urine | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Hypertension Event | Not assessed | No verified interval |
| Imm Gran % | Assessed | Applicable published or observation-owned original-report interval. |
| Imm Gran Abs | Assessed | Applicable published or observation-owned original-report interval. |
| Immunization Record | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Immunoglobulin A | Assessed | Applicable published or observation-owned original-report interval. |
| Indirect bilirubin | Assessed | Applicable published or observation-owned original-report interval. |
| Insulin | Not assessed | Measurement context required: fasting: yes |
| Intact parathyroid hormone | Assessed | Applicable published or observation-owned original-report interval. |
| International Normalization Ratio | Assessed | Applicable published or observation-owned original-report interval. |
| Iron | Assessed | Applicable published or observation-owned original-report interval. |
| Iron % saturation | Assessed | Applicable published or observation-owned original-report interval. |
| Iron binding capacity | Assessed | Applicable published or observation-owned original-report interval. |
| Ketones - urine | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Lab Result Record | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| LDL cholesterol | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| LDL medium | Not assessed | Measurement context required: assay: quest_ion_mobility |
| LDL particle number | Not assessed | Measurement context required: assay: quest_ion_mobility |
| LDL pattern | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| LDL peak size | Not assessed | Lipoprotein subfraction ranges depend on the assay (ion mobility versus NMR); do not transfer thresholds between methods. |
| LDL small | Not assessed | Measurement context required: assay: quest_ion_mobility |
| Lead | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Leptin | Not assessed | Measurement context required: assay: esoterix_elisa, bmi at measurement: 22 |
| Leukocyte esterase - urine | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Lipase | Assessed | Applicable published or observation-owned original-report interval. |
| Lipoprotein (a) | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Low Cardio Fitness Event | Not assessed | No verified interval |
| Luteinizing hormone | Assessed | Applicable published or observation-owned original-report interval. |
| Lymphocytes % | Assessed | Applicable published or observation-owned original-report interval. |
| Magnesium (serum or plasma) | Assessed | Applicable published or observation-owned original-report interval. |
| Magnesium, RBC | Not assessed | Measurement context required: method: ICP-MS, specimen: packed_red_cells |
| Mean corpuscular hemoglobin | Assessed | Applicable published or observation-owned original-report interval. |
| Mean corpuscular hemoglobin concentration | Assessed | Applicable published or observation-owned original-report interval. |
| Mean corpuscular volume | Assessed | Applicable published or observation-owned original-report interval. |
| Mean platelet volume | Assessed | Applicable published or observation-owned original-report interval. |
| Medication Doses | Not assessed | No verified interval |
| Medication Record | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Medications | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Mercury | Not assessed | Measurement context required: specimen: whole_blood |
| Methylmalonic acid | Assessed | Applicable published or observation-owned original-report interval. |
| Mindful Session | Not assessed | No verified interval |
| Monocytes % | Assessed | Applicable published or observation-owned original-report interval. |
| Neutrophils % | Assessed | Applicable published or observation-owned original-report interval. |
| Nitrite - urine | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Non-HDL cholesterol | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Nucleated RBC Auto | Assessed | Applicable published or observation-owned original-report interval. |
| Occult blood - urine | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Omega 6 / 3 ratio | Not assessed | This is a numeric fatty-acid ratio. No transferable reference interval has been verified for this specimen and assay; use the original laboratory interval. |
| Omega-3 total | Not assessed | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-3: DHA | Not assessed | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-3: DPA | Not assessed | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-3: EPA | Not assessed | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-6 total | Not assessed | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-6: arachidonic acid | Not assessed | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-6: linoleic acid | Not assessed | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Oxygen saturation | Not assessed | Measurement context required: altitude: sea_level, oxygen therapy: no |
| P Axis | Not assessed | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| P-R Interval | Not assessed | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Phosphate | Assessed | Applicable published or observation-owned original-report interval. |
| Physical Effort | Not assessed | No universal clinical interval established for this measurement protocol. |
| Platelet count | Assessed | Applicable published or observation-owned original-report interval. |
| Potassium | Assessed | Applicable published or observation-owned original-report interval. |
| Procedure Record | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Prolactin | Assessed | Applicable published or observation-owned original-report interval. |
| Prostate specific antigen %, free | Not assessed | Cancer-risk interpretation depends on concurrent total PSA, age and assay. |
| Prostate specific antigen, free | Not assessed | Free PSA alone has no universal normal interval; interpretation requires concurrent total PSA. |
| Prostate specific antigen, total | Assessed | Applicable published or observation-owned original-report interval. |
| Protein - urine | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Prothrombin Time | Assessed | Applicable published or observation-owned original-report interval. |
| Q-T Interval | Not assessed | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| QRS Duration | Not assessed | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| QTc | Not assessed | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| R Axis | Not assessed | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Recovery HRV | Not assessed | No universal clinical interval established for this measurement protocol. |
| Red blood cell count | Assessed | Applicable published or observation-owned original-report interval. |
| Red cell distribution width (CV) | Assessed | Applicable published or observation-owned original-report interval. |
| Respiratory rate | Not assessed | Measurement context required: activity: resting_awake |
| Resting Energy | Not assessed | Resting energy expenditure depends on body size and measurement method; no universal daily normal interval. |
| Resting heart rate | Assessed | Applicable published or observation-owned original-report interval. |
| Rhesus (Rh) factor | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Rheumatoid factor | Not assessed | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Running Ground Contact Time | Not assessed | No universal clinical interval established for this measurement protocol. |
| Running Power | Not assessed | No universal clinical interval established for this measurement protocol. |
| Running Speed | Not assessed | No universal clinical interval established for this measurement protocol. |
| Running Stride Length | Not assessed | No universal clinical interval established for this measurement protocol. |
| Running Vertical Oscillation | Not assessed | No universal clinical interval established for this measurement protocol. |
| SARS-CoV-2 NAAT | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Sex hormone binding globulin | Assessed | Applicable published or observation-owned original-report interval. |
| Six Minute Walk Test Distance | Not assessed | Reference equations require the standardized walking-test protocol, age, height, weight and population. A wearable estimate is not assumed equivalent to a supervised six-minute walk. |
| Sleep Apnea Events | Not assessed | No verified interval |
| Sleep sessions | Assessed | Applicable published or observation-owned original-report interval. |
| Sleeping Breathing Disturbances | Not assessed | No universal clinical interval established for this measurement protocol. |
| Sleeping Wrist Temperature | Not assessed | No universal clinical interval established for this measurement protocol. |
| Sodium | Assessed | Applicable published or observation-owned original-report interval. |
| Stair Ascent Speed | Not assessed | No universal clinical interval established for this measurement protocol. |
| Stair Descent Speed | Not assessed | No universal clinical interval established for this measurement protocol. |
| State of Mind | Not assessed | A finite, nonnegative numeric measurement is required No verified interval |
| Steps | Not assessed | Step counts are activity measures, not laboratory reference intervals. |
| T Axis | Not assessed | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Testosterone, free | Not assessed | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Testosterone, total | Assessed | Applicable published or observation-owned original-report interval. |
| Thyroglobulin | Not assessed | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Thyroglobulin antibodies | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Thyroid peroxidase antibodies | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Thyroid-stimulating hormone | Assessed | Applicable published or observation-owned original-report interval. |
| Thyroxine (T4) free | Assessed | Applicable published or observation-owned original-report interval. |
| Time In Daylight | Not assessed | No universal clinical interval established for this measurement protocol. |
| Total 25-hydroxyvitamin D (D2 + D3) | Assessed | Applicable published or observation-owned original-report interval. |
| Total bilirubin | Assessed | Applicable published or observation-owned original-report interval. |
| Total cholesterol | Assessed | Applicable published or observation-owned original-report interval. |
| Total cholesterol / HDL ratio | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Total protein | Assessed | Applicable published or observation-owned original-report interval. |
| Triglycerides | Not assessed | Measurement context required: fasting: yes |
| Triiodothyronine (T3) free | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Uric acid | Assessed | Applicable published or observation-owned original-report interval. |
| Ventricular Rate | Not assessed | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Vision Prescriptions | Not assessed | A finite, nonnegative numeric measurement is required No verified interval |
| Vital Sign Record | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Vitamin D | Not assessed | Vitamin D metabolite is unspecified. 25-hydroxy and 1,25-dihydroxy vitamin D require different intervals. |
| VO₂ Max | Assessed | Applicable published or observation-owned original-report interval. |
| Walking Asymmetry Percentage | Not assessed | No universal clinical interval established for this measurement protocol. |
| Walking Double Support Percentage | Not assessed | No universal clinical interval established for this measurement protocol. |
| Walking Heart Rate | Not assessed | No universal clinical interval established for this measurement protocol. |
| Walking Speed | Not assessed | No universal clinical interval established for this measurement protocol. |
| Walking Steadiness | Not assessed | No universal clinical interval established for this measurement protocol. |
| Walking Step Length | Not assessed | No universal clinical interval established for this measurement protocol. |
| Weight | Assessed | Applicable published or observation-owned original-report interval. |
| Wheelchair Use | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| White blood cell count | Assessed | Applicable published or observation-owned original-report interval. |
| Workout Effort Score | Not assessed | No universal clinical interval established for this measurement protocol. |
| Workout Routes | Non-numeric / no numeric buckets | Recorded values remain available in history; static characteristics use a latest-value view and ECG uses its recorded waveform. |
| Workouts | Not assessed | No verified interval |
| Zinc | Assessed | Applicable published or observation-owned original-report interval. |


Daily history now flattens date/source rows into one globally keyed list. Sleep sessions are globally newest first across sources; stage rows use a session-specific key namespace. Regression tests cover unique keys and cross-source chronology. The JSON identifier helper also recognizes generic `id` fields so multiple guideline citations keep distinct identities.

Reference explanations use a shared read-only navigation page on both clients. Chart period/data remain retained on return; window-loaded guards avoid repeating completed reads while profile changes and explicit refresh still force a fresh request. On Android, source-specific accessibility actions identify the correct paired-component info button after asynchronous layout updates.

Final acceptance: Android Build 45 is installed; the NAS API image serves the same APK bytes. All 232 catalog entries passed the captured-response/page-text verification. Six representative laboratory/daily/scalar/paired screens passed final reference navigation and actual historical-row ordering, with zero metric-read requests observed when returning from reference details. Sleep sessions and expanded stage rows were scrolled on hardware without a crash. iOS Build 96 passed compilation and the corresponding six native reference-page tests and is signed; physical installation is deferred at the user’s request. The portable suite passed 65 tests, and the backend suite passed 231 with 2 ignored.
