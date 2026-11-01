# Evidence-based range guide audit — 2026-10-31

## Scope and interpretation

Audited the deployed Browse catalog (150 laboratory/clinical entries), the 85 stored health record types, and the 118 registered laboratory definitions. Counts describe catalog entries, not distinct biological analytes. The bundled catalog includes additional supported Apple Health types. No patient values, identifiers, dates of birth or source aliases are included here.

The bundled catalog contains 372 entries; 95 have numeric guides. Other entries explicitly decline automatic numeric assessment. This is a research-backed software guide, not a clinically validated diagnostic system.

## Selection rules

- Use a valid original report interval only for that same source, timestamp and exact measurement. A FHIR interval must be unambiguous and have matching units.
- Otherwise select a unique published rule using age at measurement, reference population and required specimen/protocol context. Unknown context is not normal.
- Store date of birth and reference sex per account in versioned preferences. Do not infer them from the administrator name or device. Pregnancy context must belong to the measurement, not the current date.
- Distinguish reference intervals, risk/decision categories and recommended targets. No fabricated critical limits, no diagnosis from a single test, no mass-to-molar Lp(a) conversion.
- BMI/sex-limited leptin values are not extrapolated to other weights. Dietary, audio exposure, activity and walking-test guidance cannot classify isolated wearable records without the matching protocol or observation duration.
- Chart status refers to the latest actual measurement in the selected interval, not to an average or the min/max envelope. Source toggles change which measurement is evaluated.
- Guidelines are embedded at build time; there is no runtime medical web lookup. Existing report intervals and personal goals are not replaced.

The server returns the exact `applicable_rule` used for each assessment. The chart displays only eligible bounds, including dynamic adult weight bounds from BMI thresholds multiplied by the most recent preceding adult height squared. Missing height is explicit; future height readings never classify earlier weight readings.

VO₂ Max uses the sex- and decade-specific FRIEND 2022 treadmill reference percentiles for ages 20–89. The app labels <P20 Low, P20–P50 Below Average, P50–P80 Above Average and ≥P80 High. These application labels are not a claim to reproduce Apple's proprietary classification. For day counts, VO₂ Max uses daily mean; other metrics use the latest daily measurement. Each local date counts once, choosing the preferred visible device source. Days without measurements do not count. Thresholds are selected at measurement age, and profile changes reevaluate the current window.

## API contract

Authenticated `POST /api/v1/reference/catalog` returns versioned metrics, numeric rules, eligibility conditions, explicit exclusions and source metadata.

Authenticated `POST /api/v1/reference/evaluate` accepts `metric_id`, `value`, `unit`, `measured_at` (Unix seconds) and a `context` object. It uses the signed-in account’s profile, not a caller-supplied user ID. Unit normalization is deliberately required before evaluation; incompatible units return `not_assessed`.

Existing laboratory history, measurement chart and daily history responses include `reference_guidance` plus per-bucket `reference_assessment` (including `applicable_rule`) and `chart_ranges.reference_days`. No extra per-source frontend requests are needed.

Profiles are in `preferences.reference_profile` (`birth_date`, `reference_sex`). Both clients edit these through Settings → Reference Profile, using the existing versioned, auto-saving preferences endpoint.

## Actual Browse entries

