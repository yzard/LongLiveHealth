import SwiftUI

struct BrowseView: View {
    var model: AppModel
    @State private var labs: [JSONValue] = []
    @State private var labFailure: String?
    @State private var daily: [JSONValue] = []
    @State private var failure: String?
    @State private var loading = false
    var body: some View {
        List {
            Section("Daily metrics") {
                if loading { ProgressView("Loading metrics") }
                if let failure { Text(failure).foregroundStyle(.secondary) }
                ForEach(Array(daily.enumerated()), id: \.offset) { _, metric in
                    NavigationLink(metricTitle(metric["record_type"].stringValue), destination: DailyMetricView(model: model, metric: metric))
                }
                NavigationLink("Sleep sessions", destination: SleepSessionsView(model: model))
            }
            Section("Lab Results") {
                if let labFailure { Text(labFailure).foregroundStyle(.secondary) }
                if !loading && labFailure == nil && labs.isEmpty { Text("No imported lab results yet.").foregroundStyle(.secondary) }
                ForEach(labs, id: \.identifier) { metric in
                    NavigationLink(metric["name"].stringValue, destination: MetricHistoryView(model: model, metric: metric))
                }
            }
            Section("More") {
                NavigationLink("Daily logs", destination: DailyLogsView(model: model))
                NavigationLink("Device trends", destination: DeviceTrendsView(model: model))
                NavigationLink("Timeline", destination: ArchiveTimelineView(model: model))
                NavigationLink("Breathing practice", destination: BreathingPracticeView(model: model))
            }
        }.navigationTitle("Browse").initialTask { await reload() }.refreshable { await reload() }
    }
    private func reload() async {
        guard let token = model.session?.token else { return }
        loading = true
        failure = nil
        labFailure = nil
        defer { loading = false }
        do {
            let result = try await model.request("metrics/browse", body: .object([:]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            labs = result["metrics"].arrayValue
        } catch {
            guard !Task.isCancelled, model.session?.token == token else { return }
            labFailure = error.localizedDescription
        }
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd"
        do {
            let result = try await model.request("wellness/day", body: .object([
                "date": .string(formatter.string(from: Date())), "timezone": .string(TimeZone.current.identifier), "source_priority": .object([:])
            ]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            daily = result["metrics"].arrayValue
        } catch {
            guard !Task.isCancelled, model.session?.token == token else { return }
            failure = error.localizedDescription
        }
    }
}
