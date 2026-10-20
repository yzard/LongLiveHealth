import SwiftUI

struct SourceAliasesView: View {
    var model: AppModel
    @State private var editing: SourceAliasSelection?
    @State private var loading = false
    @State private var failure: String?
    private var keys: [String] {
        Set(model.sourceCatalog.map { $0["source_key"].stringValue })
            .union(model.sourcePreferences["preferences"]["source_aliases"].objectValue.keys).sorted()
    }
    var body: some View {
        List {
            Section {
                Text("Give each source a name to use in charts and history. Original source identifiers stay unchanged.").font(.caption)
                if loading { ProgressView("Loading sources") }
                if let failure { Text(failure).foregroundStyle(.secondary) }
                if !loading && keys.isEmpty { Text("Import or sync data to add sources.") }
            }
            Section("Sources") {
                ForEach(keys, id: \.self) { key in
                    Button { editing = SourceAliasSelection(id: key) } label: {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(model.sourceLabel(key)).foregroundStyle(.primary)
                            Text(key).font(.caption).foregroundStyle(.secondary)
                        }
                    }.accessibilityIdentifier("sourceAliasRow-" + key)
                }
            }
        }
        .navigationTitle("Source aliases")
        .initialTask { await load() }
        .refreshable { await load() }
        .sheet(item: $editing) { source in NavigationStack { SourceAliasEditor(model: model, key: source.id) } }
    }
    private func load() async {
        loading = true; failure = nil
        defer { loading = false }
        do {
            _ = try await model.request("wellness/preferences/get", body: .object([:]))
            _ = try await model.request("wellness/sources", body: .object([:]))
        } catch { if !Task.isCancelled { failure = error.localizedDescription } }
    }
}
private struct SourceAliasSelection: Identifiable { let id: String }

private struct SourceAliasEditor: View {
    var model: AppModel
    let key: String
    @Environment(\.dismiss) private var dismiss
    @State private var alias = ""
    @State private var initialAlias = ""
    @State private var snapshot: JSONValue = .null
    @State private var saving = false
    @State private var failure: String?
    @State private var discard = false
    var body: some View {
        Form {
            Section("Alias") {
                TextField("Name", text: $alias).textInputAutocapitalization(.words)
                    .accessibilityIdentifier("sourceAliasName")
                Text("Leave empty to use the original name. Up to 100 characters.").font(.caption)
            }
            Section("Original source") { Text(key).font(.caption).textSelection(.enabled) }
            if let failure { Section { Text(failure); Button("Reload current settings") { Task { await load(replaceDraft: false) } } } }
        }
        .navigationTitle("Edit source alias")
        .interactiveDismissDisabled(alias != initialAlias || saving)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) { Button("Cancel") { if alias != initialAlias { discard = true } else { dismiss() } }.disabled(saving) }
            ToolbarItem(placement: .confirmationAction) { Button("Save") { Task { await save() } }.disabled(saving || snapshot == .null || alias.count > 100) }
        }
        .confirmationDialog("Discard changes?", isPresented: $discard) { Button("Discard", role: .destructive) { dismiss() } }
        .initialTask { await load(replaceDraft: true) }
    }
    private func load(replaceDraft: Bool) async {
        saving = true
        defer { saving = false }
        do {
            snapshot = try await model.request("wellness/preferences/get", body: .object([:]))
            initialAlias = snapshot["preferences"]["source_aliases"][key].stringValue
            if replaceDraft { alias = initialAlias }
            failure = nil
        } catch { if !Task.isCancelled { failure = error.localizedDescription } }
    }
    private func save() async {
        let name = alias.trimmingCharacters(in: .whitespacesAndNewlines)
        var preferences = snapshot["preferences"]
        var aliases = preferences["source_aliases"].objectValue
        if name.isEmpty { aliases.removeValue(forKey: key) } else { aliases[key] = .string(name) }
        preferences["source_aliases"] = .object(aliases)
        saving = true
        defer { saving = false }
        do {
            _ = try await model.request("wellness/preferences/save", body: .object(["expected_version": snapshot["version"], "batch_id": .string(UUID().uuidString), "preferences": preferences]))
            dismiss()
        } catch { if !Task.isCancelled { failure = error.localizedDescription } }
    }
}

struct SourceLegend: View {
    let model: AppModel
    let keys: [String]
    let names: [String: String]
    static let colors: [Color] = [.blue, .orange, .green, .purple, .pink, .teal, .brown, .indigo]
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            ForEach(Array(keys.enumerated()), id: \.element) { index, key in
                HStack {
                    Circle().fill(Self.colors[index % Self.colors.count]).frame(width: 8, height: 8)
                    Text(model.sourceLabel(key, name: names[key] ?? key)).font(.caption)
                }
            }
        }
    }
}