| Metric | Numeric guide | Disposition / source IDs |
|---|---|---|
| 25-hydroxyvitamin D3 | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| AB Screen | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| ABO group | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| ABORh | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Absolute basophil count | Yes, subject to context | lab_context, cbc |
| Absolute eosinophil count | Yes, subject to context | lab_context, cbc |
| Absolute lymphocyte count | Yes, subject to context | lab_context, cbc |
| Absolute monocyte count | Yes, subject to context | lab_context, cbc |
| Absolute neutrophil count | Yes, subject to context | lab_context, cbc |
| Absolute NRBC | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Activated Partial Thromboplastin Time | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Alanine aminotransferase | Yes, subject to context | lab_context, cmp |
| Albumin | Yes, subject to context | lab_context, cmp |
| Albumin / globulin ratio | No | Use the laboratory interval; component methods and reference populations differ. |
| Albumin, random urine without creatinine | No | Spot urine concentration depends on dilution. Do not apply albumin/creatinine-ratio or 24-hour thresholds to mg/dL. |
| Alkaline phosphatase | Yes, subject to context | lab_context, cmp |
| Amylase | Yes, subject to context | lab_context, abim |
| Anion gap | Yes, subject to context | abim |
| Antinuclear antibodies screen | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Apolipoprotein B | Yes, subject to context | lab_context, abim |
| Appearance - urine | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Arachidonic acid/EPA ratio | No | This is a numeric fatty-acid ratio. No transferable reference interval has been verified for this specimen and assay; use the original laboratory interval. |
| Aspartate aminotransferase | Yes, subject to context | lab_context, cmp |
| Atrial Rate | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Bacteria - urine | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Basophils % | Yes, subject to context | lab_context, abim |
| Bilirubin - urine | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Blood urea nitrogen | Yes, subject to context | lab_context, cmp |
| BUN / creatinine ratio | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Calcium | Yes, subject to context | lab_context, cmp |
| Carbon dioxide | Yes, subject to context | lab_context, cmp |
| Celiac - Deamidated gliadin (IgA) | No | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Celiac - Deamidated gliadin (IgG) | No | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Celiac - Tissue transglutaminase (IgA) | No | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Celiac - Tissue transglutaminase (IgG) | No | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Chloride | Yes, subject to context | lab_context, cmp |
| Color - urine | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Cortisol | Yes, subject to context | lab_context, abim |
| Creatinine | Yes, subject to context | lab_context, cmp |
| Creatinine-based estimated glomerular filtration rate (eGFR) | No | Equation is unspecified. A single eGFR does not establish chronic kidney disease; assess persistence and albuminuria. |
| DHEA sulfate | Yes, subject to context | lab_context, dheas |
| Direct bilirubin | Yes, subject to context | abim |
| eGFR (formula not specified) | Yes, subject to context | kdigo |
| eGFR (historical CKD-EPI, Black population adjustment) | Yes, subject to context | kdigo |
| eGFR (historical CKD-EPI, non-Black population adjustment) | Yes, subject to context | kdigo |
| eGFR (historical MDRD) | Yes, subject to context | kdigo |
| Eosinophils % | Yes, subject to context | lab_context, abim |
| Estradiol | Yes, subject to context | lab_context, abim |
| Ferritin | Yes, subject to context | lab_context, ferritin |
| Follicle stimulating hormone | Yes, subject to context | lab_context, abim |
| Gamma-glutamyl transferase | Yes, subject to context | lab_context, abim |
| Globulin | Yes, subject to context | lab_context, abim |
| Glucose | Yes, subject to context | lab_context, niddk, cmp |
| Glucose - urine | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| HbA1c (NGSP) | Yes, subject to context | lab_context, niddk |
| HDL cholesterol | Yes, subject to context | lab_context, abim |
| HDL large | Yes, subject to context | lab_context, quest_lipids |
| Hematocrit | Yes, subject to context | lab_context, cbc |
| Hemoglobin | Yes, subject to context | lab_context, cbc |
| High-sensitivity C-reactive protein | Yes, subject to context | lab_context, abim |
| Homocysteine | Yes, subject to context | lab_context, abim |
| Hyaline casts - urine | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Imm Gran % | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Imm Gran Abs | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Immunoglobulin A | Yes, subject to context | lab_context, abim |
| Indirect bilirubin | Yes, subject to context | abim |
| Insulin | Yes, subject to context | lab_context, abim |
| Intact parathyroid hormone | Yes, subject to context | lab_context, pth |
| International Normalization Ratio | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Iron | Yes, subject to context | lab_context, iron |
| Iron % saturation | Yes, subject to context | lab_context, iron |
| Iron binding capacity | Yes, subject to context | lab_context, iron |
| Ketones - urine | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| LDL cholesterol | Yes, subject to context | lab_context, abim |
| LDL medium | Yes, subject to context | lab_context, quest_lipids |
| LDL particle number | Yes, subject to context | lab_context, quest_lipids |
| LDL pattern | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| LDL peak size | No | Lipoprotein subfraction ranges depend on the assay (ion mobility versus NMR); do not transfer thresholds between methods. |
| LDL small | Yes, subject to context | lab_context, quest_lipids |
| Lead | Yes, subject to context | lab_context, lead |
| Leptin | Yes, subject to context | lab_context, leptin |
| Leukocyte esterase - urine | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Lipase | Yes, subject to context | lab_context, abim |
| Lipoprotein (a) | Yes, subject to context | lab_context, nla |
| Luteinizing hormone | Yes, subject to context | lab_context, abim |
| Lymphocytes % | Yes, subject to context | lab_context, abim |
| Magnesium (serum or plasma) | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Magnesium, RBC | Yes, subject to context | lab_context, rbc_mg |
| Mean corpuscular hemoglobin | Yes, subject to context | lab_context, uhd |
| Mean corpuscular hemoglobin concentration | Yes, subject to context | lab_context, uhd |
| Mean corpuscular volume | Yes, subject to context | lab_context, cbc |
| Mean platelet volume | No | MPV depends on analyzer, anticoagulant and sample processing time. Use the original laboratory interval. |
| Mercury | Yes, subject to context | lab_context, mercury |
| Methylmalonic acid | Yes, subject to context | lab_context, abim |
| Monocytes % | Yes, subject to context | lab_context, abim |
| Neutrophils % | Yes, subject to context | lab_context, abim |
| Nitrite - urine | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Non-HDL cholesterol | No | Treatment goals depend on cardiovascular risk and the corresponding LDL goal; no single personal target is assumed. |
| Nucleated RBC Auto | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Occult blood - urine | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Omega 6 / 3 ratio | No | This is a numeric fatty-acid ratio. No transferable reference interval has been verified for this specimen and assay; use the original laboratory interval. |
| Omega-3 total | No | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-3: DHA | No | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-3: DPA | No | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-3: EPA | No | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-6 total | No | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-6: arachidonic acid | No | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-6: linoleic acid | No | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| P Axis | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| P-R Interval | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Phosphate | Yes, subject to context | abim |
| Platelet count | Yes, subject to context | lab_context, cbc |
| Potassium | Yes, subject to context | lab_context, cmp |
| Prolactin | Yes, subject to context | lab_context, abim |
| Prostate specific antigen %, free | No | Cancer-risk interpretation depends on concurrent total PSA, age and assay. |
| Prostate specific antigen, free | No | Free PSA alone has no universal normal interval; interpretation requires concurrent total PSA. |
| Prostate specific antigen, total | Yes, subject to context | lab_context, psa |
| Protein - urine | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Prothrombin Time | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Q-T Interval | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| QRS Duration | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| QTc | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| R Axis | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Red blood cell count | Yes, subject to context | lab_context, cbc |
| Red cell distribution width (CV) | Yes, subject to context | lab_context, cbc |
| Rhesus (Rh) factor | No | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Rheumatoid factor | No | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| SARS-CoV-2 NAAT | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Sex hormone binding globulin | Yes, subject to context | lab_context, abim |
| Sodium | Yes, subject to context | lab_context, cmp |
| T Axis | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Testosterone, free | No | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Testosterone, total | Yes, subject to context | lab_context, abim |
| Thyroglobulin | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Thyroglobulin antibodies | No | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Thyroid peroxidase antibodies | No | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Thyroid-stimulating hormone | Yes, subject to context | lab_context, abim |
| Thyroxine (T4) free | Yes, subject to context | lab_context, abim |
| Total 25-hydroxyvitamin D (D2 + D3) | Yes, subject to context | vitd |
| Total bilirubin | Yes, subject to context | lab_context, cmp |
| Total cholesterol | Yes, subject to context | lab_context, abim |
| Total cholesterol / HDL ratio | No | Treatment decisions depend on the full lipid profile and overall cardiovascular risk, not a universal ratio cutoff. |
| Total protein | Yes, subject to context | lab_context, cmp |
| Triglycerides | Yes, subject to context | lab_context, abim |
| Triiodothyronine (T3) free | Yes, subject to context | lab_context, abim |
| Uric acid | Yes, subject to context | lab_context, uric |
| Ventricular Rate | No | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Vitamin D | No | Vitamin D metabolite is unspecified. 25-hydroxy and 1,25-dihydroxy vitamin D require different intervals. |
| White blood cell count | Yes, subject to context | lab_context, cbc |
| Zinc | Yes, subject to context | lab_context, zinc |

