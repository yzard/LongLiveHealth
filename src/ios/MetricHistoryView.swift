import SwiftUI
import Charts

struct MetricHistoryView: View {
    var model: AppModel
    let metric: JSONValue
    @State private var history: JSONValue = .null
    @State private var loading = false
    @State private var failure: String?
    @State private var preview: DocumentLocation?

    private var points: [JSONValue] { history["points"].arrayValue }
    private var records: [JSONValue] { history["records"].arrayValue }
    private var chartUnits: [String] { Array(Set(points.map { $0["unit"].stringValue })).sorted() }
    private var sourceKeys: [String] { Array(Set(points.map(SourceNames.key))).sorted() }
    private var sourceNames: [String: String] { Dictionary(points.map { (SourceNames.key($0), $0["source_name"].stringValue) }, uniquingKeysWith: { first, _ in first }) }

    var body: some View {
        List {
            Section("All sources") {
                if loading { ProgressView("Loading measurements") }
                if let failure { Text(failure).foregroundStyle(.secondary) }
                if !loading && points.isEmpty && records.isEmpty && failure == nil {
                    Text("No comparable measurements yet. Confirm imported results with a metric, unit and collection date.").foregroundStyle(.secondary)
                }
                if !loading && points.isEmpty && !records.isEmpty { Text("Results are listed below. A chart requires an exact numeric result, unit and collection date.").font(.caption).foregroundStyle(.secondary) }
                if !points.isEmpty {
                    ForEach(chartUnits, id: \.self) { unit in
                    Text(unit).font(.caption).foregroundStyle(.secondary)
                    Chart(points.filter { $0["unit"].stringValue == unit }, id: \.historyPointID) { point in
                        if let value = point["value"].numberValue, let timestamp = point["timestamp"].numberValue {
                            PointMark(x: .value("Collection date", Date(timeIntervalSince1970: timestamp)), y: .value("Value", value))
                                .foregroundStyle(by: .value("Source", SourceNames.key(point)))
                                .symbol(by: .value("Source", SourceNames.key(point)))
                                .accessibilityLabel(model.sourceLabel(record: point) + ", " + point["sampled_at"].stringValue)
                                .accessibilityValue("\(value.formatted()) \(point["unit"].stringValue)")
                        }
                    }
                    .chartForegroundStyleScale(domain: sourceKeys, range: sourceKeys.enumerated().map { SourceLegend.colors[$0.offset % SourceLegend.colors.count] })
                    .chartSymbolScale(domain: sourceKeys)
                    .chartLegend { SourceLegend(model: model, keys: sourceKeys, names: sourceNames) }
                    .chartLegend(position: .bottom, alignment: .leading)
                    .frame(height: 260)
                    .environment(\.timeZone, .gmt)
                    }
                    Text("Each point is one measurement. Equivalent units are combined; incompatible units use separate charts. Dates use UTC; sources are shown without averaging their values.").font(.caption).foregroundStyle(.secondary)
                }
                if let count = history["excluded_count"].numberValue, count > 0 {
                    DisclosureGroup("\(Int(count)) records could not be plotted") {
                        ForEach(Array(history["excluded"].arrayValue.enumerated()), id: \.offset) { _, record in
                            Text(record["reason"].stringValue.replacingOccurrences(of: "_", with: " ")).font(.caption)
                        }
                        Text("Original data remains in your archive.").font(.caption)
                    }
                }
                Button("Refresh history") { Task { await reload() } }.disabled(loading)
            }
            Section("About this metric") {
                Text(metric["name"].stringValue).font(.headline)
                ForEach(Array(Set(records.flatMap { $0["original"]["code"]["coding"].arrayValue.compactMap { coding -> String? in
                    let description = coding["display"].stringValue
                    return description.isEmpty || description == metric["name"].stringValue ? nil : description
                } })).sorted(), id: \.self) { description in
                    Text(description)
                }
                Text("This test uses the result supplied by the laboratory. Long Live Health does not recalculate the laboratory assay or average results from different sources.")
                Text("Calculation: supported unit conversions are applied to the reported value. Equivalent results from the same source and collection time are shown once; originals are retained.").font(.caption)
                let standardUnit = metric["standard_unit"].stringValue
                Text(standardUnit.isEmpty ? "Reported units: " + (chartUnits.isEmpty ? "See original results" : chartUnits.joined(separator: ", ")) : "Standard display unit: " + standardUnit)
                ForEach(Array(Set(records.compactMap { record -> String? in
                    let method = record["method"]["text"].stringValue
                    return method.isEmpty ? nil : method
                })).sorted(), id: \.self) { method in
                    Text("Laboratory method: " + method).font(.caption)
                }
            }
            Section("Historical values") {
                ForEach(records.reversed(), id: \.historyPointID) { record in
                    VStack(alignment: .leading, spacing: 6) {
                        Text(record["display_value"].stringValue + (record["unit"].stringValue.isEmpty ? "" : " " + record["unit"].stringValue)).font(.headline)
                        Text(record["sampled_at"].stringValue.isEmpty ? "Collection date not supplied" : record["sampled_at"].stringValue).font(.caption)
                        Text(model.sourceLabel(record: record)).font(.caption).foregroundStyle(.secondary)
                        if !record["original_records"].arrayValue.isEmpty {
                            DisclosureGroup("Original results (\(record["original_records"].arrayValue.count))") {
                                ForEach(Array(record["original_records"].arrayValue.enumerated()), id: \.offset) { _, item in
                                    let quantity = item["observation"]["valueQuantity"]
                                    Text("\(quantity["comparator"].stringValue)\(quantity["value"].numberValue?.formatted() ?? record["display_value"].stringValue) \(quantity["unit"].stringValue.isEmpty ? quantity["code"].stringValue : quantity["unit"].stringValue)").font(.caption)
                                }
                            }
                        }
                        if !record["status"].stringValue.isEmpty { Text(record["status"].stringValue.capitalized).font(.caption) }
                    }
                }
                ForEach(points.filter { records.isEmpty || ($0["channel"].stringValue != "Apple Health" && $0["original"]["clinical_observation"] == .null) }.reversed(), id: \.historyPointID) { point in
                    VStack(alignment: .leading, spacing: 6) {
                        Text(model.sourceLabel(record: point)).font(.headline)
                        Text("\(point["value"].numberValue?.formatted() ?? point["value"].stringValue) \(point["unit"].stringValue)").monospacedDigit()
                        Text(point["sampled_at"].stringValue).font(.caption)
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
        .navigationTitle(metric["name"].stringValue)
        .navigationBarTitleDisplayMode(.inline)
        .initialTask { await reload() }
        .refreshable { await reload() }
        .sheet(item: $preview) { DocumentPreview(location: $0) }
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
