import SwiftUI

/// Each control commits immediately. Writes are serialized and failures restore
/// the last server value; returning never submits or reloads the parent screen.
struct AutoSavedPreferencesForm<Content: View>: View {
    var model: AppModel
    var title: String
    @ViewBuilder var content: (JSONValue, @escaping ([String: JSONValue]) -> Void) -> Content
    @Environment(\.dismiss) var dismiss
    @State var snapshot: JSONValue = .null
    @State var loading = true
    @State var saving = false
    @State var failure: String?

    var body: some View {
        Form {
            content(snapshot["preferences"], update)
                .disabled(loading || saving || snapshot == .null)
            if saving { ProgressView("Saving…") }
            if let failure {
                Section {
                    Text(failure).foregroundStyle(.red)
                    Button("Reload current settings") { Task { await load() } }.disabled(saving)
                }
            }
        }
        .mobileNavigationTitle(title)
        .interactiveDismissDisabled(saving)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) {
                Button { dismiss() } label: { Label("Back", systemImage: "chevron.left") }
                    .disabled(saving)
            }
        }
        .initialTask { await load() }
    }

    private func load() async {
        guard let token = model.session?.token else { loading = false; return }
        loading = true
        defer { loading = false }
        do {
            let response = try await model.request("wellness/preferences/get", body: .object([:]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            snapshot = response; failure = nil
        } catch { if !Task.isCancelled, model.session?.token == token { failure = error.localizedDescription } }
    }

    private func update(_ changes: [String: JSONValue]) {
        guard !loading, !saving, snapshot != .null, let token = model.session?.token else { return }
        let current = (model.sourcePreferences["version"].numberValue ?? -1) >= (snapshot["version"].numberValue ?? 0) ? model.sourcePreferences : snapshot
        guard changes.contains(where: { current["preferences"][$0.key] != $0.value }) else { return }
        var updated = current["preferences"]
        for (field, value) in changes { updated[field] = value }
        snapshot = current
        snapshot["preferences"] = updated
        saving = true; failure = nil
        Task {
            defer { saving = false }
            do {
                let response = try await model.request("wellness/preferences/save", body: .object([
                    "expected_version": current["version"], "batch_id": .string(UUID().uuidString), "preferences": updated
                ]))
                guard !Task.isCancelled, model.session?.token == token else { return }
                snapshot = response
            } catch {
                guard model.session?.token == token else { return }
                snapshot = current
                failure = "Change was not saved. " + error.localizedDescription
            }
        }
    }
}
