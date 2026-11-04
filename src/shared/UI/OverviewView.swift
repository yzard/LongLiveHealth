import SwiftUI
#if canImport(Charts)
import Charts
#endif

struct OverviewView: View {
    var model: AppModel
    @Environment(\.scenePhase) var scenePhase
    @State var date = Date()
    @State var snapshot: JSONValue = .null
    @State var preferences: JSONValue = .null
    @State var loading = false
    @State var failure: String?
    @State var customizing = false

    private var dateKey: String {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = .current
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.string(from: date)
    }
    private var requestKey: String { dateKey + TimeZone.current.identifier }
    private var sourcePriorities: JSONValue {
        preferences["preferences"]["source_priority"] == .null ? .object([:]) : preferences["preferences"]["source_priority"]
    }
    private var visibleMetrics: [JSONValue] {
        return snapshot["metrics"].arrayValue.filter { OverviewVisibility.isVisible($0["record_type"].stringValue, preferences: preferences["preferences"]) }
    }

    var body: some View {
        List {
            if loading { ProgressView("Loading summaries") }
            if let failure { Text(failure).foregroundStyle(.secondary) }
            Section {
                #if os(Android)
                VStack(spacing: 14) {
                    HStack(alignment: .top, spacing: 8) {
                        Text("Metric").frame(maxWidth: .infinity, alignment: .leading)
                        Text("Today").frame(maxWidth: .infinity, alignment: .leading)
                        ForEach([7, 30], id: \.self) { days in
                            VStack(alignment: .leading) {
                                Text("Last \(days) days")
                                Text("Daily average").font(.caption2).foregroundStyle(.secondary)
                            }.frame(maxWidth: .infinity, alignment: .leading)
                        }
                    }.font(.caption.bold())
                    ForEach(Array(visibleMetrics.enumerated()), id: \.offset) { _, metric in
                        Divider()
                        HStack(alignment: .top, spacing: 8) {
                            Text(metricTitle(metric["record_type"].stringValue)).font(.subheadline).frame(maxWidth: .infinity, alignment: .leading)
                            ForEach([1, 7, 30], id: \.self) { days in comparisonCell(metric, days: days).frame(maxWidth: .infinity, alignment: .leading) }
                        }
                    }
                }
                #else
                Grid(alignment: .leading, horizontalSpacing: 8, verticalSpacing: 14) {
                    GridRow {
                        Text("Metric")
                        Text("Today")
                        ForEach([7, 30], id: \.self) { days in
                            VStack(alignment: .leading) {
                                Text("Last \(days) days")
                                Text("Daily average").font(.caption2).foregroundStyle(.secondary)
                            }
                        }
                    }.font(.caption.bold())
                    ForEach(Array(visibleMetrics.enumerated()), id: \.offset) { _, metric in
                        Divider().gridCellUnsizedAxes(.horizontal)
                        GridRow(alignment: .top) {
                            Text(metricTitle(metric["record_type"].stringValue)).font(.subheadline)
                            ForEach([1, 7, 30], id: \.self) { days in
                                comparisonCell(metric, days: days)
                            }
                        }
                    }
                }
                #endif
                if !loading && visibleMetrics.isEmpty { Text(snapshot["metrics"].arrayValue.isEmpty ? "No synced measurements yet." : "No metrics selected. Turn on metrics in Customize.").foregroundStyle(.secondary) }
            }
        }
        .mobileNavigationTitle("Overview")
        .toolbar {
            ToolbarItem(placement: .topBarLeading) { Button("Customize") { customizing = true }.disabled(snapshot["metrics"].arrayValue.isEmpty) }
            ToolbarItem(placement: .topBarTrailing) { NavigationLink(destination: SettingsView(model: model)) { Label("Settings", systemImage: MobileSymbol.name("gearshape")) } }
        }
        .onChange(of: model.sourcePreferences) { _, updated in preferences = updated }
        .sheet(isPresented: $customizing) { NavigationStack { OverviewPreferencesView(model: model, preferences: $preferences, metrics: snapshot["metrics"].arrayValue) } }
        .initialTask(id: requestKey) { await reload() }
        .onChange(of: scenePhase) { _, phase in
            if phase == .active { date = Date() }
        }
        .refreshable {
            let previous = requestKey
            date = Date()
            if previous == requestKey { await reload() }
        }
    }

    @ViewBuilder private func comparisonCell(_ metric: JSONValue, days: Int) -> some View {
        let names = Dictionary(model.sourceCatalog.map { ($0["source_key"].stringValue, $0["name"].stringValue) }, uniquingKeysWith: { first, _ in first })
        let kind = metric["record_type"].stringValue
        let summary = OverviewSummary.comparison(metric: metric, endDate: dateKey, days: days, names: names, sleepTargetMinutes: preferences["preferences"]["sleep_target_minutes"].numberValue)
        VStack(alignment: .leading, spacing: 4) {
            if let summary {
                Text(OverviewSummary.value(summary.average, kind: kind, unit: summary.unit)).stableDigits()
                if let met = summary.goalMet {
                    Text(met ? "Goal met" : "Below goal").font(.caption2).foregroundStyle(met ? Color.green : Color.orange)
                }
            } else { Text("—").foregroundStyle(.secondary) }
        }.font(.subheadline)
    }

