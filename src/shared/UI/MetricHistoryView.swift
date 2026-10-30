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
                    MetricChart(model: model, ranges: history["chart_ranges"], unit: unit,
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
                ForEach(records.reversed(), id: \.historyPointID) { record in
                    VStack(alignment: .leading, spacing: 6) {
                        Text((record["display_value"].stringValue.isEmpty ? "—" : record["display_value"].stringValue) + (record["unit"].stringValue.isEmpty ? "" : " " + record["unit"].stringValue)).font(.headline)
                        Text(record["sampled_at"].stringValue.isEmpty ? "—" : record["sampled_at"].stringValue).font(.caption)
                        Text(model.sourceLabel(record: record)).font(.caption).foregroundStyle(.secondary)
                        if !record["original_records"].arrayValue.isEmpty {
                            DisclosureGroup("Original results (\(record["original_records"].arrayValue.count))") {
                                ForEach(Array(record["original_records"].arrayValue.enumerated()), id: \.offset) { _, item in
                                    let quantity = item["observation"]["valueQuantity"]
                                    Text("\(quantity["comparator"].stringValue)\(quantity["value"].numberValue?.formatted() ?? record["display_value"].stringValue) \(quantity["unit"].stringValue.isEmpty ? quantity["code"].stringValue : quantity["unit"].stringValue)").font(.caption)
                                }
                            }
                        }
                        if !record["report_id"].stringValue.isEmpty {
                            Button("View original report") { Task { await showSource(record) } }
                            NavigationLink("Review report", destination: ReportView(model: model, reportID: record["report_id"].stringValue))
                        }
                        if !record["status"].stringValue.isEmpty { Text(record["status"].stringValue.capitalized).font(.caption) }
                    }
                }
                ForEach(points.filter { point in !records.contains { $0["point_id"] == point["point_id"] } }.reversed(), id: \.historyPointID) { point in
                    VStack(alignment: .leading, spacing: 6) {
                        Text(model.sourceLabel(record: point)).font(.headline)
                        Text("\(point["value"].numberValue?.formatted() ?? (point["value"].stringValue.isEmpty ? "—" : point["value"].stringValue)) \(point["unit"].stringValue)").stableDigits()
                        Text(point["sampled_at"].stringValue.isEmpty ? "—" : point["sampled_at"].stringValue).font(.caption)
                        if !point["report_id"].stringValue.isEmpty {
                            Text(point["filename"].stringValue).font(.caption).foregroundStyle(.secondary)
                            Text("Original: \(point["original"]["raw_result"].stringValue) \(point["original"]["raw_unit"].stringValue)").font(.caption)
                            Button("View original report") { Task { await showSource(point) } }
                            NavigationLink("Review report", destination: ReportView(model: model, reportID: point["report_id"].stringValue))
                        } else {
                            let provider = point["original"]["provider"].stringValue.isEmpty ? point["original"]["payload"]["source_name"].stringValue : point["original"]["provider"].stringValue
                            if !provider.isEmpty { Text(model.sourceLabel(SourceNames.key(point), name: provider)).font(.caption).foregroundStyle(.secondary) }
                        }
                    }.padding(.vertical, 4)
                }
            }
        }
        .navigationTitle("Historical Values")
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
