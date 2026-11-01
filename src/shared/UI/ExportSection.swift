import SwiftUI

/// The single export entry point lives at the bottom of Data on both platforms.
struct ExportSection: View {
    var model: AppModel
    @State var exports: [JSONValue] = []
    @State var share: ShareLocation?
    struct ShareLocation: Identifiable { let id = UUID(); let url: URL }
    var body: some View {
            Section("Export") {
                Button("Create ZIP export") { Task { await action("exports/create", body: .object([:])); await reload() } }.disabled(model.isBusy)
                ForEach(exports, id: \.identifier) { item in
                    VStack(alignment: .leading) {
                        Text("\(item["status"].stringValue.capitalized) · \(Date(timeIntervalSince1970: item["created_at"].numberValue ?? 0).formatted())")
                        if item["status"] == .string("ready") { Button("Download and share") { Task { await download(item["export_id"].stringValue) } }.disabled(model.isBusy) }
                        Button("Delete export", role: .destructive) { Task { await action("exports/delete", body: .object(["export_id": item["export_id"]])); try? model.archive?.remove(name: "export-\(item["export_id"].stringValue).zip"); await reload() } }
                    }.buttonStyle(.borderless)
                }
            }
            .initialTask { await reload() }
            .sheet(item: $share, onDismiss: { clearExportDownloads() }) { item in NavigationStack { VStack(spacing: 20) { Text("Your complete archive is ready"); ShareLink(item: item.url) { Label("Save or share ZIP", systemImage: MobileSymbol.name("square.and.arrow.up")) } }.padding().toolbar { Button("Done") { share = nil } } } }
    }
    private func reload() async { do { if let client = model.client { exports = try await model.request("exports/list", body: .object([:]))["exports"].arrayValue } } catch { model.errorMessage = error.localizedDescription } }
    private func action(_ path: String, body: JSONValue) async { await model.perform { if let client = model.client { _ = try await model.request(path, body: body) } } }
    private func download(_ id: String) async { await model.perform {
        guard let client = model.client, let archive = model.archive else { return }
        let url = try archive.location("export-\(id).zip")
        try await client.download("exports/\(id)/download", destination: url)
        share = ShareLocation(url: url)
    } }
    private func clearExportDownloads() { for item in exports { try? model.archive?.remove(name: "export-\(item["export_id"].stringValue).zip") } }
}
