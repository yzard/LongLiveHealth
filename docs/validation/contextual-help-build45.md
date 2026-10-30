# Contextual explanation audit — Build 45

All top-level iOS Swift UI files were scanned for Text, section footers, disclosure groups, descriptions and server-supplied method notes.

## Behavior

- Explanations appear through a title-adjacent info.circle button, with a 44-point target, VoiceOver label, scrollable popover and Done dismissal. Opening help does not reload the underlying page.
- Browse metric descriptions and calculations below the chart remain visible.
- Errors, progress, missing-data states, actual results, source evidence and destructive-confirmation consequences remain visible.
- Report actions share a row. Source precedes Report Title. Delete removes official and draft observations and the cached report list.

## Changed explanation surfaces

- AccountViews.swift
- AnalysisView.swift
- ArchiveCoachView.swift
- ArchiveReviewView.swift
- BehaviorAssociationsView.swift
- BreathingPracticeView.swift
- CalendarPlanButton.swift
- ComparisonView.swift
- CycleHistoryView.swift
- DailyLogsView.swift
- DietQualityView.swift
- Editors.swift
- FoodLibraryView.swift
- FoodLookupView.swift
- HRVWindowsView.swift
- HealthReportView.swift
- HealthView.swift
- LongLiveHealthApp.swift
- MealGlucoseView.swift
- MealPlannerView.swift
- MetricHistoryView.swift
- NumberMapEditor.swift
- NutritionDayView.swift
- OverviewView.swift
- RemindersView.swift
- ReportsView.swift
- SettingsView.swift
- SharedReportImportView.swift
- SleepSessionsView.swift
- SourceAliasesView.swift
- SpecialtyRecordsView.swift
- TrainingDetailView.swift
- TrainingImportsView.swift
- TrainingPlannerView.swift
- TrendsView.swift
- VoiceQuestionButton.swift

## Reviewed exceptions

- MetricMeasurementsView, MetricHistoryView and DailyMetricView: About this metric prose and units remain visible.
- SleepSessionsView: source/sleep evidence and missingness remain data; regularity and session-method help are contextual.
- ReportsView: result interpretation and reference ranges remain result data.
- BehaviorAssociationsView: calibration failure remains an explicit unavailable-status warning.
- Account/report deletion dialogs retain consequences before confirmation.

## Files scanned

AccountViews.swift, AnalysisView.swift, AppLockShield.swift, AppModel.swift, ArchiveCoachView.swift, ArchiveReviewView.swift, BehaviorAssociationsView.swift, BreathingPracticeView.swift, BrowseChart.swift, BrowseView.swift, CalendarPlanButton.swift, ComparisonView.swift, CycleHistoryView.swift, DailyLogsView.swift, DietQualityView.swift, DocumentViews.swift, Editors.swift, FoodLibraryView.swift, FoodLookupView.swift, HRVWindowsView.swift, HealthBackground.swift, HealthBridge.swift, HealthCatalog.swift, HealthReportView.swift, HealthSeriesReader.swift, HealthSnapshots.swift, HealthView.swift, InfoHelp.swift, LongLiveHealthApp.swift, MealGlucoseView.swift, MealPlannerView.swift, MetricChart.swift, MetricHistoryView.swift, MetricMeasurementsView.swift, NumberMapEditor.swift, NutritionDayView.swift, OverviewView.swift, RemindersView.swift, ReportsView.swift, RetainedView.swift, SessionController.swift, SessionVault.swift, SettingsView.swift, SharedReportImportView.swift, SleepSessionsView.swift, SourceAliasesView.swift, SourceRecordCountsView.swift, SpecialtyRecordsView.swift, TrainingDetailView.swift, TrainingImportsView.swift, TrainingPlannerView.swift, TrendsView.swift, VoiceQuestionButton.swift

## Validation

- Simulator compilation passed; 23 native tests passed.
- Contextual help opens/dismisses and Browse sleep explanation remains visible: passed.
- Report delete cancellation sends no mutation; confirmation deletes once and removes the report from the list: passed.
- Backend regression proves deletion removes both physical official and draft rows; wrong-owner and stale-revision requests leave both intact: passed.
- The 126-marker traversal exercised source editing, help, action alignment, confirm-all retry, bottom/top scrolling and editor dismissal. Its original final deletion assertion expected a Cancel button in a compact confirmation popover. That flow is now an explicit alert and passed the focused deletion regression.
- Device Release build succeeded. After clearing a stale interrupted-signing resource entry by re-signing, strict signature verification passed. Build 45 installed on the connected iPhone (installation database sequence 2800).

Evidence: build/ios/validation/help45-tests.log, help45-focused-final.log, help45-delete-test.log, help45-final-attachments.
