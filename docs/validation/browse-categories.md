# Shared Browse categories

Both mobile clients use `BrowseCategory` in shared Core and the same category grid and metric directory in `BrowseView`.

Browse opens with two-column category cards. A card opens its metric list; global search searches metric names, identifiers and category names across the directory. Existing detail routes and source aggregation remain unchanged. Category navigation does not fetch the directory again. Android retains its adaptive master/detail container.

Each entry resolves to one category, independent of its source. Dietary nutrients belong to Nutrition; measured blood analytes belong to Blood Work, except cardiovascular markers such as blood lipids and apolipoproteins, which belong to Heart. Heart also includes heart rate, SDNN/RMSSD (including Recovery HRV), blood pressure, ECG and VO₂ Max. Sleep duration and session details remain under Sleep. Clinical record types have dedicated categories. Unknown laboratory metrics remain visible in Blood Work; unknown non-laboratory types remain visible in Other Data. Empty categories are omitted.

Regression coverage includes the entire checked-in Apple catalog, an explicit allowlist for miscellaneous records, lab/dietary distinctions, cardiovascular aliases, future unknown types, and simulator navigation through category → metric → Back and tab restoration.

## Validation and artifacts (2026-10-29)

- 49 shared Core tests passed, including the entire Apple catalog classification audit.
- iOS category navigation, catalog search/detail, shared detail presentation, and directory request-count checks passed on the simulator. Returning through Back and switching tabs issued zero directory reload requests. Build 67 also compiled for iPhone without signing.
- ARM64-only Android Build 4 passed Docker compilation and lint. All 854 app-native methods in the final APK have native implementations. Android uses bundled category vector glyphs because its SF Symbols bridge does not include most health symbols.
- Final APK SHA-256: `be9d900c54df0c8b5f4d88bce0dc382ce5a7a3c408ddda044fa2906bdcef10fe`.
- API image: `longlivehealth-backend-api:browse-categories-icons-20261029`.
- The real Android phone rendered the final bundled category glyphs without missing-icon placeholders, opened Heart, and loaded the Heart Rate chart from the server. It locked again before Blood Work and return/tab-state checks could finish; those real-device checks remain pending. The initial category release was not installed on iPhone; Build 68 installation is recorded below.

## Compact cards

The shared card minimum height is now 72 pt plus 12 pt padding on each side (96 pt total, previously 132 pt). Grid gaps are 10 pt and the icon/title gap is 8 pt. Text can expand the card for accessibility sizes; labels are not truncated. This follows the user’s request for smaller tiles. Android versionCode 5 / iOS build 68.

Compact-card validation passed: iOS simulator category navigation and request-count checks, iOS unsigned device Build 68, Android Docker build/lint and 854 native-interface checks. Android Build 5 was installed over Build 4 without clearing data. NAS now serves `longlivehealth-backend-api:browse-compact-20261029`; APK SHA-256 is `8721879de0999e6d3c11d96f3c70f281904af8ff2858d1895c14168395bec486`. The new card spacing was visually checked in the iOS simulator. Final Android on-screen verification remains pending phone unlock.

On 2026-10-29, iOS Build 68 was rebuilt with Apple Development signing, passed strict code-signature verification, and installed over the existing `dev.longlivehealth.ios` app on the paired iPhone 16 Pro Max (installation sequence 2748). Evidence: `build/ios/validation/build68-device.log` and `build68-install.json`. No uninstall or app-data reset was performed.
