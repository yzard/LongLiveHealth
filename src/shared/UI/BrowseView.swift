import SwiftUI

struct BrowseView: View {
    var model: AppModel
    @State var healthDefinitions: [JSONValue] = []
    @State var labs: [JSONValue] = []
    @State var labFailure: String?
    @State var daily: [JSONValue] = []
    @State var failure: String?
    @State var loading = false
    @State var search = ""
    private var measurementKinds: [(String, String)] {
        MetricMeasurementsView.kinds + healthDefinitions.filter { item in
            item["family"] == .string("quantity") && !["steps", "active_energy", "basal_energy", "resting_heart_rate", "hrv_sdnn"].contains(item["kind"].stringValue) && !MetricMeasurementsView.kinds.contains { k in k.0 == item["kind"].stringValue }
        }.map { ($0["kind"].stringValue, $0["title"].stringValue) }
    }
    private var recordDefinitions: [JSONValue] {
        healthDefinitions.filter { $0["family"] != .string("quantity") && !["sleep", "workout"].contains($0["kind"].stringValue) }
    }
    private func definition(_ kind: String) -> JSONValue { healthDefinitions.first { $0["kind"] == .string(kind) } ?? .null }
    private func matches(_ texts: String...) -> Bool {
        let words = search.split(whereSeparator: { $0.isWhitespace }).map(String.init)
        return words.allSatisfy { word in texts.contains { $0.localizedStandardContains(word) } }
    }
    private var entries: [BrowseEntry] {
        var result: [BrowseEntry] = []
        var kinds = Set<String>()
        for metric in daily {
            let kind = metric["record_type"].stringValue
            guard !MetricMeasurementsView.kinds.contains(where: { $0.0 == kind }), kinds.insert(kind).inserted else { continue }
            result.append(BrowseEntry(title: metricTitle(kind), kind: kind, choice: .daily(metric)))
        }
        result.append(BrowseEntry(title: "Sleep sessions", kind: "sleep", choice: .sleep))
        kinds.insert("sleep")
        for (kind, title) in measurementKinds where kinds.insert(kind).inserted {
            result.append(BrowseEntry(title: title, kind: kind, choice: .measurement(kind)))
        }
        for item in recordDefinitions {
            let kind = item["kind"].stringValue
            guard kinds.insert(kind).inserted else { continue }
            result.append(BrowseEntry(title: item["title"].stringValue, kind: kind, choice: .records(item)))
        }
        for metric in labs {
            result.append(BrowseEntry(title: metric["name"].stringValue, kind: metric["metric_id"].stringValue, choice: .lab(metric), laboratory: true))
        }
        return result.sorted { $0.title.localizedStandardCompare($1.title) == .orderedAscending }
    }
    private var searchedEntries: [BrowseEntry] {
        entries.filter { matches($0.title, $0.kind, $0.category.rawValue, $0.kind.contains("energy") ? "calories kcal" : "") }
    }
    #if os(Android)
    @State var chosen: BrowseChoice?
    var body: some View {
        AdaptiveDetailLayout(selected: chosen != nil, back: { chosen = nil }) {
            NavigationStack { metricList }
        } detail: {
            if let chosen {
                NavigationStack {
                    destination(chosen)
                }.id(chosen.identity)
            } else { EmptyContent(title: "Choose a metric", symbol: "chart.xyaxis.line") }
        }
    }
    #else
    var body: some View { metricList }
    #endif
    private var metricList: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                if loading { ProgressView("Loading metrics") }
                if let failure { Text(failure).foregroundStyle(.secondary) }
                if let labFailure { Text(labFailure).foregroundStyle(.secondary) }
                if search.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    categoryGrid(title: "Health Categories", clinical: false)
                    categoryGrid(title: "Health Records", clinical: true)
                } else {
                    LazyVStack(alignment: .leading, spacing: 16) {
                        ForEach(searchedEntries) { entry in
                            metricLink(entry)
                        }
                        if searchedEntries.isEmpty && !loading {
                            EmptyContent(title: "No results for \(search)", symbol: "magnifyingglass")
                        }
                    }
                }
            }.padding()
        }.navigationTitle("Browse")
            .metricsSearch($search)
            .initialTask { await reload() }.refreshable { await reload() }
    }
    private func categoryGrid(title: String, clinical: Bool) -> some View {
        let available = Set(entries.map(\.category))
        let categories = BrowseCategory.allCases.filter { category in
            category.isClinical == clinical && available.contains(category)
        }
        return VStack(alignment: .leading, spacing: 12) {
            if !categories.isEmpty {
                Text(title).font(.title2.bold())
                LazyVGrid(columns: [GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10)], spacing: 10) {
                    ForEach(categories) { category in
                        NavigationLink {
                            categoryList(category)
                        } label: {
                            VStack(alignment: .leading, spacing: 8) {
                                #if os(Android)
                                BrowseCategoryIcon(category: category, color: category.color)
                                #else
                                Image(systemName: category.symbol).font(.title).foregroundStyle(category.color)
                                #endif
                                Text(category.rawValue).font(.headline).foregroundStyle(.primary)
                                    .multilineTextAlignment(.leading)
                            }
                            .frame(maxWidth: .infinity, minHeight: 72, alignment: .leading)
                            .padding(12)
                            .background(Color.secondary.opacity(0.12), in: RoundedRectangle(cornerRadius: 20))
                        }.buttonStyle(.plain)
                    }
                }
            }
        }
    }
    private func categoryList(_ category: BrowseCategory) -> some View {
        List {
            ForEach(entries.filter { $0.category == category }) { entry in metricLink(entry) }
        }.navigationTitle(category.rawValue)
    }
    @ViewBuilder private func metricLink(_ entry: BrowseEntry) -> some View {
        #if os(Android)
        Button { chosen = entry.choice } label: { metricLabel(entry) }.buttonStyle(.plain)
        #else
        NavigationLink { destination(entry.choice) } label: { metricLabel(entry) }.buttonStyle(.plain)
        #endif
    }
    private func metricLabel(_ entry: BrowseEntry) -> some View {
        Text(entry.title).foregroundStyle(.primary)
            .frame(maxWidth: .infinity, minHeight: 44, alignment: .leading)
    }
    @ViewBuilder private func destination(_ choice: BrowseChoice) -> some View {
        switch choice {
        case .daily(let metric): DailyMetricView(model: model, metric: metric)
        case .sleep: SleepSessionsView(model: model)
        case .measurement(let kind): MetricMeasurementsView(model: model, kind: kind, definition: definition(kind))
        case .records(let item): HealthRecordHistoryView(model: model, definition: item)
        case .lab(let metric): MetricHistoryView(model: model, metric: metric)
        }
    }
    private func reload() async {
        guard let token = model.session?.token else { return }
        loading = true
        failure = nil
        labFailure = nil
        defer { loading = false }
        do {
            let sources = try await model.request("wellness/sources", body: .object([:]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            healthDefinitions = sources["browse_metrics"].arrayValue
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

enum BrowseChoice {
    case daily(JSONValue), sleep, measurement(String), lab(JSONValue), records(JSONValue)
    var identity: String {
        switch self {
        case .daily(let value): "daily:" + value["record_type"].stringValue
        case .sleep: "sleep"
        case .records(let value): "records:" + value["kind"].stringValue
        case .measurement(let kind): "measurement:" + kind
        case .lab(let value): "lab:" + value["metric_id"].stringValue
        }
    }
}

private struct BrowseEntry: Identifiable {
    let title: String
    let kind: String
    let choice: BrowseChoice
    var laboratory = false
    var id: String { choice.identity }
    var category: BrowseCategory { BrowseCategory.classify(id: kind, name: title, laboratory: laboratory) }
}

private extension BrowseCategory {
    var color: Color {
        switch self {
        case .heart, .vitals, .bloodWork: .pink
        case .activity, .mobility: .orange
        case .body, .cycle: .purple
        case .sleep, .vision: .indigo
        case .nutrition: .green
        case .wellbeing, .medications: .teal
        default: .blue
        }
    }
}
