import SwiftUI
#if canImport(Charts)
import Charts
#endif

struct SpecialtyRecordsView: View {
    var model: AppModel
    @State var kind = "ecg"
    @State var records: [JSONValue] = []
    @State var cursor: JSONValue = .null
    @State var failure: String?
    @State var loading = false
    var body: some View {
        VStack(spacing: 0) {
            Picker("Record type", selection: $kind) { Text("ECG").tag("ecg"); Text("Clinical and medication records").tag("clinical"); Text("Blood Pressure").tag("blood_pressure") }
            if kind == "blood_pressure" {
                CatalogMetricView(model: model, kind: "HKCorrelationTypeIdentifierBloodPressure")
            } else if kind == "ecg" {
                ECGRecordsView(model: model)
            } else {
            List {
            InfoHelp(title: "Record interpretation", message: "Source measurements and classifications. No new diagnostic classification is generated.")
            if let failure { Text(failure).foregroundStyle(.secondary) }
            if records.isEmpty && !loading { Text("No available records of this type.") }
            ForEach(Array(HistoricalValue.newestFirst(records, timestamps: ["at", "start_at"], dates: []).enumerated()), id: \.offset) { _, item in
                NavigationLink { SpecialtyRecordView(model: model, reference: item) } label: {
                    HistoricalValueRow(timestamp: item["at"].numberValue, dateText: "",
                        source: model.sourceLabel(SourceNames.key(item)), value: HistoricalValue.record(item))
                }
            }
            if cursor != .null { Button("More records") { Task { await load(reset: false) } }.disabled(loading) }
            }
            }
        }.mobileNavigationTitle("Specialty records").initialTask(id: kind) { if kind == "clinical" { await load(reset: true) } }
    }
    private func load(reset: Bool) async {
        guard let client = model.client, let token = model.session?.token else { return }
        loading = true; defer { loading = false }
        if reset { records = []; cursor = .null }
        do {
            let response = try await model.request("wellness/records/list", body: .object(["kind": .string(kind), "after": cursor]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            records += response["records"].arrayValue; cursor = response["next_cursor"]; failure = nil
        } catch { if !Task.isCancelled, model.session?.token == token { failure = error.localizedDescription } }
    }
}

struct SpecialtyRecordView: View {
    var model: AppModel
    let reference: JSONValue
    var title = "Source record"
    @State var result: JSONValue = .null
    @State var offset = 0
    @State var ecgTrace = ECGWaveform.Trace(frequency: nil)
    @State var loading = false
    @State var failure: String?
    var body: some View {
        List {
            if let failure { Text(failure).foregroundStyle(.secondary) }
            if result["record_type"] == .string("electrocardiogram") {
                Section {
                    ECGWaveformView(trace: ecgTrace).frame(height: 240)
                    if loading { ProgressView("Loading ECG waveform") }
                }
                Section("Record Details") {
                    Text("Collected: \(HistoricalValue.date(timestamp: result["start_at"].numberValue, text: ""))")
                    Text("Classification: \(ECGRecordInfo.classification(result["source_classification"])) · Symptoms: \(ECGRecordInfo.symptoms(result["details"]["ecg_symptoms_status"]))")
                    HStack {
                        if let frequency = result["sampling_frequency_hz"].numberValue { Text("Sampling: \(frequency.formatted()) Hz") }
                        if let rate = result["details"]["average_heart_rate"].numberValue { Text("· Heart rate: \(rate.formatted()) BPM") }
                    }
                    Text("Source: \(model.sourceLabel(SourceNames.key(reference)))").foregroundStyle(.secondary)
                }
            } else {
                Section("Source") {
                    Text(model.sourceLabel(SourceNames.key(reference)))
                    if let at = result["start_at"].numberValue { Text(Date(timeIntervalSince1970: at).formatted()).font(.caption) }
                }
                if !result["details"].objectValue.isEmpty {
                    Section("Recorded details") {
                        ForEach(result["details"].objectValue.keys.sorted(), id: \.self) { key in
                            VStack(alignment: .leading) {
                                Text(HealthSyncProgress.name(key)).font(.caption).foregroundStyle(.secondary)
                                CopyableText(result["details"][key].formattedJSON)
                            }
                        }
                    }
                }
            }
            if result["record_type"] == .string("workout_route") {
                Section("Recorded route") {
                    RouteTraceView(samples: result["samples"].arrayValue).frame(height: 260)
                    Text("Positions \(offset)–\(offset + result["samples"].arrayValue.count) of \(Int(result["sample_count"].numberValue ?? 0))").font(.caption)
                    if offset > 0 { Button("Previous positions") { offset = max(0, offset - 5000) } }
                    if let next = result["next_sample_offset"].numberValue { Button("Next positions") { offset = Int(next) } }
                }
            }
            if result["fhir"]["resource"] != .null {
                Section("Clinical resource") {
                    Text(result["fhir"]["resource"]["resourceType"].stringValue).font(.headline)
                    ForEach(result["fhir"]["resource"].objectValue.keys.sorted(), id: \.self) { key in
                        DisclosureGroup(key) { CopyableText(result["fhir"]["resource"][key].formattedJSON).font(.system(.caption, design: .monospaced)) }
                    }
                    ShareLink("Share source resource", item: result["fhir"]["resource"].formattedJSON)
                }
            }
            ForEach(Array(result["related"].arrayValue.enumerated()), id: \.offset) { _, relation in
                Section("Related source measurement") {
                    Text(relation["state"].stringValue)
                    ForEach(Array(relation["records"].arrayValue.enumerated()), id: \.offset) { _, record in
                        Text(record["record_type"].stringValue + " · " + (record["value"].numberValue?.formatted() ?? "Missing") + " " + record["unit"].stringValue)
                    }
                }
            }
            InfoHelp(title: "Source record", message: result["notes"].arrayValue.map(\.stringValue).joined(separator: "\n"))
        }.mobileNavigationTitle(title).initialTask(id: offset) { await load() }.refreshable { await load() }
    }
    private func load() async {
        guard !loading, let token = model.session?.token else { return }
        loading = true
        defer { loading = false }
        do {
            let response = try await recordPage(offset: offset)
            guard !Task.isCancelled, model.session?.token == token else { return }
            result = response; failure = nil
            if response["record_type"] == .string("electrocardiogram") {
                var trace = ECGWaveform.Trace(frequency: response["sampling_frequency_hz"].numberValue)
                trace.append(response["samples"].arrayValue)
                result["samples"] = .array([])
                ecgTrace = trace
                var page = response
                var previous = offset
                while let next = page["next_sample_offset"].numberValue {
                    guard next.isFinite, next > Double(previous), next.rounded(.towardZero) == next, next < Double(Int.max) else { throw APIError.invalidResponse }
                    let position = Int(next)
                    page = try await recordPage(offset: position)
                    guard !Task.isCancelled, model.session?.token == token else { return }
                    guard page["record_type"] == response["record_type"], page["version"] == response["version"], !page["samples"].arrayValue.isEmpty else { throw APIError.invalidResponse }
                    trace.append(page["samples"].arrayValue)
                    ecgTrace = trace; previous = position
                }
            }
        } catch { if !Task.isCancelled, model.session?.token == token { failure = error.localizedDescription } }
    }
    private func recordPage(offset: Int) async throws -> JSONValue {
        try await model.request("wellness/records/get", body: .object(["platform": reference["platform"], "source_id": reference["source_id"], "record_id": reference["record_id"], "sample_offset": .number(Double(offset))]))
    }
}
