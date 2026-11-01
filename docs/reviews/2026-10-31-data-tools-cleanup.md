# Data tools cleanup

Shared iOS/Android changes:

- Export creation, download/share and deletion are owned by one component at the bottom of Data. Settings no longer owns exports or requests their list. Removed Export contents and the archive-navigation description.
- Health sync shows failed coverage categories in Sync Error before a single All Sync Coverage Categories link. The full list puts failures first, and retains status, last success and archived date ranges.
- Removed Daily logs, Source aliases and Favorites help rows, the Sources subtitle in alias settings, and the Favorite metrics subtitle.
- Removed the Overview Practice section and breathing practice timer. Removed its web route, manual-entry editor, backend payload variant, validation and review/timeline aggregation. Existing archived records are retained for export but retired manual kinds are excluded from active editors. Respiratory health data sync and Browse remain unchanged.

Regression coverage includes export actions from Data, absence in Settings, error-first coverage, removal of redundant headings, rejection of retired practice input, and availability of respiratory metrics.

Reference Settings replaces Personal settings. Sleep Goal, Date of Birth and Biological Sex are peer rows; the Reference Profile container is removed. Each editor retains immediate saving and rollback on failure. Existing profile storage keys and saved values are unchanged; the visible Reference sex label and picker options now read Biological Sex, Female and Male.

Validated with 55 shared-core tests, 219 backend tests (2 ignored), Android build/unit tests/lint and the native Data-tools/reference-settings UI regression. The latter covers opening each peer setting, saving Biological Sex and observing the new value in Settings, creating/deleting an export, error-first coverage and absence of removed help rows. Final mobile releases: iOS 82 and Android 29; installed with account data preserved.
