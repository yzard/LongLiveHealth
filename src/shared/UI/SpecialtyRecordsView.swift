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
        List {
            Picker("Record type", selection: $kind) { Text("ECG").tag("ecg"); Text("Clinical and medication records").tag("clinical"); Text("Paired blood pressure").tag("blood_pressure") }
            InfoHelp(title: "Record interpretation", message: "Source measurements and classifications. No new diagnostic classification is generated.")
            if let failure { Text(failure).foregroundStyle(.secondary) }
            if records.isEmpty && !loading { Text("No available records of this type.") }
            ForEach(Array(records.enumerated()), id: \.offset) { _, item in
                NavigationLink { SpecialtyRecordView(model: model, reference: item) } label: {
                    VStack(alignment: .leading) {
                        Text(item["record_type"].stringValue)
                        Text(Date(timeIntervalSince1970: item["at"].numberValue ?? 0).formatted()).font(.caption)
                        Text(model.sourceLabel(SourceNames.key(item))).font(.caption)
                    }
                }
            }
            if cursor != .null { Button("More records") { Task { await load(reset: false) } }.disabled(loading) }
        }.navigationTitle("Specialty records").initialTask(id: kind) { await load(reset: true) }
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
    @State var result: JSONValue = .null
    @State var offset = 0
    @State var failure: String?
    var body: some View {
        List {
            Section("Source") {
                Text(model.sourceLabel(SourceNames.key(reference)))
                if let at = result["start_at"].numberValue { Text(Date(timeIntervalSince1970: at).formatted()).font(.caption) }
                if let classification = result["source_classification"].numberValue { Text("Source classification: \(classification.formatted())") }
                if let failure { Text(failure).foregroundStyle(.secondary) }
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
            if result["record_type"] == .string("electrocardiogram") {
                Section("Source waveform") {
                    #if os(Android)
                    AndroidPointPlot(points: AndroidPlotPoint.values(result["samples"].arrayValue, x: "time_since_start", y: "lead_i_volts", multiplier: 1000), unit: "mV", select: nil, connected: true, events: []).frame(height: 220)
                    #else
                    Chart {
                        ForEach(Array(result["samples"].arrayValue.enumerated()), id: \.offset) { _, point in
                            if let time = point["time_since_start"].numberValue, let voltage = point["lead_i_volts"].numberValue {
                                PointMark(x: .value("Seconds", time), y: .value("mV", voltage * 1000)).symbolSize(2)
                            }
                        }
                    }.frame(height: 220).accessibilityLabel("Source ECG samples in millivolts. Missing samples are not connected.")
                    #endif
                    Text("Samples \(offset)–\(offset + result["samples"].arrayValue.count) of \(Int(result["sample_count"].numberValue ?? 0))").font(.caption)
                    if offset > 0 { Button("Previous samples") { offset = max(0, offset - 5000) } }
                    if let next = result["next_sample_offset"].numberValue { Button("Next samples") { offset = Int(next) } }
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
        }.navigationTitle("Source record").initialTask(id: offset) { await load() }
    }
    private func load() async {
        guard let client = model.client, let token = model.session?.token else { return }
        do {
            let response = try await model.request("wellness/records/get", body: .object(["platform": reference["platform"], "source_id": reference["source_id"], "record_id": reference["record_id"], "sample_offset": .number(Double(offset))]))
            guard !Task.isCancelled, model.session?.token == token else { return }; result = response; failure = nil
        } catch { if !Task.isCancelled, model.session?.token == token { failure = error.localizedDescription } }
    }
}
