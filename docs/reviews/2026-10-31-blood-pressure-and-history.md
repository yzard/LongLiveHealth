# Blood pressure and historical values

The server catalog previously exposed Blood Pressure plus its systolic and diastolic component entries. The alternate Specialty records picker also opened an older list instead of the combined chart. Both mobile platforms now consume one Blood Pressure catalog entry and use the shared metric chart from either entry point.

The combined query includes exact correlation children, independent pressure samples, and manual pressure entries. Correlated samples are not counted again as independent samples. Missing components stay missing; a recent systolic sample never borrows an older diastolic reading. Account, source, deletion and requested-period boundaries remain enforced.

Historical values share one Swift row across measurement, laboratory, daily, sleep and source-record histories: time, source, value. Timestamps use `yyyy-MM-dd hh:mm a` in the device time zone. Date-only evidence retains its date with a dash for unavailable time. Missing values remain dashes. Blood pressure reads `systolic / diastolic mmHg`. Original laboratory details remain available by expanding the row. The source-record list returns compact values in its existing response rather than requiring a request for every row.

Validation: 226 backend tests passed (two optional benchmarks ignored), 59 portable Swift tests passed, and the targeted iOS Browse/chart/history test passed. Android paired-chart, compact-history geometry and ECG switching checks passed. The paired chart implementation was deployed to NAS and installed on iOS / Android 35. A subsequent installation check found the iOS bundle version was still 86 despite the intended 87/88 labels; project configuration is corrected to 88. The API cleanup Android build is 36.
