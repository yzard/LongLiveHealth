# Mobile API inventory

Both clients compile the shared application and UI sources below. Platform health bridges use the same connect/sync/coverage endpoints. Downloads are GET; business operations are POST. Internal calculation functions are not additional API routes.

| Route | Caller |
| --- | --- |
| `/api/v1/analysis/create` | src/shared/UI/AnalysisView.swift |
| `/api/v1/analysis/feedback` | src/shared/UI/AnalysisView.swift |
| `/api/v1/analysis/get` | src/shared/UI/AnalysisView.swift, src/shared/UI/ArchiveCoachView.swift |
| `/api/v1/analysis/list` | src/shared/UI/AnalysisView.swift |
| `/api/v1/analysis/retry` | src/shared/UI/AnalysisView.swift |
| `/api/v1/browse/catalog` | src/shared/UI/BrowseView.swift, src/shared/UI/CatalogMetricView.swift |
| `/api/v1/exports/create` | src/shared/UI/ExportSection.swift |
| `/api/v1/exports/delete` | src/shared/UI/ExportSection.swift |
| `/api/v1/exports/list` | src/shared/UI/ExportSection.swift |
| `/api/v1/exports/{export_id}/download` | src/shared/UI/ExportSection.swift |
| `/api/v1/files/upload` | src/shared/Core/APIClient.swift |
| `/api/v1/files/{file_id}/download` | src/shared/Application/AppModel.swift |
| `/api/v1/health/connect` | src/android/Swift/HealthBridge.swift, src/ios/HealthBridge.swift |
| `/api/v1/health/coverage` | src/android/Swift/HealthBridge.swift, src/ios/HealthBridge.swift, src/shared/UI/HealthView.swift |
| `/api/v1/health/sync` | src/android/Swift/HealthBridge.swift, src/ios/HealthBridge.swift |
| `/api/v1/jobs/list` | src/shared/Application/AppModel.swift, src/shared/UI/ReportView.swift |
| `/api/v1/jobs/retry` | src/shared/UI/ReportView.swift |
| `/api/v1/metrics/history/get` | src/shared/UI/MetricHistoryView.swift |
| `/api/v1/metrics/list` | src/shared/Application/AppModel.swift |
| `/api/v1/reports/delete` | src/shared/UI/ReportView.swift |
| `/api/v1/reports/extraction/get` | src/shared/UI/ReportView.swift |
| `/api/v1/reports/get` | src/shared/Application/AppModel.swift |
| `/api/v1/reports/input/get` | src/shared/UI/ReportView.swift |
| `/api/v1/reports/list` | src/shared/Application/AppModel.swift |
| `/api/v1/reports/relate` | src/shared/UI/ReportView.swift |
| `/api/v1/reports/rescan` | src/shared/UI/ReportView.swift |
| `/api/v1/reports/review` | src/shared/Application/AppModel.swift, src/shared/UI/ReportView.swift |
| `/api/v1/server/status` | src/shared/Application/AppModel.swift |
| `/api/v1/session/login` | src/shared/Application/AppModel.swift |
| `/api/v1/session/logout` | src/shared/Application/AppModel.swift |
| `/api/v1/session/refresh` | src/shared/Application/SessionController.swift |
| `/api/v1/session/revoke` | src/shared/Application/AppModel.swift |
| `/api/v1/trends/get` | src/shared/UI/TrendsView.swift |
| `/api/v1/user/delete` | src/shared/UI/SettingsView.swift |
| `/api/v1/user/get` | src/shared/Application/AppModel.swift |
| `/api/v1/user/password/change` | src/shared/Application/AppModel.swift |
| `/api/v1/users/create` | src/shared/UI/AccountViews.swift |
| `/api/v1/users/delete` | src/shared/UI/AccountViews.swift |
| `/api/v1/users/list` | src/shared/UI/AccountViews.swift |
| `/api/v1/wellness/associations/get` | src/shared/UI/BehaviorAssociationsView.swift |
| `/api/v1/wellness/associations/list` | src/shared/UI/BehaviorAssociationsView.swift |
| `/api/v1/wellness/associations/run` | src/shared/UI/BehaviorAssociationsView.swift |
| `/api/v1/wellness/chart` | src/shared/UI/MetricMeasurementsView.swift |
| `/api/v1/wellness/coach` | src/shared/UI/ArchiveCoachView.swift |
| `/api/v1/wellness/day` | src/shared/UI/OverviewView.swift |
| `/api/v1/wellness/diet` | src/shared/UI/DietQualityView.swift |
| `/api/v1/wellness/entries/delete` | src/shared/UI/ArchiveCoachView.swift, src/shared/UI/DailyLogsView.swift, src/shared/UI/FoodLibraryView.swift, src/shared/UI/MealPlannerView.swift, src/shared/UI/RemindersView.swift, src/shared/UI/TrainingPlannerView.swift |
| `/api/v1/wellness/entries/list` | src/shared/UI/CycleHistoryView.swift, src/shared/UI/DailyLogsView.swift, src/shared/UI/TrainingPlannerView.swift |
| `/api/v1/wellness/entries/save` | src/shared/UI/DailyLogsView.swift, src/shared/UI/SleepSessionsView.swift |
| `/api/v1/wellness/food/lookup` | src/shared/UI/FoodLookupView.swift |
| `/api/v1/wellness/food/portion` | src/shared/UI/FoodLibraryView.swift |
| `/api/v1/wellness/history` | src/shared/UI/OverviewView.swift |
| `/api/v1/wellness/hrv` | src/shared/UI/HRVWindowsView.swift |
| `/api/v1/wellness/import/delete` | src/shared/UI/TrainingImportsView.swift |
| `/api/v1/wellness/import/fit` | src/shared/UI/TrainingImportsView.swift |
| `/api/v1/wellness/import/gpx` | src/shared/UI/TrainingImportsView.swift |
| `/api/v1/wellness/import/list` | src/shared/UI/TrainingImportsView.swift |
| `/api/v1/wellness/import/tcx` | src/shared/UI/TrainingImportsView.swift |
| `/api/v1/wellness/library` | src/shared/UI/ArchiveCoachView.swift, src/shared/UI/FoodLibraryView.swift, src/shared/UI/MealPlannerView.swift, src/shared/UI/RemindersView.swift, src/shared/UI/TrainingPlannerView.swift |
| `/api/v1/wellness/meal-glucose` | src/shared/UI/MealGlucoseView.swift |
| `/api/v1/wellness/meals` | src/shared/UI/MealPlannerView.swift |
| `/api/v1/wellness/measurements` | src/shared/UI/MetricMeasurementsView.swift |
| `/api/v1/wellness/nutrition/day` | src/shared/UI/NutritionDayView.swift |
| `/api/v1/wellness/preferences/get` | src/shared/Application/AppModel.swift, src/shared/UI/AutoSavedPreferencesForm.swift, src/shared/UI/OverviewView.swift, src/shared/UI/SourceAliasesView.swift |
| `/api/v1/wellness/preferences/save` | src/shared/UI/AutoSavedPreferencesForm.swift, src/shared/UI/SourceAliasesView.swift |
| `/api/v1/wellness/records/get` | src/shared/UI/HealthRecordHistoryView.swift, src/shared/UI/SpecialtyRecordsView.swift, src/shared/UI/TrainingDetailView.swift |
| `/api/v1/wellness/records/list` | src/shared/UI/ECGRecordsView.swift, src/shared/UI/HealthRecordHistoryView.swift, src/shared/UI/SpecialtyRecordsView.swift |
| `/api/v1/wellness/reminders` | src/shared/UI/RemindersView.swift |
| `/api/v1/wellness/report` | src/shared/UI/ComparisonView.swift, src/shared/UI/HealthReportView.swift |
| `/api/v1/wellness/review` | src/shared/UI/ArchiveReviewView.swift |
| `/api/v1/wellness/sleep` | src/shared/UI/SleepSessionsView.swift |
| `/api/v1/wellness/sleep/regularity` | src/shared/UI/SleepSessionsView.swift |
| `/api/v1/wellness/sources` | src/shared/UI/BehaviorAssociationsView.swift, src/shared/UI/OverviewView.swift, src/shared/UI/SourceAliasesView.swift, src/shared/UI/SourceRecordCountsView.swift |
| `/api/v1/wellness/timeline` | src/shared/UI/ArchiveReviewView.swift |
| `/longlivehealth.apk` | Initial Android installation download |
