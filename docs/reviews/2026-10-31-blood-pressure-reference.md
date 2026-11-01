# Blood pressure reference correction

The paired chart already enriches both component series. Existing component rules incorrectly required activity=resting, which the archived HealthKit readings do not supply. Their assessment therefore returned not_assessed. Blood pressure classification does not require sex; the evaluator now only requires the age needed to select the adult rule. The rule note distinguishes classification from diagnosis based on repeated properly measured readings.

Source: 2025 AHA/ACC Guideline, https://professional.heart.org/en/science-news/2025-high-blood-pressure-guideline/top-things-to-know . Normal: systolic <120 and diastolic <80. Elevated: systolic 120–129 with diastolic <80. Stage 1: systolic 130–139 or diastolic 80–89. Stage 2: systolic >=140 or diastolic >=90. Use the higher category when paired readings differ. This change exposes the component bands and citations through the existing shared mobile chart response.

228 backend tests passed (two optional benchmarks ignored), including all component thresholds, missing resting/sex context, chart annotation and citation presence. Full Browse mapping review is recorded separately; mapping coverage does not establish clinical validity or successful UI assessment for every metric.
