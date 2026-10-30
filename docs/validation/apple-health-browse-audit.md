# Apple Health Browse audit — 2026-10-29

The 221-type registry is checked against every HealthCatalog entry and HealthSnapshots type, plus every quantity/category/correlation/clinical identifier in the installed iOS 27.1 SDK. This includes 121 numeric quantity types. Coverage includes empty types, which do not create empty extra Browse rows. Apple clinical insurance coverage is explicitly excluded as administrative. Dietary quantities remain separate from blood laboratory measurements.

| Type | Title | Stored records before repair | Numeric records before repair | Presentation |
|---|---|---:|---:|---|
| `HKCategoryTypeIdentifierAbdominalCramps` | Abdominal Cramps | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierAcne` | Acne | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierAppetiteChanges` | Appetite Changes | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierAppleStandHour` | Apple Stand Hour | 34,965 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierAppleWalkingSteadinessEvent` | Apple Walking Steadiness Event | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierAudioExposureEvent` | Audio Exposure Event | 86 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierBladderIncontinence` | Bladder Incontinence | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierBloating` | Bloating | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierBreastPain` | Breast Pain | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierCervicalMucusQuality` | Cervical Mucus Quality | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierChestTightnessOrPain` | Chest Tightness Or Pain | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierChills` | Chills | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierConstipation` | Constipation | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierContraceptive` | Contraceptive | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierCoughing` | Coughing | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierDiarrhea` | Diarrhea | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierDizziness` | Dizziness | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierDrySkin` | Dry Skin | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierEnvironmentalAudioExposureEvent` | Environmental Audio Exposure Event | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierFainting` | Fainting | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierFatigue` | Fatigue | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierFever` | Fever | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierGeneralizedBodyAche` | Generalized Body Ache | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierHairLoss` | Hair Loss | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierHandwashingEvent` | Handwashing Event | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierHeadache` | Headache | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierHeadphoneAudioExposureEvent` | Headphone Audio Exposure Event | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierHeartburn` | Heartburn | 0 | 0 | Paginated health record history |
| `high_heart_rate_event` | High Heart Rate Event | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierHotFlashes` | Hot Flashes | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierHypertensionEvent` | Hypertension Event | 12 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierInfrequentMenstrualCycles` | Infrequent Menstrual Cycles | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierIntermenstrualBleeding` | Intermenstrual Bleeding | 0 | 0 | Paginated health record history |
| `irregular_heart_rhythm_event` | Irregular Heart Rhythm Event | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierIrregularMenstrualCycles` | Irregular Menstrual Cycles | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierLactation` | Lactation | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierLossOfSmell` | Loss Of Smell | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierLossOfTaste` | Loss Of Taste | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierLowCardioFitnessEvent` | Low Cardio Fitness Event | 3 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierLowerBackPain` | Lower Back Pain | 0 | 0 | Paginated health record history |
| `low_heart_rate_event` | Low Heart Rate Event | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierMemoryLapse` | Memory Lapse | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierMenstrualFlow` | Menstrual Flow | 0 | 0 | Paginated health record history |
| `mindful_session` | Mindful Session | 35 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierMoodChanges` | Mood Changes | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierNausea` | Nausea | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierNightSweats` | Night Sweats | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierOvulationTestResult` | Ovulation Test Result | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierPelvicPain` | Pelvic Pain | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierPersistentIntermenstrualBleeding` | Persistent Intermenstrual Bleeding | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierPregnancy` | Pregnancy | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierPregnancyTestResult` | Pregnancy Test Result | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierProgesteroneTestResult` | Progesterone Test Result | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierProlongedMenstrualPeriods` | Prolonged Menstrual Periods | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierRapidPoundingOrFlutteringHeartbeat` | Rapid Pounding Or Fluttering Heartbeat | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierRunnyNose` | Runny Nose | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierSexualActivity` | Sexual Activity | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierShortnessOfBreath` | Shortness Of Breath | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierSinusCongestion` | Sinus Congestion | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierSkippedHeartbeat` | Skipped Heartbeat | 0 | 0 | Paginated health record history |
| `sleep` | Sleep Analysis | 45,828 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierSleepChanges` | Sleep Changes | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierSoreThroat` | Sore Throat | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierToothbrushingEvent` | Toothbrushing Event | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierVaginalDryness` | Vaginal Dryness | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierVomiting` | Vomiting | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierWheezing` | Wheezing | 0 | 0 | Paginated health record history |
| `HKClinicalTypeIdentifierAllergyRecord` | Allergy Record | 1 | 0 | Paginated health record history |
| `HKClinicalTypeIdentifierClinicalNoteRecord` | Clinical Note Record | 13 | 0 | Paginated health record history |
| `HKClinicalTypeIdentifierConditionRecord` | Condition Record | 0 | 0 | Paginated health record history |
| `HKClinicalTypeIdentifierCoverageRecord` | Coverage Record | 0 | 0 | Excluded: administrative |
| `HKClinicalTypeIdentifierImmunizationRecord` | Immunization Record | 5 | 0 | Paginated health record history |
| `HKClinicalTypeIdentifierLabResultRecord` | Lab Result Record | 309 | 0 | Paginated health record history |
| `HKClinicalTypeIdentifierMedicationRecord` | Medication Record | 59 | 0 | Paginated health record history |
| `HKClinicalTypeIdentifierProcedureRecord` | Procedure Record | 2 | 0 | Paginated health record history |
| `HKClinicalTypeIdentifierVitalSignRecord` | Vital Sign Record | 199 | 0 | Paginated health record history |
| `HKCorrelationTypeIdentifierBloodPressure` | Blood Pressure | 361 | 0 | Paginated health record history |
| `HKCorrelationTypeIdentifierFood` | Food | 2 | 0 | Paginated health record history |
| `active_energy` | Active Energy Burned | 883,197 | 883,197 | Chart and paginated history |
| `exercise_time` | Apple Exercise Time | 68,988 | 68,988 | Chart and paginated history |
| `HKQuantityTypeIdentifierAppleMoveTime` | Apple Move Time | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierAppleSleepingBreathingDisturbances` | Sleeping Breathing Disturbances | 758 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierAppleSleepingWristTemperature` | Sleeping Wrist Temperature | 991 | 0 | Chart and paginated history |
| `stand_time` | Apple Stand Time | 60,595 | 60,595 | Chart and paginated history |
| `HKQuantityTypeIdentifierAppleWalkingSteadiness` | Walking Steadiness | 263 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierAtrialFibrillationBurden` | Atrial Fibrillation Burden | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierBasalBodyTemperature` | Basal Body Temperature | 0 | 0 | Chart and paginated history |
| `basal_energy` | Basal Energy Burned | 499,382 | 499,382 | Chart and paginated history |
| `HKQuantityTypeIdentifierBloodAlcoholContent` | Blood Alcohol Content | 0 | 0 | Chart and paginated history |
| `blood_glucose` | Blood Glucose | 0 | 0 | Chart and paginated history |
| `blood_pressure_diastolic` | Diastolic Blood Pressure | 429 | 429 | Chart and paginated history |
| `blood_pressure_systolic` | Systolic Blood Pressure | 429 | 429 | Chart and paginated history |
| `body_fat` | Body Fat Percentage | 0 | 0 | Chart and paginated history |
| `body_mass` | Body Mass | 35,925 | 35,925 | Chart and paginated history |
| `HKQuantityTypeIdentifierBodyMassIndex` | Body Mass Index (BMI) | 35,908 | 0 | Chart and paginated history |
| `body_temperature` | Body Temperature | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierCrossCountrySkiingSpeed` | Cross Country Skiing Speed | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierCyclingCadence` | Cycling Cadence | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierCyclingFunctionalThresholdPower` | Cycling Functional Threshold Power | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierCyclingPower` | Cycling Power | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierCyclingSpeed` | Cycling Speed | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryBiotin` | Dietary Biotin | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryCaffeine` | Dietary Caffeine | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryCalcium` | Dietary Calcium | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryCarbohydrates` | Dietary Carbohydrates | 2 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryChloride` | Dietary Chloride | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryCholesterol` | Dietary Cholesterol | 2 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryChromium` | Dietary Chromium | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryCopper` | Dietary Copper | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryEnergyConsumed` | Dietary Energy Consumed | 2 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryFatMonounsaturated` | Dietary Fat Monounsaturated | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryFatPolyunsaturated` | Dietary Fat Polyunsaturated | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryFatSaturated` | Dietary Fat Saturated | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryFatTotal` | Dietary Fat Total | 2 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryFiber` | Dietary Fiber | 2 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryFolate` | Dietary Folate | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryIodine` | Dietary Iodine | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryIron` | Dietary Iron | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryMagnesium` | Dietary Magnesium | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryManganese` | Dietary Manganese | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryMolybdenum` | Dietary Molybdenum | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryNiacin` | Dietary Niacin | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryPantothenicAcid` | Dietary Pantothenic Acid | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryPhosphorus` | Dietary Phosphorus | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryPotassium` | Dietary Potassium | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryProtein` | Dietary Protein | 2 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryRiboflavin` | Dietary Riboflavin | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietarySelenium` | Dietary Selenium | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietarySodium` | Dietary Sodium | 2 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietarySugar` | Dietary Sugar | 1 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryThiamin` | Dietary Thiamin | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryVitaminA` | Dietary Vitamin A | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryVitaminB12` | Dietary Vitamin B12 | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryVitaminB6` | Dietary Vitamin B6 | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryVitaminC` | Dietary Vitamin C | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryVitaminD` | Dietary Vitamin D | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryVitaminE` | Dietary Vitamin E | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryVitaminK` | Dietary Vitamin K | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryWater` | Dietary Water | 48 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDietaryZinc` | Dietary Zinc | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDistanceCrossCountrySkiing` | Distance Cross Country Skiing | 0 | 0 | Chart and paginated history |
| `cycling_distance` | Distance Cycling | 1,115 | 1,115 | Chart and paginated history |
| `HKQuantityTypeIdentifierDistanceDownhillSnowSports` | Distance Downhill Snow Sports | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDistancePaddleSports` | Distance Paddle Sports | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDistanceRowing` | Distance Rowing | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDistanceSkatingSports` | Distance Skating Sports | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierDistanceSwimming` | Distance Swimming | 0 | 0 | Chart and paginated history |
| `walking_distance` | Distance Walking Running | 356,319 | 356,319 | Chart and paginated history |
| `HKQuantityTypeIdentifierDistanceWheelchair` | Distance Wheelchair | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierElectrodermalActivity` | Electrodermal Activity | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierEnvironmentalAudioExposure` | Environmental Audio Exposure | 62,619 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierEnvironmentalSoundReduction` | Environmental Sound Reduction | 1,749 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierEstimatedWorkoutEffortScore` | Estimated Workout Effort Score | 95 | 0 | Chart and paginated history |
| `flights_climbed` | Flights Climbed | 9,600 | 9,600 | Chart and paginated history |
| `HKQuantityTypeIdentifierForcedExpiratoryVolume1` | Forced Expiratory Volume1 | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierForcedVitalCapacity` | Forced Vital Capacity | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierHeadphoneAudioExposure` | Headphone Audio Exposure | 2,531 | 0 | Chart and paginated history |
| `heart_rate` | Heart Rate | 1,023,358 | 1,023,358 | Chart and paginated history |
| `HKQuantityTypeIdentifierHeartRateRecoveryOneMinute` | Cardio Recovery | 48 | 0 | Chart and paginated history |
| `hrv_sdnn` | Heart Rate Variability SDNN | 22,181 | 22,181 | Chart and paginated history |
| `height` | Height | 1 | 1 | Chart and paginated history |
| `HKQuantityTypeIdentifierInhalerUsage` | Inhaler Usage | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierInsulinDelivery` | Insulin Delivery | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierLeanBodyMass` | Lean Body Mass | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierNikeFuel` | Nike Fuel | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierNumberOfAlcoholicBeverages` | Number Of Alcoholic Beverages | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierNumberOfTimesFallen` | Number Of Times Fallen | 0 | 0 | Chart and paginated history |
| `oxygen_saturation` | Oxygen Saturation | 18,670 | 18,670 | Chart and paginated history |
| `HKQuantityTypeIdentifierPaddleSportsSpeed` | Paddle Sports Speed | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierPeakExpiratoryFlowRate` | Peak Expiratory Flow Rate | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierPeripheralPerfusionIndex` | Peripheral Perfusion Index | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierPhysicalEffort` | Physical Effort | 349,206 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierPushCount` | Push Count | 0 | 0 | Chart and paginated history |
| `respiratory_rate` | Respiratory Rate | 46,285 | 46,285 | Chart and paginated history |
| `resting_heart_rate` | Resting Heart Rate | 1,967 | 1,967 | Chart and paginated history |
| `HKQuantityTypeIdentifierRowingSpeed` | Rowing Speed | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierRunningGroundContactTime` | Running Ground Contact Time | 77 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierRunningPower` | Running Power | 185 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierRunningSpeed` | Running Speed | 282 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierRunningStrideLength` | Running Stride Length | 75 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierRunningVerticalOscillation` | Running Vertical Oscillation | 77 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierSixMinuteWalkTestDistance` | Six Minute Walk Test Distance | 159 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierStairAscentSpeed` | Stair Ascent Speed | 2,840 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierStairDescentSpeed` | Stair Descent Speed | 2,098 | 0 | Chart and paginated history |
| `steps` | Step Count | 370,948 | 370,948 | Chart and paginated history |
| `HKQuantityTypeIdentifierSwimmingStrokeCount` | Swimming Stroke Count | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierTimeInDaylight` | Time In Daylight | 13,944 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierUnderwaterDepth` | Underwater Depth | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierUVExposure` | UV Exposure | 0 | 0 | Chart and paginated history |
| `vo2_max` | VO2 Max | 298 | 298 | Chart and paginated history |
| `HKQuantityTypeIdentifierWaistCircumference` | Waist Circumference | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierWalkingAsymmetryPercentage` | Walking Asymmetry Percentage | 18,658 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierWalkingDoubleSupportPercentage` | Walking Double Support Percentage | 34,898 | 0 | Chart and paginated history |
| `walking_heart_rate` | Walking Heart Rate | 1,675 | 1,675 | Chart and paginated history |
| `walking_speed` | Walking Speed | 39,973 | 39,973 | Chart and paginated history |
| `HKQuantityTypeIdentifierWalkingStepLength` | Walking Step Length | 39,966 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierWaterTemperature` | Water Temperature | 0 | 0 | Chart and paginated history |
| `HKQuantityTypeIdentifierWorkoutEffortScore` | Workout Effort Score | 206 | 0 | Chart and paginated history |
| `HKCategoryTypeIdentifierBleedingAfterMenopause` | Bleeding After Menopause | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierMenopausalState` | Menopausal State | 0 | 0 | Paginated health record history |
| `HKQuantityTypeIdentifierHeartRateVariabilityRMSSD` | Recovery HRV | 7,697 | 7,697 | Chart and paginated history |
| `workout` | Workouts | 753 | 0 | Paginated health record history |
| `electrocardiogram` | ECG | 71 | 0 | Paginated health record history |
| `heartbeat_series` | Beat-to-beat Measurements | 14,625 | 0 | Paginated health record history |
| `workout_route` | Workout Routes | 224 | 0 | Paginated health record history |
| `audiogram` | Audiograms | 0 | 0 | Paginated health record history |
| `vision_prescription` | Vision Prescriptions | 1 | 0 | Paginated health record history |
| `state_of_mind` | State of Mind | 22 | 0 | Paginated health record history |
| `medication_dose_event` | Medication Doses | 6,772 | 0 | Paginated health record history |
| `activity_summary` | Activity Rings | 7,102 | 0 | Paginated health record history |
| `cda_document` | Clinical Documents | 0 | 0 | Paginated health record history |
| `user_annotated_medications` | Medications | 1 | 0 | Paginated health record history |
| `HKCharacteristicTypeIdentifierBiologicalSex` | Biological Sex | 3 | 3 | Paginated health record history |
| `HKCharacteristicTypeIdentifierBloodType` | Blood Type | 3 | 3 | Paginated health record history |
| `HKCharacteristicTypeIdentifierDateOfBirth` | Date Of Birth | 3 | 0 | Paginated health record history |
| `HKCharacteristicTypeIdentifierFitzpatrickSkinType` | Fitzpatrick Skin Type | 3 | 3 | Paginated health record history |
| `HKCharacteristicTypeIdentifierWheelchairUse` | Wheelchair Use | 3 | 3 | Paginated health record history |
| `HKCharacteristicTypeIdentifierActivityMoveMode` | Activity Move Mode | 3 | 3 | Paginated health record history |
| `HKCategoryTypeIdentifierBleedingAfterPregnancy` | Bleeding After Pregnancy | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierBleedingDuringPregnancy` | Bleeding During Pregnancy | 0 | 0 | Paginated health record history |
| `HKCategoryTypeIdentifierSleepApneaEvent` | Sleep Apnea Events | 0 | 0 | Paginated health record history |

## Validation and repair

- NAS original inventory: 83 Apple Health record types, including 53 quantity types. 32 quantity types lacked scalar numeric projections (567,696 records).
- Online repair updated 576,928 records across 38 types, also restoring nested quantity samples. No projection parse failures. Originals, synchronization identities and anchors were preserved.
- Backend: 197 tests passed, 2 intentionally ignored; formatting and Clippy passed. Every one of 121 quantity types is exercised through six chart periods and historical values. Event history pagination and account isolation are covered.
- iOS: 47 shared tests and 29 native tests passed; simulator acceptance verified BMI, systolic blood pressure, physical effort and stand-hour history. Build 66 installed and device version verified.
- Android: emulator verified those quantity charts, year selection, source toggles and stand-hour details. Fresh-install login fields were verified empty, with no stored session. No Android physical-device claim is made.
- API image: `longlivehealth-backend-api:apple-browse-apk-20261029`. The APK is embedded in the image and available at `/longlivehealth.apk`.

Repair counts (updated / unparsed):

| Type | Updated | Unparsed |
|---|---:|---:|
| `HKQuantityTypeIdentifierAppleSleepingBreathingDisturbances` | 758 | 0 |
| `HKQuantityTypeIdentifierAppleSleepingWristTemperature` | 991 | 0 |
| `HKQuantityTypeIdentifierAppleWalkingSteadiness` | 263 | 0 |
| `HKQuantityTypeIdentifierBodyMassIndex` | 35,908 | 0 |
| `HKQuantityTypeIdentifierDietaryCarbohydrates` | 2 | 0 |
| `HKQuantityTypeIdentifierDietaryCholesterol` | 2 | 0 |
| `HKQuantityTypeIdentifierDietaryEnergyConsumed` | 2 | 0 |
| `HKQuantityTypeIdentifierDietaryFatTotal` | 2 | 0 |
| `HKQuantityTypeIdentifierDietaryFiber` | 2 | 0 |
| `HKQuantityTypeIdentifierDietaryProtein` | 2 | 0 |
| `HKQuantityTypeIdentifierDietarySodium` | 2 | 0 |
| `HKQuantityTypeIdentifierDietarySugar` | 1 | 0 |
| `HKQuantityTypeIdentifierDietaryWater` | 48 | 0 |
| `HKQuantityTypeIdentifierEnvironmentalAudioExposure` | 62,619 | 0 |
| `HKQuantityTypeIdentifierEnvironmentalSoundReduction` | 1,749 | 0 |
| `HKQuantityTypeIdentifierEstimatedWorkoutEffortScore` | 95 | 0 |
| `HKQuantityTypeIdentifierHeadphoneAudioExposure` | 2,531 | 0 |
| `HKQuantityTypeIdentifierHeartRateRecoveryOneMinute` | 48 | 0 |
| `HKQuantityTypeIdentifierPhysicalEffort` | 349,206 | 0 |
| `HKQuantityTypeIdentifierRunningGroundContactTime` | 77 | 0 |
| `HKQuantityTypeIdentifierRunningPower` | 185 | 0 |
| `HKQuantityTypeIdentifierRunningSpeed` | 282 | 0 |
| `HKQuantityTypeIdentifierRunningStrideLength` | 75 | 0 |
| `HKQuantityTypeIdentifierRunningVerticalOscillation` | 77 | 0 |
| `HKQuantityTypeIdentifierSixMinuteWalkTestDistance` | 159 | 0 |
| `HKQuantityTypeIdentifierStairAscentSpeed` | 2,840 | 0 |
| `HKQuantityTypeIdentifierStairDescentSpeed` | 2,098 | 0 |
| `HKQuantityTypeIdentifierTimeInDaylight` | 13,944 | 0 |
| `HKQuantityTypeIdentifierWalkingAsymmetryPercentage` | 18,658 | 0 |
| `HKQuantityTypeIdentifierWalkingDoubleSupportPercentage` | 34,898 | 0 |
| `HKQuantityTypeIdentifierWalkingStepLength` | 39,966 | 0 |
| `HKQuantityTypeIdentifierWorkoutEffortScore` | 206 | 0 |
| `active_energy` | 3,802 | 0 |
| `basal_energy` | 2,462 | 0 |
| `cycling_distance` | 20 | 0 |
| `oxygen_saturation` | 9 | 0 |
| `steps` | 252 | 0 |
| `walking_distance` | 2,687 | 0 |

Post-repair inventory: all 4,128,197 undeleted Apple Health records remain present, across 83 record types. All 53 stored quantity types now have numeric projections for every record; no unknown stored record type is outside the registry.

| Stored type | Records after repair | Numeric projections after repair |
|---|---:|---:|
| `HKCategoryTypeIdentifierAppleStandHour` | 34,965 | 0 |
| `HKCategoryTypeIdentifierAudioExposureEvent` | 86 | 0 |
| `HKCategoryTypeIdentifierHypertensionEvent` | 12 | 0 |
| `HKCategoryTypeIdentifierLowCardioFitnessEvent` | 3 | 0 |
| `HKCharacteristicTypeIdentifierActivityMoveMode` | 3 | 3 |
| `HKCharacteristicTypeIdentifierBiologicalSex` | 3 | 3 |
| `HKCharacteristicTypeIdentifierBloodType` | 3 | 3 |
| `HKCharacteristicTypeIdentifierDateOfBirth` | 3 | 0 |
| `HKCharacteristicTypeIdentifierFitzpatrickSkinType` | 3 | 3 |
| `HKCharacteristicTypeIdentifierWheelchairUse` | 3 | 3 |
| `HKClinicalTypeIdentifierAllergyRecord` | 1 | 0 |
| `HKClinicalTypeIdentifierClinicalNoteRecord` | 13 | 0 |
| `HKClinicalTypeIdentifierImmunizationRecord` | 5 | 0 |
| `HKClinicalTypeIdentifierLabResultRecord` | 309 | 0 |
| `HKClinicalTypeIdentifierMedicationRecord` | 59 | 0 |
| `HKClinicalTypeIdentifierProcedureRecord` | 2 | 0 |
| `HKClinicalTypeIdentifierVitalSignRecord` | 199 | 0 |
| `HKCorrelationTypeIdentifierBloodPressure` | 361 | 0 |
| `HKCorrelationTypeIdentifierFood` | 2 | 0 |
| `HKQuantityTypeIdentifierAppleSleepingBreathingDisturbances` | 758 | 758 |
| `HKQuantityTypeIdentifierAppleSleepingWristTemperature` | 991 | 991 |
| `HKQuantityTypeIdentifierAppleWalkingSteadiness` | 263 | 263 |
| `HKQuantityTypeIdentifierBodyMassIndex` | 35,908 | 35,908 |
| `HKQuantityTypeIdentifierDietaryCarbohydrates` | 2 | 2 |
| `HKQuantityTypeIdentifierDietaryCholesterol` | 2 | 2 |
| `HKQuantityTypeIdentifierDietaryEnergyConsumed` | 2 | 2 |
| `HKQuantityTypeIdentifierDietaryFatTotal` | 2 | 2 |
| `HKQuantityTypeIdentifierDietaryFiber` | 2 | 2 |
| `HKQuantityTypeIdentifierDietaryProtein` | 2 | 2 |
| `HKQuantityTypeIdentifierDietarySodium` | 2 | 2 |
| `HKQuantityTypeIdentifierDietarySugar` | 1 | 1 |
| `HKQuantityTypeIdentifierDietaryWater` | 48 | 48 |
| `HKQuantityTypeIdentifierEnvironmentalAudioExposure` | 62,619 | 62,619 |
| `HKQuantityTypeIdentifierEnvironmentalSoundReduction` | 1,749 | 1,749 |
| `HKQuantityTypeIdentifierEstimatedWorkoutEffortScore` | 95 | 95 |
| `HKQuantityTypeIdentifierHeadphoneAudioExposure` | 2,531 | 2,531 |
| `HKQuantityTypeIdentifierHeartRateRecoveryOneMinute` | 48 | 48 |
| `HKQuantityTypeIdentifierHeartRateVariabilityRMSSD` | 7,697 | 7,697 |
| `HKQuantityTypeIdentifierPhysicalEffort` | 349,206 | 349,206 |
| `HKQuantityTypeIdentifierRunningGroundContactTime` | 77 | 77 |
| `HKQuantityTypeIdentifierRunningPower` | 185 | 185 |
| `HKQuantityTypeIdentifierRunningSpeed` | 282 | 282 |
| `HKQuantityTypeIdentifierRunningStrideLength` | 75 | 75 |
| `HKQuantityTypeIdentifierRunningVerticalOscillation` | 77 | 77 |
| `HKQuantityTypeIdentifierSixMinuteWalkTestDistance` | 159 | 159 |
| `HKQuantityTypeIdentifierStairAscentSpeed` | 2,840 | 2,840 |
| `HKQuantityTypeIdentifierStairDescentSpeed` | 2,098 | 2,098 |
| `HKQuantityTypeIdentifierTimeInDaylight` | 13,944 | 13,944 |
| `HKQuantityTypeIdentifierWalkingAsymmetryPercentage` | 18,658 | 18,658 |
| `HKQuantityTypeIdentifierWalkingDoubleSupportPercentage` | 34,898 | 34,898 |
| `HKQuantityTypeIdentifierWalkingStepLength` | 39,966 | 39,966 |
| `HKQuantityTypeIdentifierWorkoutEffortScore` | 206 | 206 |
| `active_energy` | 883,197 | 883,197 |
| `activity_summary` | 7,102 | 0 |
| `basal_energy` | 499,382 | 499,382 |
| `blood_pressure_diastolic` | 429 | 429 |
| `blood_pressure_systolic` | 429 | 429 |
| `body_mass` | 35,925 | 35,925 |
| `cycling_distance` | 1,115 | 1,115 |
| `electrocardiogram` | 71 | 0 |
| `exercise_time` | 68,988 | 68,988 |
| `flights_climbed` | 9,600 | 9,600 |
| `heart_rate` | 1,023,358 | 1,023,358 |
| `heartbeat_series` | 14,625 | 0 |
| `height` | 1 | 1 |
| `hrv_sdnn` | 22,181 | 22,181 |
| `medication_dose_event` | 6,772 | 0 |
| `mindful_session` | 35 | 0 |
| `oxygen_saturation` | 18,670 | 18,670 |
| `respiratory_rate` | 46,285 | 46,285 |
| `resting_heart_rate` | 1,967 | 1,967 |
| `sleep` | 45,828 | 0 |
| `stand_time` | 60,595 | 60,595 |
| `state_of_mind` | 22 | 0 |
| `steps` | 370,948 | 370,948 |
| `user_annotated_medications` | 1 | 0 |
| `vision_prescription` | 1 | 0 |
| `vo2_max` | 298 | 298 |
| `walking_distance` | 356,319 | 356,319 |
| `walking_heart_rate` | 1,675 | 1,675 |
| `walking_speed` | 39,973 | 39,973 |
| `workout` | 753 | 0 |
| `workout_route` | 224 | 0 |

## Time-bound query performance

The original mixed `OR` condition and aggregate allowed SQLite to choose the source index and scan the user’s full history repeatedly. The replacement queries the first and last undeleted record through the existing range/end indexes, then combines the manual-weight branch when applicable. No new index or schema migration is required. NAS comparison verified identical bounds for heart rate, body mass, BMI and physical effort; each old query took 1.28–1.44 seconds, while the replacement took 0.0007–0.007 seconds. This measures the bounds query, not total chart request latency. A regression covers deleted outliers, manual weight, a manual entry without weight and empty metrics.

## Final NAS query verification

The deployed API implementation completed 636 real-data measurement chart queries: all 53 stored quantity types × six periods (H, D, W, M, 6M, Y) × current and earliest historical windows. Every response reported complete coverage, zero excluded samples and no errors. Every type produced chart bars in at least one tested window; empty current windows correctly remained empty when no readings occurred there. Existing full-history daily queries, sleep sessions and 150 laboratory histories also completed without errors.

| Quantity type | Period queries | Complete | Excluded samples |
|---|---:|---:|---:|
| `HKQuantityTypeIdentifierAppleSleepingBreathingDisturbances` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierAppleSleepingWristTemperature` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierAppleWalkingSteadiness` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierBodyMassIndex` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierDietaryCarbohydrates` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierDietaryCholesterol` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierDietaryEnergyConsumed` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierDietaryFatTotal` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierDietaryFiber` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierDietaryProtein` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierDietarySodium` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierDietarySugar` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierDietaryWater` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierEnvironmentalAudioExposure` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierEnvironmentalSoundReduction` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierEstimatedWorkoutEffortScore` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierHeadphoneAudioExposure` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierHeartRateRecoveryOneMinute` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierHeartRateVariabilityRMSSD` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierPhysicalEffort` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierRunningGroundContactTime` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierRunningPower` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierRunningSpeed` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierRunningStrideLength` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierRunningVerticalOscillation` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierSixMinuteWalkTestDistance` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierStairAscentSpeed` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierStairDescentSpeed` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierTimeInDaylight` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierWalkingAsymmetryPercentage` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierWalkingDoubleSupportPercentage` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierWalkingStepLength` | 12 | 12 | 0 |
| `HKQuantityTypeIdentifierWorkoutEffortScore` | 12 | 12 | 0 |
| `active_energy` | 12 | 12 | 0 |
| `basal_energy` | 12 | 12 | 0 |
| `blood_pressure_diastolic` | 12 | 12 | 0 |
| `blood_pressure_systolic` | 12 | 12 | 0 |
| `body_mass` | 12 | 12 | 0 |
| `cycling_distance` | 12 | 12 | 0 |
| `exercise_time` | 12 | 12 | 0 |
| `flights_climbed` | 12 | 12 | 0 |
| `heart_rate` | 12 | 12 | 0 |
| `height` | 12 | 12 | 0 |
| `hrv_sdnn` | 12 | 12 | 0 |
| `oxygen_saturation` | 12 | 12 | 0 |
| `respiratory_rate` | 12 | 12 | 0 |
| `resting_heart_rate` | 12 | 12 | 0 |
| `stand_time` | 12 | 12 | 0 |
| `steps` | 12 | 12 | 0 |
| `vo2_max` | 12 | 12 | 0 |
| `walking_distance` | 12 | 12 | 0 |
| `walking_heart_rate` | 12 | 12 | 0 |
| `walking_speed` | 12 | 12 | 0 |