## Stored health record types

| Type | Numeric guide | Disposition |
|---|---|---|
| HKCategoryTypeIdentifierAppleStandHour | No | Categorical or composite record; a scalar normal range does not apply. |
| HKCategoryTypeIdentifierAudioExposureEvent | No | Hearing guidance uses sound level together with cumulative exposure duration. An isolated dB value cannot be assigned a safe exposure interval. |
| HKCategoryTypeIdentifierHypertensionEvent | No | Categorical or composite record; a scalar normal range does not apply. |
| HKCategoryTypeIdentifierLowCardioFitnessEvent | No | Categorical or composite record; a scalar normal range does not apply. |
| HKCategoryTypeIdentifierSleepApneaEvent | No | Categorical or composite record; a scalar normal range does not apply. |
| HKCharacteristicTypeIdentifierActivityMoveMode | No | Categorical or composite record; a scalar normal range does not apply. |
| HKCharacteristicTypeIdentifierBiologicalSex | No | Categorical or composite record; a scalar normal range does not apply. |
| HKCharacteristicTypeIdentifierBloodType | No | Categorical or composite record; a scalar normal range does not apply. |
| HKCharacteristicTypeIdentifierDateOfBirth | No | Categorical or composite record; a scalar normal range does not apply. |
| HKCharacteristicTypeIdentifierFitzpatrickSkinType | No | Categorical or composite record; a scalar normal range does not apply. |
| HKCharacteristicTypeIdentifierWheelchairUse | No | Categorical or composite record; a scalar normal range does not apply. |
| HKClinicalTypeIdentifierAllergyRecord | No | Categorical or composite record; a scalar normal range does not apply. |
| HKClinicalTypeIdentifierClinicalNoteRecord | No | Categorical or composite record; a scalar normal range does not apply. |
| HKClinicalTypeIdentifierImmunizationRecord | No | Categorical or composite record; a scalar normal range does not apply. |
| HKClinicalTypeIdentifierLabResultRecord | No | Categorical or composite record; a scalar normal range does not apply. |
| HKClinicalTypeIdentifierMedicationRecord | No | Categorical or composite record; a scalar normal range does not apply. |
| HKClinicalTypeIdentifierProcedureRecord | No | Categorical or composite record; a scalar normal range does not apply. |
| HKClinicalTypeIdentifierVitalSignRecord | No | Categorical or composite record; a scalar normal range does not apply. |
| HKCorrelationTypeIdentifierBloodPressure | No | Categorical or composite record; a scalar normal range does not apply. |
| HKCorrelationTypeIdentifierFood | No | Categorical or composite record; a scalar normal range does not apply. |
| HKQuantityTypeIdentifierAppleSleepingBreathingDisturbances | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierAppleSleepingWristTemperature | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierAppleWalkingSteadiness | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierBodyMassIndex | Yes, subject to context | bmi |
| HKQuantityTypeIdentifierDietaryCarbohydrates | No | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| HKQuantityTypeIdentifierDietaryCholesterol | No | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| HKQuantityTypeIdentifierDietaryEnergyConsumed | No | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| HKQuantityTypeIdentifierDietaryFatTotal | No | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| HKQuantityTypeIdentifierDietaryFiber | No | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| HKQuantityTypeIdentifierDietaryProtein | No | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| HKQuantityTypeIdentifierDietarySodium | No | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| HKQuantityTypeIdentifierDietarySugar | No | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| HKQuantityTypeIdentifierDietaryWater | No | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| HKQuantityTypeIdentifierEnvironmentalAudioExposure | No | Hearing guidance uses sound level together with cumulative exposure duration. An isolated dB value cannot be assigned a safe exposure interval. |
| HKQuantityTypeIdentifierEnvironmentalSoundReduction | No | Hearing guidance uses sound level together with cumulative exposure duration. An isolated dB value cannot be assigned a safe exposure interval. |
| HKQuantityTypeIdentifierEstimatedWorkoutEffortScore | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierHeadphoneAudioExposure | No | Hearing guidance uses sound level together with cumulative exposure duration. An isolated dB value cannot be assigned a safe exposure interval. |
| HKQuantityTypeIdentifierHeartRateRecoveryOneMinute | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierHeartRateVariabilityRMSSD | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierPhysicalEffort | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierRunningGroundContactTime | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierRunningPower | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierRunningSpeed | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierRunningStrideLength | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierRunningVerticalOscillation | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierSixMinuteWalkTestDistance | No | Reference equations require the standardized walking-test protocol, age, height, weight and population. A wearable estimate is not assumed equivalent to a supervised six-minute walk. |
| HKQuantityTypeIdentifierStairAscentSpeed | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierStairDescentSpeed | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierTimeInDaylight | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierWalkingAsymmetryPercentage | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierWalkingDoubleSupportPercentage | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierWalkingStepLength | No | No universal clinical interval established for this measurement protocol. |
| HKQuantityTypeIdentifierWorkoutEffortScore | No | No universal clinical interval established for this measurement protocol. |
| active_energy | No | Energy expenditure depends on body size, activity and recording completeness; no universal daily normal interval. |
| activity_summary | No | Categorical or composite record; a scalar normal range does not apply. |
| basal_energy | No | Resting energy expenditure depends on body size and measurement method; no universal daily normal interval. |
| blood_pressure_diastolic | Yes, subject to context | bp |
| blood_pressure_systolic | Yes, subject to context | bp |
| body_mass | Yes, subject to context | bmi |
| cycling_distance | No | No universal clinical interval established for this measurement protocol. |
| electrocardiogram | No | Categorical or composite record; a scalar normal range does not apply. |
| exercise_time | No | No universal clinical interval established for this measurement protocol. |
| flights_climbed | No | No universal clinical interval established for this measurement protocol. |
| heart_rate | No | Resting and exercise readings cannot share a normal interval. Use resting heart rate for the adult resting guide. |
| heartbeat_series | No | Categorical or composite record; a scalar normal range does not apply. |
| height | No | Adult height is not a clinical in-range/out-of-range measurement. |
| hrv_sdnn | No | Recording duration, rhythm, activity and recovery protocol determine HRV interpretation. No protocol-independent clinical normal interval is applied. |
| medication_dose_event | No | Categorical or composite record; a scalar normal range does not apply. |
| mindful_session | No | Categorical or composite record; a scalar normal range does not apply. |
| oxygen_saturation | Yes, subject to context | oxygen |
| preferences | No | Composite administrative or archive record; no scalar interval applies. |
| respiratory_rate | Yes, subject to context | resp |
| resting_heart_rate | Yes, subject to context | aha_hr |
| sleep | Yes, subject to context | sleep |
| stand_time | No | No universal clinical interval established for this measurement protocol. |
| state_of_mind | No | Categorical or composite record; a scalar normal range does not apply. |
| steps | No | Step counts are activity measures, not laboratory reference intervals. |
| user_annotated_medications | No | Categorical or composite record; a scalar normal range does not apply. |
| vision_prescription | No | Categorical or composite record; a scalar normal range does not apply. |
| vo2_max | Yes, subject to context | friend_2022 |
| walking_distance | No | No universal clinical interval established for this measurement protocol. |
| walking_heart_rate | No | No universal clinical interval established for this measurement protocol. |
| walking_speed | No | No universal clinical interval established for this measurement protocol. |
| workout | No | Categorical or composite record; a scalar normal range does not apply. |
| workout_route | No | Categorical or composite record; a scalar normal range does not apply. |