    private func reload() async {
        guard model.client != nil, let token = model.session?.token else { return }
        let key = requestKey
        loading = true
        snapshot = .null
        failure = nil
        defer { if key == requestKey { loading = false } }
        do {
            preferences = try model.cachedResponse("wellness/preferences/get", body: .object([:])) ?? .null
            snapshot = try model.cachedResponse("wellness/day", body: .object(["date": .string(dateKey), "timezone": .string(TimeZone.current.identifier), "source_priority": sourcePriorities])) ?? .null
            if model.sourceCatalog.isEmpty { _ = try await model.request("wellness/sources", body: .object([:])) }
            let saved = try await model.request("wellness/preferences/get", body: .object([:]))
            guard !Task.isCancelled, model.session?.token == token, key == requestKey else { return }
            preferences = saved
            let result = try await model.request("wellness/day", body: .object([
                "date": .string(dateKey), "timezone": .string(TimeZone.current.identifier), "source_priority": sourcePriorities
            ]))
            guard !Task.isCancelled, model.session?.token == token, key == requestKey else { return }
            snapshot = result
        } catch {
            guard !Task.isCancelled, model.session?.token == token, key == requestKey else { return }
            if case APIError.status(401, _) = error { model.errorMessage = error.localizedDescription; return }
            failure = error.localizedDescription

        }
    }
}

struct DailyMetricView: View {
    var model: AppModel
    private var sourceKeys: [String] { Array(Set(displayed["history"].arrayValue.flatMap { $0["alternatives"].arrayValue.map { $0["source"].stringValue } })).sorted() }
    let metric: JSONValue
    @State var completeHistory: JSONValue = .null
    @State var selectedWindow: ChartWindow?
    @State var loadedWindow: ChartWindow?
    @State var failure: String?
    @State var loading = true
    private var displayed: JSONValue { completeHistory == .null ? metric : completeHistory }
    private var chartUnit: String { displayed["unit"].stringValue.isEmpty ? metric["unit"].stringValue : displayed["unit"].stringValue }
    var body: some View {
        MetricDetailScreen(title: metricTitle(metric["record_type"].stringValue)) {
            MetricChartSection {
                if loading { ProgressView("Loading selected period") }
                if let failure { Text(failure).foregroundStyle(.secondary) }
                if metric["current"]["state"] == .string("query_limit_exceeded") {
                    Text("This history exceeds the daily query limit. Its records are saved, but the chart could not be loaded.").foregroundStyle(.secondary)
                }
                MetricChart(model: model, guidance: displayed["reference_guidance"], ranges: displayed["chart_ranges"], unit: chartUnit, metricKind: metric["record_type"].stringValue,
                            divisor: ["sleep", "workout"].contains(metric["record_type"].stringValue) ? 3600 : 1,
                            sourceKeys: sourceKeys, names: [:], initialPeriod: .week, showsHour: false, heartRate: metric["record_type"] == .string("resting_heart_rate"), navigationTimestamps: [completeHistory["first_record_at"].numberValue, completeHistory["last_record_at"].numberValue].compactMap { $0 }, onWindowChange: { selectedWindow = $0 })
                    .environment(\.timeZone, .gmt)

            }
            Section("About this metric") {
                MetricAboutView(id: metric["record_type"].stringValue, name: metricTitle(metric["record_type"].stringValue))
            }
            Section {
                if metric["record_type"] == .string("sleep") {
                    NavigationLink("Sleep Sessions") { SleepSessionsView(model: model) }
                }
                NavigationLink("Historical Values") {
                    List {
                        Section("Daily records") {
                            ForEach(HistoricalValue.dailyRows(displayed["history"].arrayValue)) { row in
                                HistoricalValueRow(timestamp: nil, dateText: row.value["date"].stringValue,
                                    source: model.sourceLabel(row.value["source"].stringValue), value: metricValue(row.value, kind: metric["record_type"].stringValue))
                            }
                        }
                    }.mobileNavigationTitle("Historical Values").navigationBarTitleDisplayMode(.inline)
                }
            }

        }
            .task(id: selectedWindow) { await loadHistory(force: false) }
            .onChange(of: model.sourcePreferences["preferences"]["reference_profile"]) { _, _ in Task { await loadHistory(force: true) } }
            .refreshable { await loadHistory(force: true) }
    }
    private func loadHistory(force: Bool) async {
        guard force || loadedWindow != selectedWindow || failure != nil else { return }
        guard let selectedWindow, let token = model.session?.token else { return }
        loading = true; failure = nil
        defer { loading = false }
        let formatter = DateFormatter(); formatter.calendar = Calendar(identifier: .gregorian); formatter.timeZone = .gmt; formatter.dateFormat = "yyyy-MM-dd"
        let dates = [selectedWindow.start, selectedWindow.start + selectedWindow.length - 1].map { JSONValue.string(formatter.string(from: Date(timeIntervalSince1970: $0))) }
        do {
            let response = try await model.request("wellness/history", body: .object(["record_type": metric["record_type"], "timezone": .string(TimeZone.current.identifier), "dates": .array(dates)]))
            guard !Task.isCancelled, self.selectedWindow == selectedWindow, model.session?.token == token else { return }
            completeHistory = response
            loadedWindow = selectedWindow
        } catch { if !Task.isCancelled { failure = "Selected period could not be loaded: " + error.localizedDescription } }

    }
}

