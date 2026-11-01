# Browse chart coverage — 2026-10-31

NAS audit uses actual account-scoped queries, with no patient measurements or identity data recorded here. All 804 period queries (67 metric types × H/D/W/M/6M/Y × recent/earliest history) succeeded. Seven full daily histories, 16 specialized record types, sleep sessions and all 150 laboratory histories succeeded. No query errors.

## Implementation

Correlations resolve child IDs within the same account, platform and source. Different measurements never compete in source-priority deduplication. Each component has a label, unit, visibility toggle and independent reference assessment. Different units use separate selectable plots. Latest values come from one source record, never a timestamp join.

Both clients share ECG record selection and voltage rendering. Gaps break the trace. GPS routes retain a dedicated position preview. Characteristic values and narrative records are not converted to numeric measurements.

Existing raw originals remain intact. Derived chart fields are rebuildable SQLite projections. Activity summaries exported by older clients lack explicit portable units; Build 86 adds explicit HealthKit conversions on the next snapshot sync. Their older untyped archive values are deliberately not guessed.

## Validation

224 backend tests passed (two existing opt-in tests ignored); 58 shared core tests passed. iOS simulator/device builds and Android Docker compilation, unit tests and lint passed. Android emulator verified paired blood-pressure rendering and switching between ECG recordings. iOS Build 86 and Android Build 34 installed. Android physical phone was locked during the final UI check; no physical interaction claim is made.

## Per-metric audit

