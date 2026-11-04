# Trends evidence and data readiness

## First release

The mobile Trends tab compares source-specific daily medians for sleep duration, steps, recorded workout duration and resting heart rate. It uses the same dated aggregates as Browse. The chart compares the last seven completed local calendar days with the preceding 28 completed days. A source and unit need four and 14 observed days respectively before the chart compares the medians. These coverage minima are product rules for descriptive comparisons. The chart omits missing days, deduplicates identical values and excludes conflicting duplicate values for the same source, unit and date.

Sleep duration uses the archive's wake-day summaries. Consumer sleep tracking can support descriptive observations, while clinical use requires validation for the intended purpose. [AASM position statement](https://pmc.ncbi.nlm.nih.gov/articles/PMC5940440/)

Steps and workout duration describe recorded activity. An assessment against physical activity recommendations also needs intensity and frequency. Missing workout days cannot distinguish no workout from no recording. [WHO physical activity guidelines](https://www.who.int/publications/i/item/9789240015128/)

Resting heart rate compares the same source and measurement type. Different devices, acquisition conditions and measurement methods can change what the readings represent. The first release displays the two medians and coverage without assigning a clinical severity.

## Candidate analyses

| Browse data | Analysis supported by existing records | Context needed for stronger interpretation |
| --- | --- | --- |
| Body measurements and mobility | Within-source weight, walking speed or step-length history under comparable measurement conditions | Stable measurement method and enough repeated observations. For intentional weight changes, capture the user's goal separately. |
| Heart and clinical vitals | Separate paired systolic and diastolic history. VO₂ Max history within the same device and estimation method. Keep SDNN, RMSSD and Recovery HRV distinct. | A standardized home blood pressure series needs measurement setting, times and cuff protocol. Fitness estimates can vary by device and method. |
| Blood Work and hospital lab results | Serial same-analyte results when units, collection dates, specimen and assay are compatible. Original lab reference intervals can contextualize individual results. | To assess whether a change exceeds expected variability, use assay imprecision and analyte-specific biological variation when available. |
| Sleep, respiratory, hearing and symptoms | Descriptive duration, recorded event counts or exposure histories with source and coverage | Use a validated protocol and appropriate clinical context before interpreting device estimates as health alerts. |
| Nutrition | Summaries of recorded meals and nutrient amounts on documented days | Repeated dietary recalls or complete food logs. A short questionnaire could capture typical patterns and which days were completely logged. |
| Medications, conditions, immunizations, clinical notes and procedures | Dated event context alongside independently calculated trends | Structured timestamps and reliable context. Temporal proximity alone cannot establish a medication effect or other causal link. |
| Static attributes and sparse categorical records | Latest known attribute or dated event history | Use as reference context where appropriate. A numeric slope requires a quantitative series. |

For home blood pressure, prioritize standardized paired readings and repeated sessions. [AHA measurement statement](https://pmc.ncbi.nlm.nih.gov/articles/PMC11409525/) · [2025 AHA/ACC blood pressure guideline](https://www.ahajournals.org/doi/full/10.1161/CIR.0000000000001356)

For VO₂ Max, report the measurement method and source. The cited validation study found disagreement between wearable estimates and laboratory measurements. [Apple Watch VO₂ Max validation study](https://pmc.ncbi.nlm.nih.gov/articles/PMC12080799/)

For blood work, distinguish a lab reference interval from a reference change value, which depends on analytical and within-person variation. [EFLM Biological Variation Database](https://biologicalvariation.eu/) · [EFLM review](https://www.eflm.eu/upload/publications/2034-2022-ClinChemLabMed-Sandberg-et-al.pdf.pdf)

For nutrition, capture repeat observations because day-to-day intake varies. [NIH/NCI Dietary Assessment Primer](https://epi.grants.cancer.gov/dietary-assessment-primer/approach/principles.html)
