# Reference information and characteristic values

Both mobile clients share the presentation in `MetricChart` and `HealthRecordHistoryView`.

- Reference Levels retains range selection and measured-day counts. The adjacent information button opens an overlay with assessment context, the applicable rule, notes and clickable citations. Closing it preserves chart selection and navigation.
- No applicable levels: display “No reference level”, without an information button. Removed the “Days with Measurements” heading and inline guide text.
- All six Apple Health characteristics show their latest value in large text: biological sex, blood type, date of birth, Fitzpatrick skin type, wheelchair use and activity move mode. These are not plotted. History remains available.
- Record-list pagination uses identity order. Load all characteristic metadata pages before choosing the newest timestamp; fetch only that snapshot's details. Dates use calendar components, not Unix measurement times. Missing or invalid values display a dash; unknown enum values are not assigned a known meaning.

Validation: shared tests cover latest selection, all characteristic labels, missing values and invalid dates. Native UI tests cover information disclosure, retained links, original-reference units, and characteristic detail pages.

Device review found that changing the presenting row state did not reliably dismiss the Android sheet. The overlay now uses its own environment dismissal action, matching existing account dialogs. Reference citations open the browser. Birth date, sex and skin type were checked against the latest stored snapshot on Android.
