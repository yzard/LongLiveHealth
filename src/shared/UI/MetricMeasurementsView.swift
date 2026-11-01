import SwiftUI

struct MetricMeasurementsView: View {
    var model: AppModel
    let kind: String
    let definition: JSONValue
    private var title: String { definition["title"].stringValue.isEmpty ? HealthSyncProgress.name(kind) : definition["title"].stringValue }
    @State var result: JSONValue = .null
    @State var failure: String?
    @State var loading = false
    @State var selectedWindow: ChartWindow?
    @State var selectedUnit = ""
    @State var loadedWindow: ChartWindow?
    private var units: [String] { CombinedChartData.units(result) }
    private var displayUnit: String { units.contains(selectedUnit) ? selectedUnit : units.first ?? result["unit"].stringValue }
    private var chartRanges: JSONValue { result["components"] == .null ? result["chart_ranges"] : CombinedChartData.ranges(result, unit: displayUnit) }
    private var bounds: [Double] { [result["first_at"].numberValue, result["last_at"].numberValue].compactMap { $0 } }
    var body: some View {
        MetricDetailScreen(title: title) {
            if loading { Section { ProgressView("Loading selected period") } }
            if let failure { Section { Text(failure).foregroundStyle(.secondary) } }
            MetricChartSection {
                if units.count > 1 {
                    Picker("Measurement unit", selection: Binding(get: { displayUnit }, set: { selectedUnit = $0 })) {
                        ForEach(units, id: \.self) { Text($0).tag($0) }
                    }
                }
                MetricChart(model: model, guidance: result["reference_guidance"], ranges: chartRanges, unit: displayUnit,
                    divisor: 1, sourceKeys: result["sources"].arrayValue.map(\.stringValue), names: [:],
                    initialPeriod: kind == "heart_rate" ? .day : .week, showsHour: true, heartRate: kind == "heart_rate",
                    navigationTimestamps: bounds, onWindowChange: { selectedWindow = $0 })
            }
            Section("About this metric") {
                if let explanation = MetricEducation.explanation(id: kind, name: title) { Text(explanation) }
                else { Text(definition["description"].stringValue) }
            }
            Section {
                if !loading && failure == nil && result["valid_samples"].numberValue == 0 { Text("No recorded measurements in this period.") }
                if let excluded = result["excluded_samples"].numberValue, excluded > 0 {
                    Text("\(Int(excluded)) measurements have unsupported values or units. Their originals remain in Historical Values.").font(.caption)
                }
            }
            if let selectedWindow {
                Section {
                    NavigationLink("Historical Values") {
                        if definition["family"] != .string("quantity") {
                            HealthRecordHistoryView(model: model, definition: definition, historicalOnly: true)
                        } else {
                            MeasurementValuesView(model: model, kind: kind, window: selectedWindow)
                        }
                    }
                }
            }
        }
                .task(id: selectedWindow) { if let selectedWindow { await reload(selectedWindow, force: false) } }
            .onChange(of: model.sourcePreferences["preferences"]["reference_profile"]) { _, _ in
                if let selectedWindow { Task { await reload(selectedWindow, force: true) } }
            }
            .onChange(of: model.entryMutation) { _, _ in
                if kind == "body_mass", let selectedWindow { Task { await reload(selectedWindow, force: true) } }
            }
    }
    private func reload(_ window: ChartWindow, force: Bool) async {
        guard force || loadedWindow != window || failure != nil else { return }
        guard let token = model.session?.token else { return }
        loading = true; failure = nil
        defer { if selectedWindow == window { loading = false } }
        do {
            let response = try await model.request("wellness/chart", body: .object([
                "timezone": .string(TimeZone.current.identifier), "record_type": .string(kind),
                "period": .string(window.period.rawValue), "start_at": .number(floor(window.start)),
                "end_at": .number(ceil(window.start + window.length))
            ]))
            guard !Task.isCancelled, model.session?.token == token, selectedWindow == window else { return }
            result = response
            loadedWindow = window
        } catch {
            guard !Task.isCancelled, model.session?.token == token, selectedWindow == window else { return }
            failure = error.localizedDescription
        }
    }
}

struct MeasurementValuesView: View {
    let model: AppModel
    let kind: String
    let window: ChartWindow
    @State var points: [JSONValue] = []
    @State var nextOffset: Double? = 0
    @State var revision: JSONValue = .null
    @State var loading = false
    @State var failure: String?
    var body: some View {
        List {
            ForEach(Array(HistoricalValue.newestFirst(points, timestamps: ["at"], dates: []).enumerated()), id: \.offset) { _, point in
                HistoricalValueRow(timestamp: point["at"].numberValue, dateText: "",
                    source: model.sourceLabel(point["source"].stringValue),
                    value: HistoricalValue.quantity(point["value"] == .null ? point["raw_value"] : point["value"], unit: HistoricalValue.displayUnit(point["unit"].stringValue.isEmpty ? point["raw_unit"].stringValue : point["unit"].stringValue, kind: kind)))
            }
            if let failure { Text(failure).foregroundStyle(.secondary) }
            if loading { ProgressView() }
            if nextOffset != nil { Button("Load more") { Task { await load() } }.disabled(loading) }
            if !loading && points.isEmpty && failure == nil { Text("No measurements in this period.") }
        }.mobileNavigationTitle("Historical Values").navigationBarTitleDisplayMode(.inline)
            .initialTask { await load() }
    }
    private func load() async {
        guard !loading, let offset = nextOffset, let token = model.session?.token else { return }
        loading = true; failure = nil
        defer { loading = false }
        do {
            let response = try await model.request("wellness/measurements", body: .object([
                "record_type": .string(kind), "start_at": .number(floor(window.start)),
                "end_at": .number(ceil(window.start + window.length)), "offset": .number(offset), "expected_revision": revision
            ]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            points += response["points"].arrayValue; nextOffset = response["next_offset"].numberValue; revision = response["data_revision"]
        } catch { if !Task.isCancelled { failure = error.localizedDescription } }
    }
}
