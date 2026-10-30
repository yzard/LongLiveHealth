import SwiftUI

struct MetricMeasurementsView: View {
    var model: AppModel
    let kind: String
    let definition: JSONValue
    private var title: String { definition["title"].stringValue.isEmpty ? (Self.kinds.first { $0.0 == kind }?.1 ?? HealthSyncProgress.name(kind)) : definition["title"].stringValue }
    @State var result: JSONValue = .null
    @State var failure: String?
    @State var loading = false
    @State var selectedWindow: ChartWindow?
    static let kinds = [("heart_rate", "Heart rate"), ("blood_glucose", "Blood glucose"), ("vo2_max", "VO₂ Max"), ("body_mass", "Weight"), ("body_fat", "Body fat"), ("oxygen_saturation", "Oxygen saturation"), ("respiratory_rate", "Respiratory rate"), ("hrv_rmssd", "HRV RMSSD"), ("HKQuantityTypeIdentifierHeartRateVariabilityRMSSD", "Recovery HRV"), ("height", "Height"), ("body_temperature", "Body temperature")]
    private var bounds: [Double] { [result["first_at"].numberValue, result["last_at"].numberValue].compactMap { $0 } }
    var body: some View {
        MetricDetailScreen(title: title) {
            if loading { Section { ProgressView("Loading selected period") } }
            if let failure { Section { Text(failure).foregroundStyle(.secondary) } }
            MetricChartSection {
                MetricChart(model: model, ranges: result["chart_ranges"], unit: result["unit"].stringValue,
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
                        MeasurementValuesView(model: model, kind: kind, window: selectedWindow)
                    }
                }
            }
        }
                .task(id: selectedWindow) { if let selectedWindow { await reload(selectedWindow) } }
            .onChange(of: model.entryMutation) { _, _ in
                if kind == "body_mass", let selectedWindow { Task { await reload(selectedWindow) } }
            }
    }
    private func reload(_ window: ChartWindow) async {
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
            ForEach(Array(points.enumerated()), id: \.offset) { _, point in
                VStack(alignment: .leading) {
                    Text(point["value"].numberValue.map { "\($0.formatted()) \(point["unit"].stringValue)" } ?? "\(point["raw_value"].numberValue?.formatted() ?? "Unavailable") \(point["raw_unit"].stringValue)")
                    if let at = point["at"].numberValue { Text(Date(timeIntervalSince1970: at).formatted()).font(.caption) }
                    Text(model.sourceLabel(point["source"].stringValue)).font(.caption)
                }
            }
            if let failure { Text(failure).foregroundStyle(.secondary) }
            if loading { ProgressView() }
            if nextOffset != nil { Button("Load more") { Task { await load() } }.disabled(loading) }
            if !loading && points.isEmpty && failure == nil { Text("No measurements in this period.") }
        }.navigationTitle("Historical Values").navigationBarTitleDisplayMode(.inline)
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
