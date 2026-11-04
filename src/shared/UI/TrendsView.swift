import SwiftUI

struct TrendsView: View {
    var model: AppModel
    private let kinds = ["sleep", "steps", "workout", "resting_heart_rate"]
    @State var window: TrendComparison.Window?
    @State var summaries: [String: [TrendComparison.Summary]] = [:]
    @State var failures: [String: String] = [:]
    @State var loading = false

    var body: some View {
        List {
            Section {
                Text("Recorded trends").font(.title2.bold())
                if let window {
                    Text("Recent: \(window.recentStart) – \(window.last)")
                        .font(.subheadline)
                }
                InfoHelp(title: "How trends are calculated",
                         message: "Compare the median on observed days in the last 7 completed days with the preceding 28. Each source and unit is compared separately. A comparison needs at least 4 recent and 14 earlier days. Missing days are omitted. Workout duration describes recorded workout days. These comparisons describe changes in your records.")
                if loading { ProgressView("Loading trends") }
                if !loading && failures.isEmpty && summaries.values.allSatisfy(\.isEmpty) {
                    Text("No observations in these periods.").foregroundStyle(.secondary)
                }
            }
            ForEach(kinds.flatMap { summaries[$0] ?? [] }) { row in
                tile(row, kind: row.kind)
            }
            ForEach(kinds, id: \.self) { kind in
                if let failure = failures[kind] {
                    Section(metricTitle(kind)) { Text(failure).foregroundStyle(.secondary) }
                }
            }
        }
        .mobileNavigationTitle("Trends")
        .initialTask { await reload() }
        .refreshable { await reload() }
    }

    private func tile(_ row: TrendComparison.Summary, kind: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(metricTitle(kind)).font(.headline)
            Text(model.sourceLabel(row.source)).font(.subheadline).foregroundStyle(.secondary)
            if let recent = row.recentMedian, let baseline = row.baselineMedian {
                let scale = max(recent, baseline, 1)
                HStack(alignment: .bottom, spacing: 16) {
                    bar(recent, scale: scale, label: "Last 7 days", kind: kind, unit: row.unit, color: .accentColor)
                    bar(baseline, scale: scale, label: "Prior 28 days", kind: kind, unit: row.unit, color: .secondary)
                }
            } else {
                Text("Not enough comparable days").font(.subheadline)
            }
            Text("\(row.recentDays)/7 recent · \(row.baselineDays)/28 earlier days")
                .font(.caption).foregroundStyle(.secondary)
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.primary.opacity(0.06), in: RoundedRectangle(cornerRadius: 18))
        .listRowSeparator(.hidden)
    }

    private func bar(_ value: Double, scale: Double, label: String, kind: String, unit: String, color: Color) -> some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(OverviewSummary.value(value, kind: kind, unit: unit)).font(.headline)
            RoundedRectangle(cornerRadius: 5)
                .fill(color)
                .frame(width: 30, height: max(1, 56 * value / scale))
                .accessibilityHidden(true)
            Text(label).font(.caption).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func reload() async {
        guard !loading, let token = model.session?.token else { return }
        loading = true
        defer { loading = false }
        let scope = dateScope(days: 1)
        let nextWindow = TrendComparison.Window(lastCompleteDay: scope["start_date"].stringValue)
        var nextSummaries: [String: [TrendComparison.Summary]] = [:]
        var nextFailures: [String: String] = [:]
        for kind in kinds {
            do {
                let response = try await model.request("wellness/history", body: .object([
                    "record_type": .string(kind), "timezone": scope["timezone"],
                    "dates": .array([.string(nextWindow.first), .string(nextWindow.last)])
                ]))
                nextSummaries[kind] = TrendComparison.summaries(kind: kind, history: response["history"].arrayValue, window: nextWindow)
            } catch {
                guard !Task.isCancelled, model.session?.token == token else { return }
                nextFailures[kind] = error.localizedDescription
            }
        }
        guard !Task.isCancelled, model.session?.token == token else { return }
        window = nextWindow
        summaries = nextSummaries
        failures = nextFailures
    }
}
