import SwiftUI
import Charts

struct OverviewView: View {
    var model: AppModel
    @Environment(\.scenePhase) private var scenePhase
    @State private var date = Calendar.current.date(byAdding: .day, value: -1, to: Date()) ?? Date()
    @State private var snapshot: JSONValue = .null
    @State private var preferences: JSONValue = .null
    @State private var cached = false
    @State private var loading = false
    @State private var failure: String?
    @State private var customizing = false

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
        let favorites = preferences["preferences"]["favorite_metrics"].arrayValue.map(\.stringValue)
        return snapshot["metrics"].arrayValue.filter { favorites.isEmpty || favorites.contains($0["record_type"].stringValue) }
    }

    var body: some View {
        List {
            if loading { ProgressView("Loading summaries") }
            if let failure { Text(failure).foregroundStyle(.secondary) }
            if cached { Label("Cached data · refresh when connected", systemImage: "wifi.slash").font(.caption) }
            Section("Yesterday") { summary(days: 1) }
            Section("Last 7 days") {
                Text("Seven complete days ending yesterday. Averages use recorded days only.").font(.caption).foregroundStyle(.secondary)
                summary(days: 7)
            }
            Section {
                Text("Open Browse to explore individual metrics and all their sources.").font(.caption).foregroundStyle(.secondary)
            }
        }
        .navigationTitle("Overview")
        .toolbar {
            ToolbarItem(placement: .topBarLeading) { Button("Customize") { customizing = true } }
            ToolbarItem(placement: .topBarTrailing) { NavigationLink(destination: SettingsView(model: model)) { Label("Settings", systemImage: "gearshape") } }
        }
        .sheet(isPresented: $customizing) { NavigationStack { OverviewPreferencesView(model: model, preferences: $preferences) } }
        .initialTask(id: requestKey) { await reload() }
        .onChange(of: scenePhase) { _, phase in
            if phase == .active { date = Calendar.current.date(byAdding: .day, value: -1, to: Date()) ?? Date() }
        }
        .refreshable {
            let previous = requestKey
            date = Calendar.current.date(byAdding: .day, value: -1, to: Date()) ?? Date()
            if previous == requestKey { await reload() }
        }
    }

    @ViewBuilder private func summary(days: Int) -> some View {
        if !loading && visibleMetrics.isEmpty { Text("No synced measurements yet.").foregroundStyle(.secondary) }
        ForEach(Array(visibleMetrics.enumerated()), id: \.offset) { _, metric in
            let kind = metric["record_type"].stringValue
            let sources = OverviewSummary.sources(metric: metric, endDate: dateKey, days: days, sleepTargetMinutes: preferences["preferences"]["sleep_target_minutes"].numberValue)
            VStack(alignment: .leading, spacing: 6) {
                Text(metricTitle(kind)).font(.headline)
                if sources.isEmpty {
                    Text(metric["current"]["state"] == .string("query_limit_exceeded") ? "Too much data to summarize. Open Browse for details." : "No recorded data").foregroundStyle(.secondary)
                }
                ForEach(sources) { source in
                    VStack(alignment: .leading, spacing: 4) {
                        Text(OverviewSummary.value(source.average, kind: kind, unit: source.unit)).font(.title3).monospacedDigit()
                        if days > 1 { Text("Daily average · \(source.days)/7 days recorded").font(.caption) }
                        if let met = source.targetDays, let target = preferences["preferences"]["sleep_target_minutes"].numberValue {
                            Label(days == 1 ? (met == 1 ? "Sleep goal met" : "Below your sleep goal") : "Sleep goal met on \(met)/\(source.days) recorded days", systemImage: met == source.days ? "checkmark.circle" : "info.circle")
                                .font(.caption).foregroundStyle(met == source.days ? Color.green : Color.orange)
                            Text("Your goal: " + OverviewSummary.duration(target * 60)).font(.caption).foregroundStyle(.secondary)
                        } else {
                            Text(kind == "sleep" ? "No sleep goal set" : "No reference range set").font(.caption).foregroundStyle(.secondary)
                        }
                        Text(model.sourceLabel(source.source)).font(.caption).foregroundStyle(.secondary)
                    }
                }
            }.padding(.vertical, 4)
        }
    }

    private func reload() async {
        guard let client = model.client, let archive = model.archive, let token = model.session?.token else { return }
        let key = requestKey
        loading = true
        snapshot = .null
        failure = nil
        cached = false
        defer { if key == requestKey { loading = false } }
        do {
            preferences = try archive.load(JSONValue.self, name: "wellness-preferences.json") ?? .null
            let saved = try await model.request("wellness/preferences/get", body: .object([:]))
            guard !Task.isCancelled, model.session?.token == token, key == requestKey else { return }
            preferences = saved
            try archive.save(saved, name: "wellness-preferences.json")
            let result = try await model.request("wellness/day", body: .object([
                "date": .string(dateKey), "timezone": .string(TimeZone.current.identifier), "source_priority": sourcePriorities
            ]))
            guard !Task.isCancelled, model.session?.token == token, key == requestKey else { return }
            snapshot = result
            try archive.save(result, name: "daily-overview.json")
        } catch {
            guard !Task.isCancelled, model.session?.token == token, key == requestKey else { return }
            if case APIError.status(401, _) = error { model.errorMessage = error.localizedDescription; return }
            failure = error.localizedDescription
            if let result = try? archive.load(JSONValue.self, name: "daily-overview.json"),
               result["date"].stringValue == dateKey, result["timezone"].stringValue == TimeZone.current.identifier,
               result["source_priority"] == sourcePriorities {
                snapshot = result
                cached = true
            }
        }
    }
}

