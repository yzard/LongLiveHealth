import SwiftUI
#if canImport(Charts)
import Charts
#endif

struct MetricHistoryView: View {
    var model: AppModel
    let metric: JSONValue
    @State var history: JSONValue = .null
    @State var historyRevision: Int?
    @State var loading = false
    @State var failure: String?
    @State var editing = false

    private var metricName: String {
        model.renamedLabMetrics[metric["metric_id"].stringValue]
            ?? (currentHistory["metric"]["name"] == .null ? nil : currentHistory["metric"]["name"].stringValue)
            ?? metric["name"].stringValue
    }

    private var currentHistory: JSONValue { historyRevision == model.reportDataRevision ? history : .null }
    private var points: [JSONValue] { currentHistory["points"].arrayValue }
    private var records: [JSONValue] { currentHistory["records"].arrayValue }
    private var incompatibleUnits: [String] { currentHistory["chart_unit_conflict"].arrayValue.map(\.stringValue) }
    private var showsChart: Bool { !points.isEmpty && incompatibleUnits.isEmpty }
    private var chartUnits: [String] { Array(Set(points.map { $0["unit"].stringValue })).sorted() }
    private var sourceKeys: [String] { Array(Set(points.map(SourceNames.key))).sorted() }
    private var sourceNames: [String: String] { Dictionary(points.map { (SourceNames.key($0), $0["source_name"].stringValue) }, uniquingKeysWith: { first, _ in first }) }

    var body: some View {
        MetricDetailScreen(title: metricName) {
            if loading {
                ProgressView("Loading measurements")
            } else if let failure {
                Text(failure).foregroundStyle(.secondary)
            } else if currentHistory != .null && points.isEmpty && records.isEmpty {
                Text("No historical values yet.").foregroundStyle(.secondary)
            }
            if showsChart {
                MetricChartSection {
                    ForEach(chartUnits, id: \.self) { unit in
                        MetricChart(model: model, guidance: currentHistory["reference_guidance"], ranges: currentHistory["chart_ranges"], unit: unit, metricKind: metric["metric_id"].stringValue,
                                    divisor: 1, sourceKeys: sourceKeys, names: sourceNames,
                                    initialPeriod: .week, showsHour: false, heartRate: false, navigationTimestamps: nil, onWindowChange: nil)
                            .environment(\.timeZone, .gmt)
                    }
                }
            }
            Section("About this metric") {
                MetricAboutView(id: metric["metric_id"].stringValue, name: metricName)
                ForEach(Array(currentHistory["metric"]["loinc"].arrayValue.enumerated()), id: \.offset) { _, entry in
                    VStack(alignment: .leading, spacing: 4) {
                        Text("LOINC \(entry["code"].stringValue)").font(.subheadline.weight(.semibold))
                        Text(entry["official_name"] == .null ? "Official name unavailable" : entry["official_name"].stringValue)
                            .font(.caption).foregroundStyle(.secondary)
                    }
                }
            }
            if showsChart {
                Section {
                    NavigationLink("Historical Values") {
                        List {
                            MetricHistoricalValuesSection(model: model, records: records, points: points)
                        }
                        .mobileNavigationTitle("Historical Values")
                        .navigationBarTitleDisplayMode(.inline)
                    }
                }
            } else if !records.isEmpty || !points.isEmpty {
                MetricHistoricalValuesSection(model: model, records: records, points: points, incompatibleUnits: incompatibleUnits)
            }
        }
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button("Edit") { editing = true }
            }
        }
        .sheet(isPresented: $editing) {
            NavigationStack {
                EditLabMetricView(model: model, metricID: metric["metric_id"].stringValue, initialName: metricName)
            }
        }
        .initialTask(id: model.reportDataRevision) { await reload() }
        .onChange(of: model.sourcePreferences["preferences"]["reference_profile"]) { _, _ in Task { await reload() } }
        .refreshable { await reload() }
    }

    private func reload() async {
        guard let token = model.session?.token else { return }
        let revision = model.reportDataRevision
        loading = true
        failure = nil
        defer { if model.reportDataRevision == revision { loading = false } }
        do {
            let result = try await model.request("metrics/history/get", body: .object(["metric_id": metric["metric_id"]]))
            guard !Task.isCancelled, model.session?.token == token, model.reportDataRevision == revision else { return }
            history = result
            historyRevision = revision
        } catch {
            guard !Task.isCancelled, model.session?.token == token, model.reportDataRevision == revision else { return }
            failure = error.localizedDescription
        }
    }
}

struct EditLabMetricView: View {
    var model: AppModel
    let metricID: String
    let initialName: String
    @Environment(\.dismiss) var dismiss
    @State var name = ""
    @State var saving = false
    @State var failure: String?
    @State var discard = false

    private var enteredName: String { name.trimmingCharacters(in: .whitespacesAndNewlines) }

    var body: some View {
        Form {
            TextField("Metric name", text: $name)
            if let failure { Text(failure).foregroundStyle(.red) }
        }
        .mobileNavigationTitle("Edit metric")
        .onAppear { name = initialName }
        .interactiveDismissDisabled(name != initialName || saving)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) {
                Button("Cancel") {
                    if name == initialName { dismiss() } else { discard = true }
                }.disabled(saving)
            }
            ToolbarItem(placement: .confirmationAction) {
                Button("Save") { Task { await save() } }
                    .disabled(saving || enteredName.isEmpty || enteredName == initialName || enteredName.utf8.count > 256)
            }
        }
        .confirmationDialog("Discard changes?", isPresented: $discard) {
            Button("Discard", role: .destructive) { dismiss() }
        }
    }

    private func save() async {
        saving = true
        failure = nil
        defer { saving = false }
        do {
            try await model.renameLabMetric(metricID, name: enteredName)
            dismiss()
        } catch is CancellationError {
            return
        } catch {
            failure = error.localizedDescription
        }
    }
}

struct MetricHistoricalValuesSection: View {
    var model: AppModel
    let records: [JSONValue]
    let points: [JSONValue]
    var incompatibleUnits: [String] = []
    @State var preview: DocumentLocation?
    @State var failure: String?

    var body: some View {
        Section("Historical values") {
            if !incompatibleUnits.isEmpty {
                Text("A chart is unavailable because these units cannot be converted to a common unit: \(incompatibleUnits.joined(separator: ", ")).")
                    .font(.caption).foregroundStyle(.secondary)
            }
            if let failure { Text(failure).foregroundStyle(.secondary) }
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
        .originalReportPreview($preview)
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
