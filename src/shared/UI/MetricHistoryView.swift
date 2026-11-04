import SwiftUI
#if canImport(Charts)
import Charts
#endif

struct MetricHistoryView: View {
    var model: AppModel
    let metric: JSONValue
    @State var history: JSONValue = .null
    @State var loading = false
    @State var failure: String?

    private var points: [JSONValue] { history["points"].arrayValue }
    private var records: [JSONValue] { history["records"].arrayValue }
    private var chartUnits: [String] { Array(Set(points.map { $0["unit"].stringValue })).sorted() }
    private var sourceKeys: [String] { Array(Set(points.map(SourceNames.key))).sorted() }
    private var sourceNames: [String: String] { Dictionary(points.map { (SourceNames.key($0), $0["source_name"].stringValue) }, uniquingKeysWith: { first, _ in first }) }

    var body: some View {
        MetricDetailScreen(title: metric["name"].stringValue) {
            MetricChartSection {
                if loading { ProgressView("Loading measurements") }
                if let failure { Text(failure).foregroundStyle(.secondary) }
                if !loading && points.isEmpty && records.isEmpty && failure == nil {
                    Text("No comparable measurements yet. Confirm imported results with a metric, unit and collection date.").foregroundStyle(.secondary)
                }
                if !loading && points.isEmpty && !records.isEmpty { Text("Open Historical Values to see the results. A chart requires an exact numeric result, unit and collection date.").font(.caption).foregroundStyle(.secondary) }
                if !points.isEmpty {
                    ForEach(chartUnits, id: \.self) { unit in
                    MetricChart(model: model, guidance: history["reference_guidance"], ranges: history["chart_ranges"], unit: unit, metricKind: metric["metric_id"].stringValue,
                                divisor: 1, sourceKeys: sourceKeys, names: sourceNames,
                                initialPeriod: .week, showsHour: false, heartRate: false, navigationTimestamps: nil, onWindowChange: nil)
                        .environment(\.timeZone, .gmt)

                    }
                }
                if let count = history["excluded_count"].numberValue, count > 0 {
                    DisclosureGroup("\(Int(count)) records could not be plotted") {
                        ForEach(Array(history["excluded"].arrayValue.enumerated()), id: \.offset) { _, record in
                            Text(record["reason"].stringValue.replacingOccurrences(of: "_", with: " ")).font(.caption)
                        }
                        Text("Original data remains in your archive.").font(.caption)
                    }
                }
            }
            Section("About this metric") {
                MetricAboutView(id: metric["metric_id"].stringValue, name: metric["name"].stringValue)
            }
            Section {
                NavigationLink("Historical Values") {
                    MetricHistoricalValuesView(model: model, records: records, points: points)
                }
            }
        }
        .initialTask { await reload() }
        .onChange(of: model.sourcePreferences["preferences"]["reference_profile"]) { _, _ in Task { await reload() } }
        .refreshable { await reload() }
    }

    private func reload() async {
        guard let token = model.session?.token else { return }
        loading = true
        failure = nil
        defer { loading = false }
        do {
            let result = try await model.request("metrics/history/get", body: .object(["metric_id": metric["metric_id"]]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            history = result
        } catch {
            guard !Task.isCancelled, model.session?.token == token else { return }
            failure = error.localizedDescription
        }
    }

}

struct MetricHistoricalValuesView: View {
    var model: AppModel
    let records: [JSONValue]
    let points: [JSONValue]
    @State var preview: DocumentLocation?
    @State var failure: String?

    var body: some View {
        List {
            if let failure { Text(failure).foregroundStyle(.secondary) }
            if records.isEmpty && points.isEmpty { Text("No historical values yet.").foregroundStyle(.secondary) }
            Section("Historical values") {
                let ids = Set(records.map(\.historyPointID).filter { !$0.isEmpty })
                let all = records + points.filter { $0.historyPointID.isEmpty || !ids.contains($0.historyPointID) }
                ForEach(Array(HistoricalValue.newestFirst(all, timestamps: ["timestamp"], dates: ["sampled_at"]).enumerated()), id: \.offset) { _, record in
                    DisclosureGroup {
                        Text(model.sourceLabel(record: record)).font(.subheadline)
                        if !record["original_records"].arrayValue.isEmpty {
                            DisclosureGroup("Original results (\(record["original_records"].arrayValue.count))") {
                                ForEach(Array(record["original_records"].arrayValue.enumerated()), id: \.offset) { _, item in
                                    let quantity = item["observation"]["valueQuantity"]
                                    let unit = quantity["unit"].stringValue.isEmpty ? quantity["code"].stringValue : quantity["unit"].stringValue
                                    Text("\(quantity["comparator"].stringValue)\(quantity["value"].numberValue?.formatted() ?? record["display_value"].stringValue) \(HistoricalValue.displayUnit(unit, kind: ""))").font(.caption)
                                }
                            }
                        }
                        if !record["report_id"].stringValue.isEmpty {
                            Button("View original report") { Task { await showSource(record) } }
                            NavigationLink("Review report", destination: ReportView(model: model, reportID: record["report_id"].stringValue))
                        }
                        if !record["status"].stringValue.isEmpty { Text(record["status"].stringValue.capitalized).font(.caption) }
                    } label: {
                        let value = record["display_value"] == .null ? record["value"] : record["display_value"]
                        let unit = record["unit"].stringValue
                        let numeric = value.numberValue != nil
                        HistoricalValueRow(timestamp: nil, dateText: record["sampled_at"].stringValue,
                            source: model.sourceLabel(record: record), value: HistoricalValue.quantity(value, unit: unit.isEmpty && numeric ? "Unit Not Specified" : unit))
                    }
                }
            }
        }
        .mobileNavigationTitle("Historical Values")
        .navigationBarTitleDisplayMode(.inline)
        .sheet(item: $preview) { DocumentPreview(location: $0) }
    }

    private func showSource(_ point: JSONValue) async {
        guard let token = model.session?.token else { return }
        do {
            let url = try await model.source(point["report_id"].stringValue)
            guard !Task.isCancelled, model.session?.token == token else { return }
            preview = DocumentLocation(url: url, page: Int(point["original"]["source"]["page"].numberValue ?? 1))
        } catch {
            guard !Task.isCancelled, model.session?.token == token else { return }
            failure = error.localizedDescription
        }
    }
}

private extension JSONValue {
    var historyPointID: String { self["point_id"].stringValue }
}