| Metric | Display/query outcome |
|---|---|
| HKCategoryTypeIdentifierAppleStandHour | All 12 period queries passed; numeric chart available |
| HKCategoryTypeIdentifierAudioExposureEvent | All 12 period queries passed; numeric chart available |
| HKCategoryTypeIdentifierHypertensionEvent | All 12 period queries passed; numeric chart available |
| HKCategoryTypeIdentifierLowCardioFitnessEvent | All 12 period queries passed; numeric chart available |
| HKCategoryTypeIdentifierSleepApneaEvent | All 12 period queries passed; numeric chart available |
| HKCharacteristicTypeIdentifierActivityMoveMode | Latest value and history |
| HKCharacteristicTypeIdentifierBiologicalSex | Latest value and history |
| HKCharacteristicTypeIdentifierBloodType | Latest value and history |
| HKCharacteristicTypeIdentifierDateOfBirth | Latest value and history |
| HKCharacteristicTypeIdentifierFitzpatrickSkinType | Latest value and history |
| HKCharacteristicTypeIdentifierWheelchairUse | Latest value and history |
| HKClinicalTypeIdentifierAllergyRecord | Source records |
| HKClinicalTypeIdentifierClinicalNoteRecord | Source records |
| HKClinicalTypeIdentifierImmunizationRecord | Source records |
| HKClinicalTypeIdentifierLabResultRecord | Source records |
| HKClinicalTypeIdentifierMedicationRecord | Source records |
| HKClinicalTypeIdentifierProcedureRecord | Source records |
| HKClinicalTypeIdentifierVitalSignRecord | Source records |
| HKCorrelationTypeIdentifierBloodPressure | All 12 period queries passed; numeric chart available |
| HKCorrelationTypeIdentifierFood | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierAppleSleepingBreathingDisturbances | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierAppleSleepingWristTemperature | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierAppleWalkingSteadiness | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierBodyMassIndex | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierDietaryCarbohydrates | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierDietaryCholesterol | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierDietaryEnergyConsumed | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierDietaryFatTotal | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierDietaryFiber | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierDietaryProtein | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierDietarySodium | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierDietarySugar | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierDietaryWater | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierEnvironmentalAudioExposure | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierEnvironmentalSoundReduction | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierEstimatedWorkoutEffortScore | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierHeadphoneAudioExposure | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierHeartRateRecoveryOneMinute | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierHeartRateVariabilityRMSSD | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierPhysicalEffort | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierRunningGroundContactTime | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierRunningPower | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierRunningSpeed | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierRunningStrideLength | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierRunningVerticalOscillation | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierSixMinuteWalkTestDistance | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierStairAscentSpeed | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierStairDescentSpeed | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierTimeInDaylight | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierWalkingAsymmetryPercentage | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierWalkingDoubleSupportPercentage | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierWalkingStepLength | All 12 period queries passed; numeric chart available |
| HKQuantityTypeIdentifierWorkoutEffortScore | All 12 period queries passed; numeric chart available |
| abo_group | History retained; no exact numeric value with a valid date/unit |
| active_energy | All 12 period queries passed; numeric chart available |
| activity_summary | All 12 queries passed; explicit-unit activity snapshot refresh required |
| albumin | Numeric laboratory chart and history |
| albumin_globulin_ratio | History retained; no exact numeric value with a valid date/unit |
| alp | Numeric laboratory chart and history |
| alt | Numeric laboratory chart and history |
| amylase | Numeric laboratory chart and history |
| ana_screen | History retained; no exact numeric value with a valid date/unit |
| apob | Numeric laboratory chart and history |
| arachidonic_epa_ratio | Numeric laboratory chart and history |
| ast | Numeric laboratory chart and history |
| basal_energy | All 12 period queries passed; numeric chart available |
| basophil_count | Numeric laboratory chart and history |
| basophil_percent | Numeric laboratory chart and history |
| blood_pressure_diastolic | All 12 period queries passed; numeric chart available |
| blood_pressure_systolic | All 12 period queries passed; numeric chart available |
| body_mass | All 12 period queries passed; numeric chart available |
| bun | Numeric laboratory chart and history |
| calcium | Numeric laboratory chart and history |
| carbon_dioxide | Numeric laboratory chart and history |
| chloride | Numeric laboratory chart and history |
| cholesterol_hdl_ratio | History retained; no exact numeric value with a valid date/unit |
| clinical:01e4aaaa76263658ed469ed1ba93f54639c76ab3ec0f4d047c3617aeb7be1bc1 | Numeric laboratory chart and history |
| clinical:17dbbf9707d90174afa4c24991b5bb699bd3f6fffa855244704ede31f35d5334 | Numeric laboratory chart and history |
| clinical:315527890aee645f163f6b6c15b3b34ee8996ef93463f5f8599448e5b28e0b42 | Numeric laboratory chart and history |
| clinical:372380786a9f6bd054269a5eec72970f2f1889d610a6c9c84c7272491839ae08 | History retained; no exact numeric value with a valid date/unit |
| clinical:490428c3e27898159ffedd0cba2194dcbd3dd72008b481bc6591b3bc530236b3 | Numeric laboratory chart and history |
| clinical:4b10f1addcf0f58658edc62849b98b63f8de77c0f1ebffb0faf93a11386e9562 | History retained; no exact numeric value with a valid date/unit |
| clinical:5914e274c743ec724bc77a056c2a4432401103b09b1a39a1320cc7949e1cceb7 | History retained; no exact numeric value with a valid date/unit |
| clinical:599284c9549c3c631d6aee4ddabe2a400a698996e714ffe4cc69e767b6245417 | Numeric laboratory chart and history |
| clinical:794e151e93fa5b998d850a4eb7b59eada50f41fc6f17b533b1ccf2448b3913f9 | Numeric laboratory chart and history |
| clinical:7ab8a8adb6b2f4ec5ec4cb6fe7c9842f952dfea139183dc8c21d15a887f1c44e | Numeric laboratory chart and history |
| clinical:7f9e2995712b84d7aa5b752f20e27a2d4f72015b6a0d784d6034b28b94f2d1f9 | Numeric laboratory chart and history |
| clinical:8336834a573ddd1d45859c97950256a7003329770c6a7873aebf5c9cc34680e3 | History retained; no exact numeric value with a valid date/unit |
| clinical:850dc743685e182448de58e03d63cbe7c679b7960ac3886200d0fb7f8c509e22 | Numeric laboratory chart and history |
| clinical:87f15941a3b61f2b480675015e8801af40749d296a4dde44c600967162c80186 | Numeric laboratory chart and history |
| clinical:92333b0050780165d77dcab76904281da6ea4ac57ab7e493e9ddea646f85ddae | History retained; no exact numeric value with a valid date/unit |
| clinical:96feb2056ce4641c5589c7dcabebbb4c6c95eea22df875b679d3f5d8a76b27ab | Numeric laboratory chart and history |
| clinical:9ef34c0f79c13fad10027eaec6dfbb688d78d802651a6bacb2883b06fd7902d4 | Numeric laboratory chart and history |
| clinical:a335f19f51d64583e1f1f6152dc993c69bdbd1147c4d35663472d81acc74e7da | Numeric laboratory chart and history |
| clinical:b4208484a629babc4e17e8b4ba790e106aaef1b7ab4c62453b520a64e3575e43 | Numeric laboratory chart and history |
| clinical:b9eea2a3f386e9c03fa55e1b9dd53f349c18f4d5b0ea10ce29617c7d2ec6ce9f | Numeric laboratory chart and history |
| clinical:c4984e4189742ffb054258b800419c3906005e3971d0832dddb9f0feb36853e8 | Numeric laboratory chart and history |
| clinical:cba92749dfff0342bdf57a4ac09165636d5d94eb4695b2798c0bb9499782b817 | Numeric laboratory chart and history |
| clinical:d43ebdddabb6a0e1eae4e570efaffc9e3db46ba8da583cae1e011fa3dbf8893d | Numeric laboratory chart and history |
| clinical:e4c1ef2c51dac3bd5240dc82556330573bee7e98f8fceefa03c70bf38cc9bf0d | Numeric laboratory chart and history |
| clinical:e621607c60a56072a0d6a3da59611796baf6752f160eeda9ae691196faa99f02 | Numeric laboratory chart and history |
| clinical:e72d76d9f5efb4586c214350591df35164ed183acb1ead4ef5d79a15351775bb | Numeric laboratory chart and history |
| clinical:e7b48568d0b6fe08a4b78d6bdab729308b598f787608ca36106078807b6e6359 | Numeric laboratory chart and history |
| clinical:edd89ae78ca4fa80bef33fcbf1acceca239aba400eab37d16677d13d83293271 | Numeric laboratory chart and history |
| clinical:ef3b2d1f3d722a6d1b7d3306571a53a84560630f527c8127befcebbd735aa509 | Numeric laboratory chart and history |
| clinical:f28e573ddd37ffc1ee38662db937feb89ae2b5b4774ef704cceb46ab6b33c36b | Numeric laboratory chart and history |
| clinical:f38eaccb3e83d07a5708a91f68023580d25f873f68a32db5fbecbb7b1c927ecf | Numeric laboratory chart and history |
| clinical:fe2af07100962787d08e4bc6f8229aa20875d7598bf1a9f9a8869749012c822b | Numeric laboratory chart and history |
| clinical:ffe40976dbeabd18f3d048aa275e67dd00adf9412249c9b1735a42f7c739aa7f | Numeric laboratory chart and history |
| cortisol | Numeric laboratory chart and history |
| creatinine | Numeric laboratory chart and history |
| cycling_distance | All 12 period queries passed; numeric chart available |
| dhea_sulfate | Numeric laboratory chart and history |
| egfr_creatinine_unspecified | Numeric laboratory chart and history |
| electrocardiogram | ECG waveform and recording selector |
| eosinophil_count | Numeric laboratory chart and history |
| eosinophil_percent | Numeric laboratory chart and history |
| estradiol | History retained; no exact numeric value with a valid date/unit |
| exercise_time | All 12 period queries passed; numeric chart available |
| ferritin | Numeric laboratory chart and history |
| flights_climbed | All 12 period queries passed; numeric chart available |
| free_t3 | History retained; no exact numeric value with a valid date/unit |
| free_t4 | Numeric laboratory chart and history |
| fsh | Numeric laboratory chart and history |
| ggt | Numeric laboratory chart and history |
| gliadin_iga | History retained; no exact numeric value with a valid date/unit |
| gliadin_igg | Numeric laboratory chart and history |
| globulin | Numeric laboratory chart and history |
| glucose | Numeric laboratory chart and history |
| hba1c | Numeric laboratory chart and history |
| hdl_cholesterol | Numeric laboratory chart and history |
| hdl_large | Numeric laboratory chart and history |
| heart_rate | All 12 period queries passed; numeric chart available |
| heartbeat_series | All 12 period queries passed; numeric chart available |
| height | All 12 period queries passed; numeric chart available |
| hematocrit | Numeric laboratory chart and history |
| hemoglobin | Numeric laboratory chart and history |
| homocysteine | Numeric laboratory chart and history |
| hrv_sdnn | All 12 period queries passed; numeric chart available |
| hs_crp | Numeric laboratory chart and history |
| iga | Numeric laboratory chart and history |
| insulin | Numeric laboratory chart and history |
| iron | Numeric laboratory chart and history |
| iron_binding_capacity | Numeric laboratory chart and history |
| iron_saturation | Numeric laboratory chart and history |
| ldl_cholesterol | History retained; no exact numeric value with a valid date/unit |
| ldl_medium | Numeric laboratory chart and history |
| ldl_particle_number | Numeric laboratory chart and history |
| ldl_pattern | History retained; no exact numeric value with a valid date/unit |
| ldl_peak_size | Numeric laboratory chart and history |
| ldl_small | Numeric laboratory chart and history |
| lead | History retained; no exact numeric value with a valid date/unit |
| leptin | Numeric laboratory chart and history |
| lh | Numeric laboratory chart and history |
| lipase | Numeric laboratory chart and history |
| lipoprotein_a | History retained; no exact numeric value with a valid date/unit |
| lymphocyte_count | Numeric laboratory chart and history |
| lymphocyte_percent | Numeric laboratory chart and history |
| mch | Numeric laboratory chart and history |
| mchc | Numeric laboratory chart and history |
| mcv | Numeric laboratory chart and history |
| medication_dose_event | All 12 period queries passed; numeric chart available |
| mercury | Numeric laboratory chart and history |
| mindful_session | All 12 period queries passed; numeric chart available |
| mma | Numeric laboratory chart and history |
| monocyte_count | Numeric laboratory chart and history |
| monocyte_percent | Numeric laboratory chart and history |
| mpv | Numeric laboratory chart and history |
| neutrophil_count | Numeric laboratory chart and history |
| neutrophil_percent | Numeric laboratory chart and history |
| non_hdl_cholesterol | History retained; no exact numeric value with a valid date/unit |
| omega3_dha | Numeric laboratory chart and history |
| omega3_dpa | Numeric laboratory chart and history |
| omega3_epa | Numeric laboratory chart and history |
| omega3_total | Numeric laboratory chart and history |
| omega6_arachidonic | Numeric laboratory chart and history |
| omega6_linoleic | Numeric laboratory chart and history |
| omega6_total | Numeric laboratory chart and history |
| omega_6_3_ratio | Numeric laboratory chart and history |
| oxygen_saturation | All 12 period queries passed; numeric chart available |
| platelets | Numeric laboratory chart and history |
| potassium | Numeric laboratory chart and history |
| prolactin | Numeric laboratory chart and history |
| psa_free | Numeric laboratory chart and history |
| psa_free_percent | Numeric laboratory chart and history |
| psa_total | Numeric laboratory chart and history |
| rbc | Numeric laboratory chart and history |
| rbc_magnesium | Numeric laboratory chart and history |
| rdw | Numeric laboratory chart and history |
| respiratory_rate | All 12 period queries passed; numeric chart available |
| resting_heart_rate | All 12 period queries passed; numeric chart available |
| rh_factor | History retained; no exact numeric value with a valid date/unit |
| rheumatoid_factor | Numeric laboratory chart and history |
| shbg | Numeric laboratory chart and history |
| sleep | Daily chart and complete historical query passed |
| sodium | Numeric laboratory chart and history |
| stand_time | All 12 period queries passed; numeric chart available |
| state_of_mind | All 12 period queries passed; numeric chart available |
| steps | All 12 period queries passed; numeric chart available |
| testosterone_free | Numeric laboratory chart and history |
| testosterone_total | Numeric laboratory chart and history |
| thyroglobulin_antibodies | History retained; no exact numeric value with a valid date/unit |
| total_bilirubin | Numeric laboratory chart and history |
| total_cholesterol | Numeric laboratory chart and history |
| total_protein | Numeric laboratory chart and history |
| tpo_antibodies | History retained; no exact numeric value with a valid date/unit |
| triglycerides | Numeric laboratory chart and history |
| tsh | Numeric laboratory chart and history |
| ttg_iga | History retained; no exact numeric value with a valid date/unit |
| ttg_igg | History retained; no exact numeric value with a valid date/unit |
| uric_acid | Numeric laboratory chart and history |
| urine_albumin | Numeric laboratory chart and history |
| urine_appearance | History retained; no exact numeric value with a valid date/unit |
| urine_bacteria | History retained; no exact numeric value with a valid date/unit |
| urine_bilirubin | History retained; no exact numeric value with a valid date/unit |
| urine_color | History retained; no exact numeric value with a valid date/unit |
| urine_glucose | History retained; no exact numeric value with a valid date/unit |
| urine_hyaline_casts | History retained; no exact numeric value with a valid date/unit |
| urine_ketones | History retained; no exact numeric value with a valid date/unit |
| urine_leukocyte_esterase | History retained; no exact numeric value with a valid date/unit |
| urine_nitrite | History retained; no exact numeric value with a valid date/unit |
| urine_occult_blood | History retained; no exact numeric value with a valid date/unit |
| urine_protein | History retained; no exact numeric value with a valid date/unit |
| user_annotated_medications | Source records |
| vision_prescription | All 12 period queries passed; numeric chart available |
| vitamin_d_unspecified | Numeric laboratory chart and history |
| vo2_max | All 12 period queries passed; numeric chart available |
| walking_distance | All 12 period queries passed; numeric chart available |
| walking_heart_rate | All 12 period queries passed; numeric chart available |
| walking_speed | All 12 period queries passed; numeric chart available |
| wbc | Numeric laboratory chart and history |
| workout | All 12 period queries passed; numeric chart available |
| workout_route | Source records |
| zinc | Numeric laboratory chart and history |
