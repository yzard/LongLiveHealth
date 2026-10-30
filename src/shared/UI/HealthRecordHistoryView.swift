import SwiftUI

/// Shared iOS/Android entry point for events and non-scalar health records.
struct HealthRecordHistoryView: View {
    var model: AppModel
    let definition: JSONValue
    @State var records: [JSONValue] = []
    @State var cursor: JSONValue = .null
    @State var loading = false
    @State var failure: String?
    var body: some View {
        MetricDetailScreen(title: definition["title"].stringValue) {
            Section("About this metric") { Text(definition["description"].stringValue) }
            Section("Historical Values") {
                if let failure { Text(failure).foregroundStyle(.secondary) }
                ForEach(Array(records.enumerated()), id: \.offset) { _, item in
                    NavigationLink { SpecialtyRecordView(model: model, reference: item) } label: {
                        VStack(alignment: .leading) {
                            if let at = item["at"].numberValue { Text(Date(timeIntervalSince1970: at).formatted()) }
                            Text(model.sourceLabel(SourceNames.key(item))).font(.caption)
                        }
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
        } catch { if !Task.isCancelled, model.session?.token == token { failure = error.localizedDescription } }
    }
}
