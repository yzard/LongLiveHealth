# Trends evidence and data readiness

## Directional display policy — November 5, 2026

Trends displays worsening changes before improving changes. It omits stable results, small fluctuations, insufficient comparisons and changes without a defensible direction. These labels describe movement relative to an applicable reference guide or personal sleep goal, not a diagnosis or a validated clinical-change assessment.

The server retains separate source, unit, assay and specimen comparisons. Daily sleep and resting heart rate compare medians from seven completed local calendar days against the preceding 28, with at least four and 14 observed days. Sleep uses distance below the saved goal; sleeping eight hours with a seven-hour goal is not labelled below goal. No sleep goal means no goal-based classification. Lower heart rate is not inherently better: its direction uses distance outside the applicable reference interval.

For device series with daily points, compare the latest 14 calendar days with earlier history when both contain two points. Monthly histories use the latest 90 days and earlier history. Sparse histories compare their last two collection dates. Retired device sources without data in 90 days and lab series without data in 366 days are not current changes. Those horizons and coverage requirements are product rules.

VO₂ Max direction requires an applicable age/biological-sex fitness guide. Blood pressure evaluates both explicit components; any worsening component places the card in Worsening while retaining each component's direction. Readings below 90 systolic or 60 diastolic are not called improvements using a hypertension-only guide. Blood Work uses identical original reference bounds when all included results have them, otherwise a uniquely applicable published guide. Unknown or changed context cannot be guessed.

To reduce minor fluctuations, display filters require 5% of a two-sided reference interval's width, or 5% of the previous magnitude for a one-sided interval; VO₂ Max requires a 5% change and sleep requires a 30-minute change in goal shortfall. These are explicit UI filters, not medically validated significance thresholds or reference change values. The method sheet states this limitation.

Steps and workout duration are not automatically classified as better/worse: recorded volume alone lacks intensity, recording completeness and personal goals. [WHO physical activity guidelines](https://www.who.int/publications/i/item/9789240015128/)

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

## Long-term cards added November 5, 2026

The authenticated long-term Trends endpoint reads complete stored histories and checks the archive revision before returning results. It returns source-preserving directional cards in one response.

Blood pressure resolves explicit correlations and manual pairs before calculating daily means for both components. Isolated components cannot become pairs through approximate timestamps. VO₂ Max uses the existing unit conversion and preserves device sources. For device histories spanning over 90 days, each observed day contributes equally to its calendar month's average. Two observed dates spanning at least 30 days are a product chart-coverage rule.

Blood Work preserves collected numeric results by calendar date. Identical values at the same timestamp are coalesced. Source, canonical unit, recorded assay method and specimen type define comparison boundaries. A specimen-instance reference identifies a particular collection and does not by itself create a different method. Missing assay/specimen context is disclosed in the information sheet. Two distinct collection dates are required for a serial-result chart.

Insufficient comparisons are omitted from Trends. Original report and Browse history retain text, bounded, unnormalized and undated results.

The read-only NAS audit found five VO₂ Max device histories beginning in 2018 and two blood pressure histories beginning in 2021. Hospital lab records contained 85 numeric source/code/unit combinations with explicit units. Of those, 37 had multiple collection dates and 48 had a single date before additional assay/specimen checks. Confirmed PDF results contained one collection date. A further read-only audit of 199 Clinical Vitals resources found 38 explicitly paired blood pressure observations using UCUM mm[Hg]; the endpoint includes these hospital pairs alongside device correlations and manual pairs. These counts describe the inspected archive on November 5 and can change as data is added.

Home BP interpretation requires repeated readings and appropriate technique. [AHA home monitoring guidance](https://www.heart.org/en/health-topics/high-blood-pressure/understanding-blood-pressure-readings/monitoring-your-blood-pressure-at-home)

Wearable cardio fitness estimates depend on the device method and personal context. [Apple cardio fitness white paper](https://www.apple.com/healthcare/docs/site/Using_Apple_Watch_to_Estimate_Cardio_Fitness_with_VO2_max.pdf)

Laboratory change assessment requires analytical imprecision and within-person biological variation. [EFLM Biological Variation Database](https://biologicalvariation.eu/)
