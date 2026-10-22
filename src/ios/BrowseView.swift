import SwiftUI

struct BrowseView: View {
    var model: AppModel
    @State private var labs: [JSONValue] = []
    @State private var labFailure: String?
    @State private var daily: [JSONValue] = []
    @State private var failure: String?
    @State private var loading = false
    @State private var search = ""
    private func matches(_ texts: String...) -> Bool {
        let words = search.split(whereSeparator: { $0.isWhitespace }).map(String.init)
        return words.allSatisfy { word in texts.contains { $0.localizedStandardContains(word) } }
    }
    private var filteredDaily: [JSONValue] { daily.filter { metric in !MetricMeasurementsView.kinds.contains { $0.0 == metric["record_type"].stringValue } }.filter { matches(metricTitle($0["record_type"].stringValue), $0["record_type"].stringValue, $0["unit"] == .string("kcal") ? "calories kcal" : "") } }
    private var filteredLabs: [JSONValue] { labs.filter { matches($0["name"].stringValue, $0["metric_id"].stringValue, $0["standard_unit"].stringValue) } }
    var body: some View {
        List {
            Section("Daily metrics") {
                if loading { ProgressView("Loading metrics") }
                if let failure { Text(failure).foregroundStyle(.secondary) }
                ForEach(Array(filteredDaily.enumerated()), id: \.offset) { _, metric in
                    NavigationLink(metricTitle(metric["record_type"].stringValue), destination: DailyMetricView(model: model, metric: metric))
                }
                if matches("Sleep sessions") { NavigationLink("Sleep sessions", destination: SleepSessionsView(model: model)) }
            }
            Section("Measurements") {
                ForEach(MetricMeasurementsView.kinds.filter { matches($0.0, $0.1) }, id: \.0) { kind, title in
                    NavigationLink(title, destination: MetricMeasurementsView(model: model, kind: kind))
                }
            }
            Section("Lab Results") {
                if let labFailure { Text(labFailure).foregroundStyle(.secondary) }
                if !loading && labFailure == nil && labs.isEmpty { Text("No imported lab results yet.").foregroundStyle(.secondary) }
                ForEach(filteredLabs, id: \.identifier) { metric in
                    NavigationLink(metric["name"].stringValue, destination: MetricHistoryView(model: model, metric: metric))
                }
            }
            if !search.isEmpty && filteredDaily.isEmpty && filteredLabs.isEmpty && !MetricMeasurementsView.kinds.contains(where: { matches($0.0, $0.1) }) && !matches("Sleep sessions") && !loading {
                ContentUnavailableView.search(text: search)
            }
        }.navigationTitle("Browse")
            .searchable(text: $search, placement: .navigationBarDrawer(displayMode: .always), prompt: "Search metrics")
            .initialTask { await reload() }.refreshable { await reload() }
    }
    private func reload() async {
        guard let token = model.session?.token else { return }
        loading = true
        failure = nil
        labFailure = nil
        defer { loading = false }
        do {
            if model.sourceCatalog.isEmpty { _ = try await model.request("wellness/sources", body: .object([:])) }
            if model.sourcePreferences == .null { _ = try await model.request("wellness/preferences/get", body: .object([:])) }
        } catch {
            guard !Task.isCancelled, model.session?.token == token else { return }
            failure = "Source names could not be loaded. Pull to refresh."
        }
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
