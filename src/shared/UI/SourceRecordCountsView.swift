import SwiftUI

struct SourceRecordCountsView: View {
    var model: AppModel
    @State var sources: [JSONValue] = []
    @State var loading = true
    @State var failure: String?

    var body: some View {
        List {
            if loading { ProgressView(uiText("Loading sources")) }
            if let failure {
                Text(uiMessage(failure)).foregroundStyle(.secondary)
                Button(uiText("Retry")) { Task { await reload() } }
            } else if !loading && sources.isEmpty {
                Text(uiText("No visible records. Sync to start archiving data.")).foregroundStyle(.secondary)
            }
            ForEach(Array(sources.enumerated()), id: \.offset) { _, source in
                VStack(alignment: .leading, spacing: 6) {
                    Text(model.sourceLabel(SourceNames.key(source)))
                    Text("\(source["platform"].stringValue) · \(HealthSyncProgress.name(source["record_type"].stringValue))").font(.caption)
                    Text(uiText("\(Int(source["record_count"].numberValue ?? 0).formatted()) records")).font(.caption)
                }
            }
        }.mobileNavigationTitle(uiText("Sources Record Count")).navigationBarTitleDisplayMode(.inline)
            .initialTask { await reload() }.refreshable { await reload() }
    }

    private func reload() async {
        guard model.client != nil, let token = model.session?.token else { loading = false; return }
        loading = true
        failure = nil
        defer { loading = false }
        do {
            let result = try await model.request("wellness/sources", body: .object([:]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            sources = result["sources"].arrayValue
        } catch {
            if !Task.isCancelled, model.session?.token == token { failure = error.localizedDescription }
        }
    }
}
