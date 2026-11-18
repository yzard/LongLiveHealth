import SwiftUI
#if canImport(Charts)
import Charts
#endif

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
                if loading { ProgressView(uiText("Loading selected period")) }
                if let failure { Text(uiMessage(failure)).foregroundStyle(.secondary) }
                if metric["current"]["state"] == .string("query_limit_exceeded") {
                    Text(uiText("This history exceeds the daily query limit. Its records are saved, but the chart could not be loaded.")).foregroundStyle(.secondary)
                }
                MetricChart(model: model, guidance: displayed["reference_guidance"], ranges: displayed["chart_ranges"], unit: chartUnit, metricKind: metric["record_type"].stringValue,
                            divisor: ["sleep", "workout"].contains(metric["record_type"].stringValue) ? 3600 : 1,
                            sourceKeys: sourceKeys, names: [:], initialPeriod: .week, showsHour: false, heartRate: metric["record_type"] == .string("resting_heart_rate"), navigationTimestamps: [completeHistory["first_record_at"].numberValue, completeHistory["last_record_at"].numberValue].compactMap { $0 }, onWindowChange: { selectedWindow = $0 })
                    .environment(\.timeZone, .gmt)

            }
            Section(uiText("About this metric")) {
                MetricAboutView(id: metric["record_type"].stringValue, name: metricTitle(metric["record_type"].stringValue))
            }
            Section {
                if metric["record_type"] == .string("sleep") {
                    NavigationLink(uiText("Sleep Sessions")) { SleepSessionsView(model: model) }
                }
                NavigationLink(uiText("Historical Values")) {
                    List {
                        Section(uiText("Daily records")) {
                            ForEach(HistoricalValue.dailyRows(displayed["history"].arrayValue)) { row in
                                HistoricalValueRow(timestamp: nil, dateText: row.value["date"].stringValue,
                                    source: model.sourceLabel(row.value["source"].stringValue), value: metricValue(row.value, kind: metric["record_type"].stringValue))
                            }
                        }
                    }.mobileNavigationTitle(uiText("Historical Values")).navigationBarTitleDisplayMode(.inline)
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
        defer { if self.selectedWindow == selectedWindow { loading = false } }
        let formatter = DateFormatter(); formatter.calendar = Calendar(identifier: .gregorian); formatter.timeZone = .gmt; formatter.dateFormat = "yyyy-MM-dd"
        let dates = [selectedWindow.start, selectedWindow.start + selectedWindow.length - 1].map { JSONValue.string(formatter.string(from: Date(timeIntervalSince1970: $0))) }
        do {
            let response = try await model.request("wellness/history", body: .object(["record_type": metric["record_type"], "timezone": .string(TimeZone.current.identifier), "dates": selectedWindow.period == .all ? .null : .array(dates)]))
            guard !Task.isCancelled, self.selectedWindow == selectedWindow, model.session?.token == token else { return }
            completeHistory = response
            loadedWindow = selectedWindow
        } catch {
            guard !Task.isCancelled, self.selectedWindow == selectedWindow, model.session?.token == token else { return }
            failure = "Selected period could not be loaded: " + error.localizedDescription
        }

    }
}

private func metricValue(_ selected: JSONValue, kind: String) -> String {
    guard let value = selected["value"].numberValue else { return "Not available" }
    return MetricPresentation.value(value, kind: kind, unit: selected["unit"].stringValue)
}
func metricTitle(_ kind: String) -> String {
    ["sleep": "Sleep", "workout": "Workouts", "active_energy": "Active Energy", "basal_energy": "Resting Energy", "steps": "Steps", "resting_heart_rate": "Resting heart rate", "heart_rate": "Heart rate", "hrv_sdnn": "HRV SDNN"][kind] ?? kind
}