struct DailyMetricView: View {
    var model: AppModel
    private var sourceKeys: [String] { Array(Set(metric["history"].arrayValue.flatMap { $0["alternatives"].arrayValue.map { $0["source"].stringValue } })).sorted() }
    let metric: JSONValue
    private var calculationDescription: String {
        switch metric["record_type"].stringValue {
        case "sleep": "Time recorded asleep during each calendar day. Overlapping sleep intervals from the same source are merged, then their durations are added. The chart uses hours; history uses hours and minutes."
        case "workout": "Recorded workout duration per calendar day. Overlapping intervals from the same source are merged before their durations are added."
        case "steps": "Recorded steps per day, summed within each source. Records crossing midnight are allocated in proportion to their duration within each day."
        case "heart_rate": "Heartbeats per minute. Each daily point is the arithmetic mean of valid samples from that source: sum of sample values divided by sample count."
        case "resting_heart_rate": "Resting heart rate reported by the source, in beats per minute. Each daily point is the arithmetic mean of that day's valid samples."
        case "hrv_sdnn": "SDNN is the standard deviation of normal heartbeat intervals, reported in milliseconds by the source. Each daily point averages valid SDNN samples; the app does not recalculate SDNN from raw heartbeats."
        default: "The daily value is calculated from valid records within each source."
        }
    }
    var body: some View {
        List {
            Section("All sources") {
                Chart {
                    ForEach(Array(metric["history"].arrayValue.enumerated()), id: \.offset) { _, day in
                        ForEach(Array(day["alternatives"].arrayValue.enumerated()), id: \.offset) { _, source in
                        if let value = source["value"].numberValue {
                            PointMark(x: .value("Day", day["date"].stringValue), y: .value("Value", metric["record_type"] == .string("sleep") ? value / 3600 : value))
                                .foregroundStyle(by: .value("Source", source["source"].stringValue))
                        }
                        }
                    }
                }.chartForegroundStyleScale(domain: sourceKeys, range: sourceKeys.enumerated().map { SourceLegend.colors[$0.offset % SourceLegend.colors.count] })
                .chartLegend { SourceLegend(model: model, keys: sourceKeys, names: [:]) }
                .frame(height: 220).accessibilityLabel("Daily values from all sources")
                Text("\(metric["record_type"] == .string("sleep") ? "Hours" : metric["unit"].stringValue) · Points do not imply continuous coverage.").font(.caption)
            }
            Section("About this metric") {
                Text("Display unit: " + (metric["record_type"] == .string("sleep") ? "hours and minutes" : metric["unit"].stringValue))
                Text(calculationDescription)
                Text("Sources are summarized separately. Missing records are not counted as zero.").font(.caption)
                Text("The comparison uses 28 prior valid days from the selected source within 42 days. It describes your data and does not estimate disease risk.").font(.caption)
            }
            Section("Daily records") {
                ForEach(Array(metric["history"].arrayValue.reversed().enumerated()), id: \.offset) { _, day in
                    DisclosureGroup(day["date"].stringValue) {
                        ForEach(Array(day["alternatives"].arrayValue.enumerated()), id: \.offset) { _, source in
                            VStack(alignment: .leading) {
                                Text(metricValue(source, kind: metric["record_type"].stringValue))
                                Text(model.sourceLabel(source["source"].stringValue)).font(.caption)
                            }
                        }
                    }
                }
            }

        }.navigationTitle(metricTitle(metric["record_type"].stringValue))
    }
}

private func metricValue(_ selected: JSONValue, kind: String) -> String {
    guard let value = selected["value"].numberValue else { return "Not available" }
    return OverviewSummary.value(value, kind: kind, unit: selected["unit"].stringValue)
}
func metricTitle(_ kind: String) -> String {
    ["sleep": "Sleep", "workout": "Activity", "steps": "Steps", "resting_heart_rate": "Resting heart rate", "heart_rate": "Heart rate", "hrv_sdnn": "HRV SDNN"][kind] ?? kind
}