private func metricValue(_ selected: JSONValue, kind: String) -> String {
    guard let value = selected["value"].numberValue else { return "Not available" }
    return OverviewSummary.value(value, kind: kind, unit: selected["unit"].stringValue)
}
func metricTitle(_ kind: String) -> String {
    ["sleep": "Sleep", "workout": "Workouts", "active_energy": "Active Energy", "basal_energy": "Resting Energy", "steps": "Steps", "resting_heart_rate": "Resting heart rate", "heart_rate": "Heart rate", "hrv_sdnn": "HRV SDNN"][kind] ?? kind
}

enum DataPage: Hashable {
    case reports, health, training, entries, settings, aliases, counts
}

struct DataCenterView: View {
    var model: AppModel
    #if os(Android)
    @State var selected: DataPage?
    var body: some View {
        AdaptiveDetailLayout(selected: selected != nil) {
            NavigationStack { menu }
        } detail: {
            if let selected {
                NavigationStack {
                    destination(selected).toolbar {
                        ToolbarItem(placement: .topBarLeading) {
                            Button { self.selected = nil } label: { Image(systemName: "chevron.left") }
                                .accessibilityLabel(Text("Back"))
                        }
                    }
                }.id(selected)
            }
            else { EmptyContent(title: "Choose a data tool", symbol: "externaldrive") }
        }.onChange(of: model.dataPath) { _, path in
            if let last = path.last { selected = last == .reports ? .reports : .health }
        }
    }
    #else
    var body: some View { menu }
    #endif
    private var menu: some View {
        List {
            Section("Import") {
                link("Health Report", page: .reports)
                if !model.sharedReports.isEmpty { Button("Import shared PDFs (\(model.sharedReports.count))") { model.showSharedReportImport = true } }
                link(MobilePlatform.healthProviderName, page: .health)
                link("Import GPX, TCX, or FIT activities", page: .training)
                link("Manual Imports", page: .entries)
            }
            Section("Sources") {
                link("Source Aliases", page: .aliases)
                link("Sources Record Count", page: .counts)
            }
            ExportSection(model: model)
        }.mobileNavigationTitle("Data")
            .navigationDestination(for: DataDestination.self) { value in destination(value == .reports ? .reports : .health) }
            .toolbar {
                #if os(Android)
                Button { selected = .settings } label: { Label("Settings", systemImage: MobileSymbol.name("gearshape")) }
                #else
                NavigationLink(destination: SettingsView(model: model)) { Label("Settings", systemImage: MobileSymbol.name("gearshape")) }
                #endif
            }
    }
    @ViewBuilder private func link(_ title: String, page: DataPage) -> some View {
        #if os(Android)
        Button { selected = page } label: { row(title, page: page) }
        #else
        if page == .reports || page == .health {
            NavigationLink(value: page == .reports ? DataDestination.reports : .health) { row(title, page: page) }
        } else { NavigationLink(destination: destination(page)) { row(title, page: page) } }
        #endif
    }
    private func row(_ title: String, page: DataPage) -> some View {
        HStack {
            Text(title)
            Spacer()
            if page == .health && model.isHealthSyncRunning { ProgressView().accessibilityHidden(true) }
        }.accessibilityValue(page == .health && model.isHealthSyncRunning ? "Syncing" : "")
    }
    @ViewBuilder private func destination(_ page: DataPage) -> some View {
        switch page {
        case .reports: ReportsView(model: model)
        case .health: HealthView(model: model)
        case .training: TrainingImportsView(model: model)
        case .entries: DailyLogsView(model: model)
        case .settings: SettingsView(model: model)
        case .aliases: SourceAliasesView(model: model)
        case .counts: SourceRecordCountsView(model: model)
        }
    }
}

struct OverviewPreferencesView: View {
    var model: AppModel
    @Binding var preferences: JSONValue
    let metrics: [JSONValue]
    private var kinds: [String] { metrics.map { $0["record_type"].stringValue } }
    var body: some View {
        AutoSavedPreferencesForm(model: model, title: "Customize overview") { current, update in
            Section {
                ForEach(kinds, id: \.self) { kind in
                    Toggle(metricTitle(kind), isOn: Binding(get: {
                        OverviewVisibility.isVisible(kind, preferences: current)
                    }, set: { selected in
                        update(OverviewVisibility.change(kind, visible: selected, available: kinds, preferences: current))
                    }))
                }
            }
        }.onChange(of: model.sourcePreferences) { _, updated in preferences = updated }
    }
}
