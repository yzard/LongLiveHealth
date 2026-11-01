import SwiftUI

struct BrowseView: View {
    var model: AppModel
    @State var catalog: JSONValue = .null
    @State var failure: String?
    @State var loading = false
    @State var search = ""
    private var categories: [BrowseCategory] { catalog["categories"].arrayValue.map { BrowseCategory($0) } }
    private func matches(_ texts: String...) -> Bool {
        let words = search.split(whereSeparator: { $0.isWhitespace }).map(String.init)
        return words.allSatisfy { word in texts.contains { $0.localizedStandardContains(word) } }
    }
    private var entries: [BrowseEntry] { catalog["entries"].arrayValue.map { BrowseEntry(value: $0) } }
    private var searchedEntries: [BrowseEntry] {
        entries.filter { entry in matches(entry.title, entry.kind, categories.first { $0.id == entry.categoryID }?.title ?? "") }
    }
    #if os(Android)
    @State var chosen: BrowseChoice?
    var body: some View {
        AdaptiveDetailLayout(selected: chosen != nil) {
            NavigationStack { metricList }
        } detail: {
            NavigationStack {
                EmptyContent(title: "Choose a metric", symbol: "chart.xyaxis.line")
                    .navigationDestination(isPresented: Binding(get: { chosen != nil }, set: { if !$0 { chosen = nil } })) {
                        if let chosen { AnyView(destination(chosen)).id(chosen.identity) }
                    }
            }
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
                if search.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    ForEach(Array(Set(categories.map(\.section))).sorted(), id: \.self) { section in
                        categoryGrid(title: section)
                    }
                    if entries.isEmpty && !loading && failure == nil {
                        EmptyContent(title: "No health data", symbol: "heart")
                    }
                } else {
                    VStack(alignment: .leading, spacing: 16) {
                        ForEach(searchedEntries) { entry in
                            metricLink(entry)
                        }
                        if searchedEntries.isEmpty && !loading {
                            EmptyContent(title: "No results for \(search)", symbol: "magnifyingglass")
                        }
                    }
                }
            }.padding()
        }.mobileNavigationTitle("Browse")
            .metricsSearch($search)
            .initialTask { await reload() }.refreshable { await reload() }
    }
    private func categoryGrid(title: String) -> some View {
        let categories = categories.filter { $0.section == title }
        return VStack(alignment: .leading, spacing: 12) {
            if !categories.isEmpty {
                Text(title).font(.title2.bold())
                // A lazy grid becomes its own scrolling container on Android.
                // Keep these small category rows in the page's single ScrollView.
                VStack(spacing: 10) {
                    ForEach(0..<((categories.count + 1) / 2), id: \.self) { row in
                        HStack(spacing: 10) {
                            categoryTile(categories[row * 2])
                            if row * 2 + 1 < categories.count {
                                categoryTile(categories[row * 2 + 1])
                            } else {
                                Color.clear.frame(maxWidth: .infinity)
                            }
                        }
                    }
                }
            }
        }
    }
    private func categoryTile(_ category: BrowseCategory) -> some View {
        NavigationLink {
            categoryList(category)
        } label: {
            VStack(alignment: .leading, spacing: 8) {
                #if os(Android)
                BrowseCategoryIcon(category: category, color: category.color)
                #else
                Image(systemName: category.symbol).font(.title).foregroundStyle(category.color)
                #endif
                Text(category.title).font(.headline).foregroundStyle(.primary)
                    .multilineTextAlignment(.leading)
            }
            .frame(maxWidth: .infinity, minHeight: 72, alignment: .leading)
            .padding(12)
            .background(Color.secondary.opacity(0.12), in: RoundedRectangle(cornerRadius: 20))
        }.buttonStyle(.plain).frame(maxWidth: .infinity)
    }
    private func categoryList(_ category: BrowseCategory) -> some View {
        List {
            ForEach(entries.filter { $0.categoryID == category.id }) { entry in metricLink(entry) }
        }.mobileNavigationTitle(category.title)
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
        case .sleep: DailyMetricView(model: model, metric: .object(["record_type": .string("sleep")]))
        case .measurement(let item): MetricMeasurementsView(model: model, kind: item["kind"].stringValue, definition: item)
        case .records(let item):
            if item["kind"] == .string("electrocardiogram") { ECGRecordsView(model: model) }
            else { HealthRecordHistoryView(model: model, definition: item, historicalOnly: false) }
        case .lab(let metric): MetricHistoryView(model: model, metric: metric)
        }
    }
    private func reload() async {
        guard let token = model.session?.token else { return }
        loading = true; failure = nil
        defer { loading = false }
        do {
            if catalog == .null, let snapshot = try model.cachedResponse("browse/catalog", body: .object([:])) {
                catalog = snapshot
            }
            let result = try await model.request("browse/catalog", body: .object([:]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            catalog = result
        } catch {
            guard !Task.isCancelled, model.session?.token == token else { return }
            failure = error.localizedDescription
        }
    }
}

enum BrowseChoice {
    case daily(JSONValue), sleep, measurement(JSONValue), lab(JSONValue), records(JSONValue)
    var identity: String {
        switch self {
        case .daily(let value): "daily:" + value["record_type"].stringValue
        case .sleep: "sleep"
        case .records(let value): "records:" + value["kind"].stringValue
        case .measurement(let value): "measurement:" + value["kind"].stringValue
        case .lab(let value): "lab:" + value["metric_id"].stringValue
        }
    }
}

private struct BrowseEntry: Identifiable {
    let value: JSONValue
    var id: String { value["id"].stringValue }
    var title: String { value["title"].stringValue }
    var kind: String { value["kind"].stringValue }
    var categoryID: String { value["category_id"].stringValue }
    var choice: BrowseChoice {
        let definition = value["definition"]
        switch value["presentation"].stringValue {
        case "sleep": return .sleep
        case "daily": return .daily(.object(["record_type": .string(kind), "title": .string(title), "unit": definition["unit"]]))
        case "measurement": return .measurement(definition)
        case "lab": return .lab(definition)
        default: return .records(definition)
        }
    }
}

private extension BrowseCategory {
    var color: Color {
        switch tint {
        case "pink": .pink
        case "orange": .orange
        case "purple": .purple
        case "indigo": .indigo
        case "green": .green
        case "teal": .teal
        default: .blue
        }
    }
}