struct DataCenterView: View {
    var model: AppModel
    @State private var sources: [JSONValue] = []
    var body: some View {
        List {
            Section("Import") {
                NavigationLink("Health Report", value: DataDestination.reports)
                if !model.sharedReports.isEmpty { Button("Import shared PDFs (\(model.sharedReports.count))") { model.showSharedReportImport = true } }
                NavigationLink(value: DataDestination.health) {
                    HStack {
                        Text("Apple Health")
                        Spacer()
                        if model.isHealthSyncRunning { ProgressView().accessibilityHidden(true) }
                    }
                }.accessibilityValue(model.isHealthSyncRunning ? "Syncing" : "")
                NavigationLink("Import GPX, TCX, or FIT activities", destination: TrainingImportsView(model: model))
            }
            Section("Export") {
                NavigationLink("Export your complete archive", destination: SettingsView(model: model))
            }
            Section("Sources") {
                NavigationLink("Source aliases", destination: SourceAliasesView(model: model))
            }
            Section("Archived sources") {
                if sources.isEmpty { Text("No visible records. Sync to start archiving data.").foregroundStyle(.secondary) }
                ForEach(Array(sources.enumerated()), id: \.offset) { _, source in
                    VStack(alignment: .leading, spacing: 6) {
                        Text(model.sourceLabel(SourceNames.key(source)))
                        Text("\(source["platform"].stringValue) · \(source["record_type"].stringValue)").font(.caption)
                        Text("\(Int(source["record_count"].numberValue ?? 0)) records").font(.caption)
                    }
                }
            }
        }.navigationTitle("Data")
            .navigationDestination(for: DataDestination.self) { destination in
                switch destination {
                case .reports: ReportsView(model: model)
                case .health: HealthView(model: model)
                }
            }
            .toolbar { NavigationLink(destination: SettingsView(model: model)) { Label("Settings", systemImage: "gearshape") } }
            .initialTask { await reload() }.refreshable { await reload() }
    }
    private func reload() async {
        guard let client = model.client, let token = model.session?.token else { return }
        do {
            let result = try await model.request("wellness/sources", body: .object([:]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            sources = result["sources"].arrayValue
        } catch { if !Task.isCancelled, model.session?.token == token { model.errorMessage = error.localizedDescription } }
    }
}

private struct OverviewPreferencesView: View {
    var model: AppModel
    @Binding var preferences: JSONValue
    @Environment(\.dismiss) private var dismiss
    @State private var favorites: Set<String> = []
    @State private var target = ""
    @State private var snapshot: JSONValue = .null
    @State private var failure: String?
    @State private var loading = true
    private let kinds = ["sleep", "workout", "steps", "resting_heart_rate", "heart_rate", "hrv_sdnn"]
    var body: some View {
        Form {
            Section("Favorite metrics") {
                ForEach(kinds, id: \.self) { kind in
                    Toggle(metricTitle(kind), isOn: Binding(get: { favorites.contains(kind) }, set: { selected in
                        if selected { favorites.insert(kind) } else { favorites.remove(kind) }
                    }))
                }
                Text("No favorites shows all six metrics.").font(.caption)
            }
            Section("Your daily sleep target") {
                TextField("Minutes, optional", text: $target).keyboardType(.numberPad)
                Text("This is your own goal, not a calculated biological sleep need.").font(.caption)
            }
            if let failure { Section { Text(failure) } }
            Button("Save preferences") { Task { await save() } }.disabled(loading || snapshot == .null)
        }.disabled(loading).interactiveDismissDisabled().navigationTitle("Customize overview")
            .toolbar { Button("Cancel") { dismiss() } }
            .initialTask { await load() }
    }
    private func load() async {
        guard let client = model.client, let token = model.session?.token else { loading = false; return }
        do {
            let result = try await model.request("wellness/preferences/get", body: .object([:]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            snapshot = result
            favorites = Set(result["preferences"]["favorite_metrics"].arrayValue.map(\.stringValue))
            target = result["preferences"]["sleep_target_minutes"].numberValue.map { String(Int($0)) } ?? ""
            loading = false
        } catch { if !Task.isCancelled, model.session?.token == token { failure = error.localizedDescription; loading = false } }
    }
    private func save() async {
        guard let client = model.client, let token = model.session?.token else { return }
        let text = target.trimmingCharacters(in: .whitespacesAndNewlines)
        guard text.isEmpty || Int(text).map({ (1...1440).contains($0) }) == true else { failure = "Use a whole number from 1 to 1440 minutes."; return }
        var updated = snapshot["preferences"]
        updated["favorite_metrics"] = .array(kinds.filter { favorites.contains($0) }.map(JSONValue.string))
        updated["sleep_target_minutes"] = text.isEmpty ? .null : .number(Double(text) ?? 0)
        loading = true
        do {
            let result = try await model.request("wellness/preferences/save", body: .object(["expected_version": snapshot["version"], "batch_id": .string(UUID().uuidString), "preferences": updated]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            preferences = result
            try model.archive?.save(result, name: "wellness-preferences.json")
            dismiss()
        } catch { if !Task.isCancelled, model.session?.token == token { failure = error.localizedDescription; loading = false } }
    }
}
