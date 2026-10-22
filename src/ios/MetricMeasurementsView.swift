import SwiftUI
import Charts

struct MetricMeasurementsView: View {
    var model: AppModel
    let kind: String
    @State private var result: JSONValue = .null
    @State private var failure: String?
    @State private var loading = false
    static let kinds = [("heart_rate", "Heart rate"), ("blood_glucose", "Blood glucose"), ("vo2_max", "VO₂ Max"), ("body_mass", "Weight"), ("body_fat", "Body fat"), ("oxygen_saturation", "Oxygen saturation"), ("respiratory_rate", "Respiratory rate")]
    private var sourceKeys: [String] { Array(Set(sources.map { $0["source"].stringValue })).sorted() }
    private var unit: String { result["unit"].stringValue.isEmpty && kind == "body_mass" ? "kg" : result["unit"].stringValue }
    private var sources: [JSONValue] { result["sources"].arrayValue }
    var body: some View {
        List {
            if loading || failure != nil {
                Section {
                    if loading { ProgressView() }
                    if let failure { Text(failure).foregroundStyle(.secondary) }
                }
            }
            Section {
                MetricChart(model: model, ranges: result["chart_ranges"], unit: unit,
                            divisor: 1, sourceKeys: sourceKeys, names: [:],
                            initialPeriod: kind == "heart_rate" ? .day : .week, showsHour: kind == "heart_rate", heartRate: kind == "heart_rate")

            }.listRowBackground(Color.clear)
            ForEach(Array(sources.enumerated()), id: \.offset) { _, source in
                if source["glucose_summary"] != .null {
                Section(model.sourceLabel(source["source"].stringValue)) {
                    if source["glucose_summary"] != .null {
                        let glucose = source["glucose_summary"]
                        Text("Observed coverage: \(((glucose["coverage_fraction"].numberValue ?? 0) * 100).formatted())%")
                        Text("Time in 70–180 mg/dL: \(glucose["tir_percent"].numberValue?.formatted() ?? "Not available")\(glucose["tir_percent"] == .null ? "" : "%")")
                        Text("Mean: \(glucose["mean_mg_dl"].numberValue?.formatted() ?? "Not available") mg/dL")
                    }

                }
            }
            }
            Section("About this metric") {
                if !loading && failure == nil && sources.isEmpty { Text("No recorded measurements in this window.") }
                if kind == "heart_rate" {
                    Text("Individual heart rate measurements in beats per minute. The chart preserves recorded sample times; it does not create readings between samples.")
                    if let latest = sources.flatMap({ $0["chart_points"].arrayValue.compactMap { $0["at"].numberValue } }).max() {
                        Text("Latest measurement on server: \(Date(timeIntervalSince1970: latest).formatted())").font(.caption)
                        if !Calendar.current.isDateInToday(Date(timeIntervalSince1970: latest)) {
                            Text("No measurements from today have reached the server. Check Apple Health sync in Data.").font(.caption)
                        }
                    }
                    Text("Showing measurements from the last 30 days.").font(.caption)
                }
                else if kind == "blood_glucose" { ForEach(result["notes"].arrayValue.map(\.stringValue), id: \.self) { Text($0).font(.caption) } }
                if kind == "blood_glucose" { Text("Glucose summaries hold each sample until the next sample only when the gap is at most 15 minutes. Long gaps remain missing.").font(.caption) }
                Text(kind == "body_mass" ? "Body weight at the recorded time, in kilograms. Apple Health measurements and manually recorded weights are shown with their sources." : "Recorded measurements are shown in the display unit. Missing intervals remain unknown.")
            }
            Section {
                NavigationLink("Historical Values") {
                    List {
                        ForEach(Array(sources.enumerated()), id: \.offset) { _, source in
                            Section(model.sourceLabel(source["source"].stringValue)) {
                                ForEach(Array(source["points"].arrayValue.sorted { ($0["at"].numberValue ?? 0) > ($1["at"].numberValue ?? 0) }.enumerated()), id: \.offset) { _, point in
                                    VStack(alignment: .leading) {
                                        Text("\(point["value"].numberValue?.formatted() ?? "Not available") \(unit)")
                                        if let at = point["at"].numberValue { Text(Date(timeIntervalSince1970: at).formatted()).font(.caption) }
                                    }
                                }
                            }
                        }
                    }.navigationTitle("Historical Values").navigationBarTitleDisplayMode(.inline)
                }
            }
        }.navigationTitle(Self.kinds.first { $0.0 == kind }?.1 ?? kind)
            .navigationBarTitleDisplayMode(.inline)
            .onChange(of: model.entryMutation) { _, change in
                guard kind == "body_mass", change != nil else { return }
                Task {
                    guard let token = model.session?.token else { return }
                    do {
                        let end = Date().timeIntervalSince1970.rounded(.down)
                        let response = try await model.request("wellness/series", body: .object([
                            "timezone": .string(TimeZone.current.identifier), "record_type": .string(kind), "start_at": .number(end - 30 * 86400),
                            "end_at": .number(end), "maximum_gap_seconds": .number(900), "declared_maximum": .null
                        ]))
                        guard !Task.isCancelled, model.session?.token == token else { return }
                        result = response
                    } catch { failure = error.localizedDescription }
                }
            }
            .initialTask(id: kind) { await reload() }.refreshable { await reload() }
    }
    private func reload() async {
        guard model.client != nil, let token = model.session?.token else { return }
        let selected = kind
        loading = true
        result = .null
        failure = nil
        defer { if selected == kind { loading = false } }
        let end = Date().timeIntervalSince1970.rounded(.down)
        do {
            // VO₂ Max is sparse: fetch its bounded complete history in one request.
            let start = selected == "vo2_max" ? -62_135_596_800.0 : end - 30 * 86400
            let response = try await model.request("wellness/series", body: .object([
                "timezone": .string(TimeZone.current.identifier), "record_type": .string(selected), "start_at": .number(start), "end_at": .number(end), "maximum_gap_seconds": .number(selected == "heart_rate" ? 15 : 900), "declared_maximum": .null
            ]))
            guard !Task.isCancelled, model.session?.token == token, selected == kind else { return }
            result = response

        } catch {
            guard !Task.isCancelled, model.session?.token == token, selected == kind else { return }
            failure = error.localizedDescription
        }
    }
}