## Sources

- `abim`: [ABIM Laboratory Test Reference Ranges (January 2026)](https://www.abim.org/media/e2wdwdqu/laboratory-reference-ranges.pdf); checked 2026-10-30.
- `cmp`: [Mayo Clinic Laboratories: Comprehensive Metabolic Panel](https://www.mayocliniclabs.com/test-catalog/Overview/113631); checked 2026-10-30.
- `cbc`: [Mayo Clinic Laboratories: Complete Blood Cell Count](https://www.mayocliniclabs.com/test-catalog/Overview/9109); checked 2026-10-30.
- `ferritin`: [Mayo Clinic Laboratories: Ferritin](https://www.mayocliniclabs.com/test-catalog/Overview/619953); checked 2026-10-30.
- `iron`: [Mayo Clinic Laboratories: Iron and Total Iron-Binding Capacity](https://www.mayocliniclabs.com/test-catalog/Overview/621385); checked 2026-10-30.
- `dheas`: [Mayo Clinic Laboratories: Dehydroepiandrosterone Sulfate](https://www.mayocliniclabs.com/test-catalog/Overview/113594); checked 2026-10-30.
- `psa`: [Mayo Clinic Laboratories: Prostate-Specific Antigen Screen](https://www.mayocliniclabs.com/test-catalog/Overview/82023); checked 2026-10-30.
- `niddk`: [NIDDK: Diabetes Tests and Diagnosis](https://www.niddk.nih.gov/health-information/diabetes/overview/tests-diagnosis); checked 2026-10-30.
- `nla`: [Koschinsky et al. NLA focused update (2024), doi:10.1016/j.jacl.2024.03.001](https://www.lipid.org/resource/a-focused-update-to-the-2019-nla-scientific-statement-on-use-of-lipoproteina-in-clinical-practice/); checked 2026-10-30.
- `aha_hr`: [American Heart Association: All About Heart Rate](https://www.heart.org/en/health-topics/high-blood-pressure/the-facts-about-high-blood-pressure/all-about-heart-rate-pulse); checked 2026-10-30.
- `bmi`: [CDC: Adult BMI Categories](https://www.cdc.gov/bmi/adult-calculator/); checked 2026-10-30.
- `sleep`: [CDC: About Sleep; AASM / Sleep Research Society consensus](https://www.cdc.gov/sleep/about/); checked 2026-10-30.
- `lead`: [CDC / NIOSH: Blood Lead Level Guidance](https://www.cdc.gov/niosh/lead/bll-reference/index.html); checked 2026-10-30.
- `lab_context`: [MedlinePlus: How to Understand Your Lab Results](https://medlineplus.gov/lab-tests/how-to-understand-your-lab-results/); checked 2026-10-30.
- `zinc`: [Mayo Clinic Laboratories: Zinc, Serum](https://www.mayocliniclabs.com/test-catalog/Overview/7735); checked 2026-10-30.
- `uric`: [Mayo Clinic Laboratories: Uric Acid, Serum](https://www.mayocliniclabs.com/test-catalog/Overview/8440); checked 2026-10-30.
- `mercury`: [Mayo Clinic Laboratories: Mercury, Blood](https://pediatric.testcatalog.org/show/HG); checked 2026-10-30.
- `rbc_mg`: [Michigan Medicine MLabs: Magnesium, RBC](https://mlabs.umich.edu/tests/magnesium-rbc); checked 2026-10-30.
- `leptin`: [Mayo Clinic Laboratories / Esoterix: Leptin](https://www.mayocliniclabs.com/test-catalog/Overview/91339); checked 2026-10-30.
- `kdigo`: [KDIGO 2024 Clinical Practice Guideline for CKD, Kidney International 105 (Suppl 4S), S117–S314](https://kdigo.org/wp-content/uploads/2024/03/KDIGO-2024-CKD-Guideline.pdf); checked 2026-10-30.
- `vitd`: [NIH Office of Dietary Supplements: Vitamin D, Health Professional Fact Sheet](https://ods.od.nih.gov/factsheets/VITAMIND-HealthProfessional/); checked 2026-10-30.
- `bp`: [2025 AHA/ACC High Blood Pressure Guideline](https://professional.heart.org/en/science-news/2025-high-blood-pressure-guideline/top-things-to-know); checked 2026-10-30.
- `oxygen`: [MedlinePlus: Pulse Oximetry](https://medlineplus.gov/lab-tests/pulse-oximetry/); checked 2026-10-30.
- `resp`: [WHO / ICRC: Basic Emergency Care — Normal Adult Vital Signs](https://hlh.who.int/docs/librariesprovider4/hlh-documents/who-icrc-basic-emergency-care.pdf?sfvrsn=4460e22e_5); checked 2026-10-30.
- `uhd`: [University Hospitals Dorset NHS: Haematology Reference Ranges](https://www.uhd.nhs.uk/services/pathology/haematology/test-repertoire-and-reference-ranges); checked 2026-10-30.
- `quest_lipids`: [Quest Diagnostics: Advanced Cardiovascular Testing — ion mobility subfractions](https://www.questdiagnostics.com/content/dam/corporate/unrestricted/documents/other/cardiovascular/advanced_cardiovascular_testing_brochure/Advanced_Cardiovascular_Testing_Brochure.pdf); checked 2026-10-30.
- `pth`: [Mayo Clinic Laboratories: Parathyroid Hormone, Serum (PTH2)](https://www.mayocliniclabs.com/test-catalog/Overview/800172); checked 2026-10-30.
- `hrv`: [Nunan et al. A quantitative systematic review of normal values for short-term heart rate variability in healthy adults (2010)](https://pubmed.ncbi.nlm.nih.gov/20663071/); checked 2026-10-30.
- `walk`: [ERS/ATS technical standard: Field walking tests in chronic respiratory disease (2014)](https://www.thoracic.org/statements/resources/copd/FWT-Syst-Rev.pdf); checked 2026-10-30.
- `hearing`: [WHO–ITU: Safe listening devices and systems (2019)](https://www.who.int/publications/i/item/9789241515276); checked 2026-10-30.
- `dri`: [National Academies: Dietary Reference Intakes, Reference Tables](https://www.ncbi.nlm.nih.gov/books/NBK208874/); checked 2026-10-30.
- `urine`: [Mayo Clinic: Urinalysis](https://www.mayoclinic.org/tests-procedures/urinalysis/about/pac-20384907); checked 2026-10-30.

## Limits

Published laboratory ranges are laboratory- and method-dependent. Absence of a rule means no transferable rule has been verified for that identity and context; it is not a claim that no research exists. In particular, qualitative records, detailed ECG parameters, hormone assay differences and incomplete nutrition tracking are not forced into numeric normal/abnormal categories. Critical illness or treatment decisions must not be inferred from these UI labels.

Weight-, sex-, age-, phase-, timing- and assay-dependent contexts remain explicit in the API. The clients do not silently fill missing fasting, pregnancy or assay information.

## Initial release — 2026-10-30

- Backend image `longlivehealth-backend-api:reference-guides-20261030` passed formatting, Clippy and all backend tests: 210 passed, 2 intentionally ignored.
- Deployed that image to NAS on 2026-10-30. The authenticated catalog route correctly rejects unauthenticated requests with HTTP 401. No patient measurements were modified for verification.
- Android build 22 passed Docker compilation/package checks. Emulator checks covered the auto-saving reference profile, date picker, chart assessment, expandable intervals and source link. Build 22 was installed over the existing app on the USB-connected Android phone on 2026-10-31; package version and successful activity launch were verified.
- iOS build 75 passed compilation, 50 core tests and the targeted chart UI test. Device signing completed and build 75 was installed on the paired iPhone on 2026-10-31.
- The root build script subsequently failed while fetching the unchanged OCR image's `uv` base from GHCR (network timeout). API and APK builds had already completed; the running OCR service was not changed.

## Current verification — 2026-10-31

See [the complete NAS audit](../reviews/2026-10-31-complete-browse-reference-audit.md) for all 205 metric identities. Catalog version `2026-10-31.2` ships in the final dynamic-ranges image. Backend: 218 tests passed, 2 ignored; shared core: 53 passed; iOS chart UI regression passed. iOS 77 and Android 24 introduced selectable range bands and day counts. Device-level coverage is recorded separately in the audit; server audit coverage does not imply every phone screen has been verified.

- FRIEND 2022: [Updated Reference Standards for Cardiorespiratory Fitness Measured with Cardiopulmonary Exercise Testing](https://pubmed.ncbi.nlm.nih.gov/34809986/), Table 3; DOI 10.1016/j.mayocp.2021.08.020.

The subsequent Android 25 and iOS 78 builds correct the applicable interval unit label. The NAS `dynamic-ranges-20261031-verified` release also anchors measurement navigation to actual scalar/sample timestamps instead of record end dates. See the audit report for installation and physical-device verification status.

Browse sleep history and Overview use the same whole-session wake-date totals. Sleep history reads bounded chunks with preceding-night padding; it does not split nightly duration at midnight before applying a nightly duration guide.
