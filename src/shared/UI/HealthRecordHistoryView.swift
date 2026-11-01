import SwiftUI

/// Shared iOS/Android entry point for events and non-scalar health records.
struct HealthRecordHistoryView: View {
    var model: AppModel
    let definition: JSONValue
    let historicalOnly: Bool
    @State var records: [JSONValue] = []
    @State var latestDetail: JSONValue = .null
    @State var cursor: JSONValue = .null
    @State var loading = false
    @State var failure: String?
    var body: some View {
        MetricDetailScreen(title: historicalOnly ? "Historical Values" : definition["title"].stringValue) {
            if !historicalOnly && definition["family"] == .string("characteristic") {
                Section {
                    Text("Latest value").font(.subheadline).foregroundStyle(.secondary)
                    if loading { ProgressView("Loading latest value") }
                    else {
                        Text(CharacteristicValue.text(kind: definition["kind"].stringValue, details: latestDetail["details"]))
                            .font(.largeTitle.bold()).accessibilityIdentifier("latest-characteristic-value")
                    }
                }
            }
            if !historicalOnly { Section("About this metric") { Text(definition["description"].stringValue) } }
            Section("Historical Values") {
                if let failure { Text(failure).foregroundStyle(.secondary) }
                ForEach(Array(HistoricalValue.newestFirst(records, timestamps: ["at"], dates: []).enumerated()), id: \.offset) { _, item in
                    NavigationLink { SpecialtyRecordView(model: model, reference: item) } label: {
                        HistoricalValueRow(timestamp: item["at"].numberValue, dateText: "",
                            source: model.sourceLabel(SourceNames.key(item)), value: HistoricalValue.record(item))
                    }
                }
                if loading { ProgressView("Loading records") }
                if !loading && records.isEmpty && failure == nil { Text("No recorded history.") }
                if cursor != .null { Button("Load more") { Task { await load() } }.disabled(loading) }
            }
        }.initialTask { await load() }
    }
    private func load() async {
        guard !loading, let token = model.session?.token else { return }
        loading = true; failure = nil
        defer { loading = false }
        do {
            let result = try await model.request("wellness/records/list", body: .object(["kind": definition["kind"], "after": cursor]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            records += result["records"].arrayValue; cursor = result["next_cursor"]
            if definition["family"] == .string("characteristic") {
                // Read all metadata snapshots so the historical list is complete across devices.
                while cursor != .null {
                    let page = try await model.request("wellness/records/list", body: .object(["kind": definition["kind"], "after": cursor]))
                    guard !Task.isCancelled, model.session?.token == token else { return }
                    records += page["records"].arrayValue; cursor = page["next_cursor"]
                }
                if let latest = CharacteristicValue.latest(records) {
                    let detail = try await model.request("wellness/records/get", body: .object([
                        "platform": latest["platform"], "source_id": latest["source_id"],
                        "record_id": latest["record_id"], "sample_offset": .number(0)
                    ]))
                    guard !Task.isCancelled, model.session?.token == token else { return }
                    latestDetail = detail
                }
            }
        } catch { if !Task.isCancelled, model.session?.token == token { failure = error.localizedDescription } }
    }
}
