# Live Browse reference mapping audit — 2026-10-31

This is a mapping audit of all 232 live Browse entries, not a claim of new literature research for every entry. Laboratory-specific printed intervals remain attached to individual observations. Missing scalar rules need individual clinical review before adding thresholds.

| Metric | Guide mapping | Rule status / reason |
| --- | --- | --- |
| 25-hydroxyvitamin D3 | `clinical:e4c1ef2c51dac3bd5240dc82556330573bee7e98f8fceefa03c70bf38cc9bf0d` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| AB Screen | `clinical:372380786a9f6bd054269a5eec72970f2f1889d610a6c9c84c7272491839ae08` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| ABO group | `abo_group` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| ABORh | `clinical:4b10f1addcf0f58658edc62849b98b63f8de77c0f1ebffb0faf93a11386e9562` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Absolute basophil count | `basophil_count` | 1 numeric rules |
| Absolute eosinophil count | `eosinophil_count` | 1 numeric rules |
| Absolute lymphocyte count | `lymphocyte_count` | 1 numeric rules |
| Absolute monocyte count | `monocyte_count` | 1 numeric rules |
| Absolute neutrophil count | `neutrophil_count` | 1 numeric rules |
| Absolute NRBC | `clinical:b9eea2a3f386e9c03fa55e1b9dd53f349c18f4d5b0ea10ce29617c7d2ec6ce9f` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Activated Partial Thromboplastin Time | `clinical:e7b48568d0b6fe08a4b78d6bdab729308b598f787608ca36106078807b6e6359` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Active Energy | `active_energy` | Energy expenditure depends on body size, activity and recording completeness; no universal daily normal interval. |
| Activity Move Mode | `HKCharacteristicTypeIdentifierActivityMoveMode` | Categorical or composite record; a scalar normal range does not apply. |
| Activity Rings | `activity_summary` | Categorical or composite record; a scalar normal range does not apply. |
| Alanine aminotransferase | `alt` | 2 numeric rules |
| Albumin | `albumin` | 1 numeric rules |
| Albumin / globulin ratio | `albumin_globulin_ratio` | Use the laboratory interval; component methods and reference populations differ. |
| Albumin, random urine without creatinine | `urine_albumin` | Spot urine concentration depends on dilution. Do not apply albumin/creatinine-ratio or 24-hour thresholds to mg/dL. |
| Alkaline phosphatase | `alp` | 2 numeric rules |
| Allergy Record | `HKClinicalTypeIdentifierAllergyRecord` | Categorical or composite record; a scalar normal range does not apply. |
| Amylase | `amylase` | 1 numeric rules |
| Anion gap | `clinical:f28e573ddd37ffc1ee38662db937feb89ae2b5b4774ef704cceb46ab6b33c36b` | 1 numeric rules |
| Antinuclear antibodies screen | `ana_screen` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Apolipoprotein B | `apob` | 1 numeric rules |
| Appearance - urine | `urine_appearance` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Apple Exercise Time | `exercise_time` | No universal clinical interval established for this measurement protocol. |
| Apple Stand Hour | `HKCategoryTypeIdentifierAppleStandHour` | Categorical or composite record; a scalar normal range does not apply. |
| Apple Stand Time | `stand_time` | No universal clinical interval established for this measurement protocol. |
| Arachidonic acid/EPA ratio | `arachidonic_epa_ratio` | This is a numeric fatty-acid ratio. No transferable reference interval has been verified for this specimen and assay; use the original laboratory interval. |
| Aspartate aminotransferase | `ast` | 2 numeric rules |
| Atrial Rate | `clinical:9ef34c0f79c13fad10027eaec6dfbb688d78d802651a6bacb2883b06fd7902d4` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Audio Exposure Event | `HKCategoryTypeIdentifierAudioExposureEvent` | Hearing guidance uses sound level together with cumulative exposure duration. An isolated dB value cannot be assigned a safe exposure interval. |
| Bacteria - urine | `urine_bacteria` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Basophils % | `basophil_percent` | 1 numeric rules |
| Beat-to-beat Measurements | `heartbeat_series` | Categorical or composite record; a scalar normal range does not apply. |
| Bilirubin - urine | `urine_bilirubin` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Biological Sex | `HKCharacteristicTypeIdentifierBiologicalSex` | Categorical or composite record; a scalar normal range does not apply. |
| Blood Pressure | `HKCorrelationTypeIdentifierBloodPressure` | Systolic and diastolic component rules; removed unrecorded resting-context requirement. |
| Blood Type | `HKCharacteristicTypeIdentifierBloodType` | Categorical or composite record; a scalar normal range does not apply. |
| Blood urea nitrogen | `bun` | 2 numeric rules |
| Body Mass Index (BMI) | `HKQuantityTypeIdentifierBodyMassIndex` | 1 numeric rules |
| BUN / creatinine ratio | `clinical:7ab8a8adb6b2f4ec5ec4cb6fe7c9842f952dfea139183dc8c21d15a887f1c44e` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Calcium | `calcium` | 3 numeric rules |
| Carbon dioxide | `carbon_dioxide` | 1 numeric rules |
| Cardio Recovery | `HKQuantityTypeIdentifierHeartRateRecoveryOneMinute` | No universal clinical interval established for this measurement protocol. |
| Celiac - Deamidated gliadin (IgA) | `gliadin_iga` | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Celiac - Deamidated gliadin (IgG) | `gliadin_igg` | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Celiac - Tissue transglutaminase (IgA) | `ttg_iga` | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Celiac - Tissue transglutaminase (IgG) | `ttg_igg` | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Chloride | `chloride` | 1 numeric rules |
| Clinical Note Record | `HKClinicalTypeIdentifierClinicalNoteRecord` | Categorical or composite record; a scalar normal range does not apply. |
| Color - urine | `urine_color` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Cortisol | `cortisol` | 2 numeric rules |
| Creatinine | `creatinine` | 2 numeric rules |
| Creatinine-based estimated glomerular filtration rate (eGFR) | `egfr_creatinine_unspecified` | Equation is unspecified. A single eGFR does not establish chronic kidney disease; assess persistence and albuminuria. |
| Date Of Birth | `HKCharacteristicTypeIdentifierDateOfBirth` | Categorical or composite record; a scalar normal range does not apply. |
| DHEA sulfate | `dhea_sulfate` | 12 numeric rules |
| Dietary Carbohydrates | `HKQuantityTypeIdentifierDietaryCarbohydrates` | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Cholesterol | `HKQuantityTypeIdentifierDietaryCholesterol` | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Energy Consumed | `HKQuantityTypeIdentifierDietaryEnergyConsumed` | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Fat Total | `HKQuantityTypeIdentifierDietaryFatTotal` | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Fiber | `HKQuantityTypeIdentifierDietaryFiber` | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Protein | `HKQuantityTypeIdentifierDietaryProtein` | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Sodium | `HKQuantityTypeIdentifierDietarySodium` | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Sugar | `HKQuantityTypeIdentifierDietarySugar` | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Dietary Water | `HKQuantityTypeIdentifierDietaryWater` | Dietary recommendations apply to complete daily intake, not an individual food record. Recorded intake may omit foods; no deficient/excess status is inferred from incomplete tracking. |
| Direct bilirubin | `clinical:17dbbf9707d90174afa4c24991b5bb699bd3f6fffa855244704ede31f35d5334` | 1 numeric rules |
| Distance Cycling | `cycling_distance` | No universal clinical interval established for this measurement protocol. |
| Distance Walking Running | `walking_distance` | No universal clinical interval established for this measurement protocol. |
| ECG | `electrocardiogram` | Categorical or composite record; a scalar normal range does not apply. |
| eGFR (formula not specified) | `clinical:8336834a573ddd1d45859c97950256a7003329770c6a7873aebf5c9cc34680e3` | 1 numeric rules |
| eGFR (historical CKD-EPI, Black population adjustment) | `clinical:315527890aee645f163f6b6c15b3b34ee8996ef93463f5f8599448e5b28e0b42` | 1 numeric rules |
| eGFR (historical CKD-EPI, non-Black population adjustment) | `clinical:850dc743685e182448de58e03d63cbe7c679b7960ac3886200d0fb7f8c509e22` | 1 numeric rules |
| eGFR (historical MDRD) | `clinical:5914e274c743ec724bc77a056c2a4432401103b09b1a39a1320cc7949e1cceb7` | 1 numeric rules |
| Environmental Audio Exposure | `HKQuantityTypeIdentifierEnvironmentalAudioExposure` | Hearing guidance uses sound level together with cumulative exposure duration. An isolated dB value cannot be assigned a safe exposure interval. |
| Environmental Sound Reduction | `HKQuantityTypeIdentifierEnvironmentalSoundReduction` | Hearing guidance uses sound level together with cumulative exposure duration. An isolated dB value cannot be assigned a safe exposure interval. |
| Eosinophils % | `eosinophil_percent` | 1 numeric rules |
| Estimated Workout Effort Score | `HKQuantityTypeIdentifierEstimatedWorkoutEffortScore` | No universal clinical interval established for this measurement protocol. |
| Estradiol | `estradiol` | 5 numeric rules |
| Ferritin | `ferritin` | 3 numeric rules |
| Fitzpatrick Skin Type | `HKCharacteristicTypeIdentifierFitzpatrickSkinType` | Categorical or composite record; a scalar normal range does not apply. |
| Flights Climbed | `flights_climbed` | No universal clinical interval established for this measurement protocol. |
| Follicle stimulating hormone | `fsh` | 5 numeric rules |
| Food | `HKCorrelationTypeIdentifierFood` | Categorical or composite record; a scalar normal range does not apply. |
| Gamma-glutamyl transferase | `ggt` | 2 numeric rules |
| Globulin | `globulin` | 1 numeric rules |
| Glucose | `glucose` | 1 numeric rules |
| Glucose - urine | `urine_glucose` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| HbA1c (NGSP) | `hba1c` | 1 numeric rules |
| HDL cholesterol | `hdl_cholesterol` | 2 numeric rules |
| HDL large | `hdl_large` | 1 numeric rules |
| Headphone Audio Exposure | `HKQuantityTypeIdentifierHeadphoneAudioExposure` | Hearing guidance uses sound level together with cumulative exposure duration. An isolated dB value cannot be assigned a safe exposure interval. |
| Heart rate | `heart_rate` | Resting and exercise readings cannot share a normal interval. Use resting heart rate for the adult resting guide. |
| Height | `height` | Adult height is not a clinical in-range/out-of-range measurement. |
| Hematocrit | `hematocrit` | 2 numeric rules |
| Hemoglobin | `hemoglobin` | 2 numeric rules |
| High-sensitivity C-reactive protein | `hs_crp` | 1 numeric rules |
| Homocysteine | `homocysteine` | 1 numeric rules |
| HRV SDNN | `hrv_sdnn` | Recording duration, rhythm, activity and recovery protocol determine HRV interpretation. No protocol-independent clinical normal interval is applied. |
| Hyaline casts - urine | `urine_hyaline_casts` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Hypertension Event | `HKCategoryTypeIdentifierHypertensionEvent` | Categorical or composite record; a scalar normal range does not apply. |
| Imm Gran % | `clinical:c4984e4189742ffb054258b800419c3906005e3971d0832dddb9f0feb36853e8` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Imm Gran Abs | `clinical:96feb2056ce4641c5589c7dcabebbb4c6c95eea22df875b679d3f5d8a76b27ab` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Immunization Record | `HKClinicalTypeIdentifierImmunizationRecord` | Categorical or composite record; a scalar normal range does not apply. |
| Immunoglobulin A | `iga` | 1 numeric rules |
| Indirect bilirubin | `clinical:fe2af07100962787d08e4bc6f8229aa20875d7598bf1a9f9a8869749012c822b` | 1 numeric rules |
| Insulin | `insulin` | 1 numeric rules |
| Intact parathyroid hormone | `clinical:e72d76d9f5efb4586c214350591df35164ed183acb1ead4ef5d79a15351775bb` | 1 numeric rules |
| International Normalization Ratio | `clinical:490428c3e27898159ffedd0cba2194dcbd3dd72008b481bc6591b3bc530236b3` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Iron | `iron` | 2 numeric rules |
| Iron % saturation | `iron_saturation` | 1 numeric rules |
| Iron binding capacity | `iron_binding_capacity` | 1 numeric rules |
| Ketones - urine | `urine_ketones` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Lab Result Record | `HKClinicalTypeIdentifierLabResultRecord` | Categorical or composite record; a scalar normal range does not apply. |
| LDL cholesterol | `ldl_cholesterol` | 1 numeric rules |
| LDL medium | `ldl_medium` | 1 numeric rules |
| LDL particle number | `ldl_particle_number` | 1 numeric rules |
| LDL pattern | `ldl_pattern` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| LDL peak size | `ldl_peak_size` | Lipoprotein subfraction ranges depend on the assay (ion mobility versus NMR); do not transfer thresholds between methods. |
| LDL small | `ldl_small` | 1 numeric rules |
| Lead | `lead` | 1 numeric rules |
| Leptin | `leptin` | 2 numeric rules |
| Leukocyte esterase - urine | `urine_leukocyte_esterase` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Lipase | `lipase` | 1 numeric rules |
| Lipoprotein (a) | `lipoprotein_a` | 1 numeric rules |
| Low Cardio Fitness Event | `HKCategoryTypeIdentifierLowCardioFitnessEvent` | Categorical or composite record; a scalar normal range does not apply. |
| Luteinizing hormone | `lh` | 5 numeric rules |
| Lymphocytes % | `lymphocyte_percent` | 1 numeric rules |
| Magnesium (serum or plasma) | `clinical:a335f19f51d64583e1f1f6152dc993c69bdbd1147c4d35663472d81acc74e7da` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Magnesium, RBC | `rbc_magnesium` | 1 numeric rules |
| Mean corpuscular hemoglobin | `mch` | 1 numeric rules |
| Mean corpuscular hemoglobin concentration | `mchc` | 1 numeric rules |
| Mean corpuscular volume | `mcv` | 1 numeric rules |
| Mean platelet volume | `mpv` | MPV depends on analyzer, anticoagulant and sample processing time. Use the original laboratory interval. |
| Medication Doses | `medication_dose_event` | Categorical or composite record; a scalar normal range does not apply. |
| Medication Record | `HKClinicalTypeIdentifierMedicationRecord` | Categorical or composite record; a scalar normal range does not apply. |
| Medications | `user_annotated_medications` | Categorical or composite record; a scalar normal range does not apply. |
| Mercury | `mercury` | 1 numeric rules |
| Methylmalonic acid | `mma` | 1 numeric rules |
| Mindful Session | `mindful_session` | Categorical or composite record; a scalar normal range does not apply. |
| Monocytes % | `monocyte_percent` | 1 numeric rules |
| Neutrophils % | `neutrophil_percent` | 1 numeric rules |
| Nitrite - urine | `urine_nitrite` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Non-HDL cholesterol | `non_hdl_cholesterol` | Treatment goals depend on cardiovascular risk and the corresponding LDL goal; no single personal target is assumed. |
| Nucleated RBC Auto | `clinical:ef3b2d1f3d722a6d1b7d3306571a53a84560630f527c8127befcebbd735aa509` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Occult blood - urine | `urine_occult_blood` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Omega 6 / 3 ratio | `omega_6_3_ratio` | This is a numeric fatty-acid ratio. No transferable reference interval has been verified for this specimen and assay; use the original laboratory interval. |
| Omega-3 total | `omega3_total` | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-3: DHA | `omega3_dha` | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-3: DPA | `omega3_dpa` | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-3: EPA | `omega3_epa` | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-6 total | `omega6_total` | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-6: arachidonic acid | `omega6_arachidonic` | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Omega-6: linoleic acid | `omega6_linoleic` | Fatty-acid percentages and ratios depend on specimen and assay; no interchangeable universal clinical interval verified. |
| Oxygen saturation | `oxygen_saturation` | 1 numeric rules |
| P Axis | `clinical:ffe40976dbeabd18f3d048aa275e67dd00adf9412249c9b1735a42f7c739aa7f` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| P-R Interval | `clinical:7f9e2995712b84d7aa5b752f20e27a2d4f72015b6a0d784d6034b28b94f2d1f9` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Phosphate | `clinical:cba92749dfff0342bdf57a4ac09165636d5d94eb4695b2798c0bb9499782b817` | 1 numeric rules |
| Physical Effort | `HKQuantityTypeIdentifierPhysicalEffort` | No universal clinical interval established for this measurement protocol. |
| Platelet count | `platelets` | 2 numeric rules |
| Potassium | `potassium` | 1 numeric rules |
| Procedure Record | `HKClinicalTypeIdentifierProcedureRecord` | Categorical or composite record; a scalar normal range does not apply. |
| Prolactin | `prolactin` | 1 numeric rules |
| Prostate specific antigen %, free | `psa_free_percent` | Cancer-risk interpretation depends on concurrent total PSA, age and assay. |
| Prostate specific antigen, free | `psa_free` | Free PSA alone has no universal normal interval; interpretation requires concurrent total PSA. |
| Prostate specific antigen, total | `psa_total` | 6 numeric rules |
| Protein - urine | `urine_protein` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Prothrombin Time | `clinical:01e4aaaa76263658ed469ed1ba93f54639c76ab3ec0f4d047c3617aeb7be1bc1` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Q-T Interval | `clinical:87f15941a3b61f2b480675015e8801af40749d296a4dde44c600967162c80186` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| QRS Duration | `clinical:794e151e93fa5b998d850a4eb7b59eada50f41fc6f17b533b1ccf2448b3913f9` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| QTc | `clinical:d43ebdddabb6a0e1eae4e570efaffc9e3db46ba8da583cae1e011fa3dbf8893d` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| R Axis | `clinical:b4208484a629babc4e17e8b4ba790e106aaef1b7ab4c62453b520a64e3575e43` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Recovery HRV | `HKQuantityTypeIdentifierHeartRateVariabilityRMSSD` | No universal clinical interval established for this measurement protocol. |
| Red blood cell count | `rbc` | 2 numeric rules |
| Red cell distribution width (CV) | `rdw` | 2 numeric rules |
| Respiratory rate | `respiratory_rate` | 1 numeric rules |
| Resting Energy | `basal_energy` | Resting energy expenditure depends on body size and measurement method; no universal daily normal interval. |
| Resting heart rate | `resting_heart_rate` | 1 numeric rules |
| Rhesus (Rh) factor | `rh_factor` | Qualitative or categorical result. Preserve the printed interpretation rather than assign a numeric normal range. |
| Rheumatoid factor | `rheumatoid_factor` | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Running Ground Contact Time | `HKQuantityTypeIdentifierRunningGroundContactTime` | No universal clinical interval established for this measurement protocol. |
| Running Power | `HKQuantityTypeIdentifierRunningPower` | No universal clinical interval established for this measurement protocol. |
| Running Speed | `HKQuantityTypeIdentifierRunningSpeed` | No universal clinical interval established for this measurement protocol. |
| Running Stride Length | `HKQuantityTypeIdentifierRunningStrideLength` | No universal clinical interval established for this measurement protocol. |
| Running Vertical Oscillation | `HKQuantityTypeIdentifierRunningVerticalOscillation` | No universal clinical interval established for this measurement protocol. |
| SARS-CoV-2 NAAT | `clinical:92333b0050780165d77dcab76904281da6ea4ac57ab7e493e9ddea646f85ddae` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Sex hormone binding globulin | `shbg` | 2 numeric rules |
| Six Minute Walk Test Distance | `HKQuantityTypeIdentifierSixMinuteWalkTestDistance` | Reference equations require the standardized walking-test protocol, age, height, weight and population. A wearable estimate is not assumed equivalent to a supervised six-minute walk. |
| Sleep Apnea Events | `HKCategoryTypeIdentifierSleepApneaEvent` | Categorical or composite record; a scalar normal range does not apply. |
| Sleep sessions | `sleep` | 3 numeric rules |
| Sleeping Breathing Disturbances | `HKQuantityTypeIdentifierAppleSleepingBreathingDisturbances` | No universal clinical interval established for this measurement protocol. |
| Sleeping Wrist Temperature | `HKQuantityTypeIdentifierAppleSleepingWristTemperature` | No universal clinical interval established for this measurement protocol. |
| Sodium | `sodium` | 1 numeric rules |
| Stair Ascent Speed | `HKQuantityTypeIdentifierStairAscentSpeed` | No universal clinical interval established for this measurement protocol. |
| Stair Descent Speed | `HKQuantityTypeIdentifierStairDescentSpeed` | No universal clinical interval established for this measurement protocol. |
| State of Mind | `state_of_mind` | Categorical or composite record; a scalar normal range does not apply. |
| Steps | `steps` | Step counts are activity measures, not laboratory reference intervals. |
| T Axis | `clinical:e621607c60a56072a0d6a3da59611796baf6752f160eeda9ae691196faa99f02` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Testosterone, free | `testosterone_free` | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Testosterone, total | `testosterone_total` | 2 numeric rules |
| Thyroglobulin | `clinical:599284c9549c3c631d6aee4ddabe2a400a698996e714ffe4cc69e767b6245417` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Thyroglobulin antibodies | `thyroglobulin_antibodies` | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Thyroid peroxidase antibodies | `tpo_antibodies` | Assay-dependent thresholds require the exact method and laboratory interval; units alone do not establish comparability. |
| Thyroid-stimulating hormone | `tsh` | 1 numeric rules |
| Thyroxine (T4) free | `free_t4` | 1 numeric rules |
| Time In Daylight | `HKQuantityTypeIdentifierTimeInDaylight` | No universal clinical interval established for this measurement protocol. |
| Total 25-hydroxyvitamin D (D2 + D3) | `clinical:edd89ae78ca4fa80bef33fcbf1acceca239aba400eab37d16677d13d83293271` | 1 numeric rules |
| Total bilirubin | `total_bilirubin` | 1 numeric rules |
| Total cholesterol | `total_cholesterol` | 1 numeric rules |
| Total cholesterol / HDL ratio | `cholesterol_hdl_ratio` | Treatment decisions depend on the full lipid profile and overall cardiovascular risk, not a universal ratio cutoff. |
| Total protein | `total_protein` | 1 numeric rules |
| Triglycerides | `triglycerides` | 1 numeric rules |
| Triiodothyronine (T3) free | `free_t3` | 1 numeric rules |
| Uric acid | `uric_acid` | 2 numeric rules |
| Ventricular Rate | `clinical:f38eaccb3e83d07a5708a91f68023580d25f873f68a32db5fbecbb7b1c927ecf` | No transferable interval verified for this coded assay. Use its original reference interval; ECG intervals require rhythm, rate and correction-method context. |
| Vision Prescriptions | `vision_prescription` | Categorical or composite record; a scalar normal range does not apply. |
| Vital Sign Record | `HKClinicalTypeIdentifierVitalSignRecord` | Categorical or composite record; a scalar normal range does not apply. |
| Vitamin D | `vitamin_d_unspecified` | Vitamin D metabolite is unspecified. 25-hydroxy and 1,25-dihydroxy vitamin D require different intervals. |
| VO₂ Max | `vo2_max` | 14 numeric rules |
| Walking Asymmetry Percentage | `HKQuantityTypeIdentifierWalkingAsymmetryPercentage` | No universal clinical interval established for this measurement protocol. |
| Walking Double Support Percentage | `HKQuantityTypeIdentifierWalkingDoubleSupportPercentage` | No universal clinical interval established for this measurement protocol. |
| Walking Heart Rate | `walking_heart_rate` | No universal clinical interval established for this measurement protocol. |
| Walking Speed | `walking_speed` | No universal clinical interval established for this measurement protocol. |
| Walking Steadiness | `HKQuantityTypeIdentifierAppleWalkingSteadiness` | No universal clinical interval established for this measurement protocol. |
| Walking Step Length | `HKQuantityTypeIdentifierWalkingStepLength` | No universal clinical interval established for this measurement protocol. |
| Weight | `body_mass` | 1 numeric rules |
| Wheelchair Use | `HKCharacteristicTypeIdentifierWheelchairUse` | Categorical or composite record; a scalar normal range does not apply. |
| White blood cell count | `wbc` | 1 numeric rules |
| Workout Effort Score | `HKQuantityTypeIdentifierWorkoutEffortScore` | No universal clinical interval established for this measurement protocol. |
| Workout Routes | `workout_route` | Categorical or composite record; a scalar normal range does not apply. |
| Workouts | `workout` | Categorical or composite record; a scalar normal range does not apply. |
| Zinc | `zinc` | 3 numeric rules |
