import SwiftUI

struct SettingsView: View {
    var model: AppModel
    @State var exports: [JSONValue] = []
    @State var share: ShareLocation?
    @State var changingPassword = false
    @State var managingUsers = false
    @State var deleting = false
    @State var confirmation = ""
    struct ShareLocation: Identifiable { let id = UUID(); let url: URL }
    var body: some View {
        List {
            Section("Your server") {
                CopyableText(model.session?.server.absoluteString ?? "")
                Text(model.session?.username ?? "")
                InfoHelp(title: "Server processing", message: "OCR and analysis use the providers chosen in your server's TOML configuration. This app does not route your data through an official Long Live Health server.")
            }
            Section("Account") {
                Toggle(MobilePlatform.biometricLabel, isOn: Binding(get: { model.biometricUnlock }, set: { enabled in Task { await model.setBiometricUnlock(enabled) } }))
                    .disabled(model.isAuthenticating)
                InfoHelp(title: "App unlock", message: "Stay signed in securely. Use biometrics or your device passcode to unlock the app; background imports can continue.")
                Button("Change password") { changingPassword = true }
                if model.session?.username == "admin" { Button("Manage users") { managingUsers = true } }
            }
            Section("Full data export") {
                InfoHelp(title: "Export contents", message: "Includes original files, structured data and revision history. Export files contain sensitive health data.")
                Button("Create ZIP export") { Task { await action("exports/create", body: .object([:])); await reload() } }.disabled(model.isBusy)
                ForEach(exports, id: \.identifier) { item in
                    VStack(alignment: .leading) {
                        Text("\(item["status"].stringValue.capitalized) · \(Date(timeIntervalSince1970: item["created_at"].numberValue ?? 0).formatted())")
                        if item["status"] == .string("ready") { Button("Download and share") { Task { await download(item["export_id"].stringValue) } }.disabled(model.isBusy) }
                        Button("Delete export", role: .destructive) { Task { await action("exports/delete", body: .object(["export_id": item["export_id"]])); try? model.archive?.remove(name: "export-\(item["export_id"].stringValue).zip"); await reload() } }
                    }.buttonStyle(.borderless)
                }
            }
            Section {
                Button("Sign out and clear this device") { Task { await model.logout() } }.disabled(model.isBusy || model.isHealthSyncRunning)
                if model.isHealthSyncRunning { Text("Pause \(MobilePlatform.healthProviderName) sync before signing out or deleting your account.").font(.caption) }
                InfoHelp(title: "Clear this device", message: "This also removes cached reports, pending uploads and local sync checkpoints.")
                if model.session?.username != "admin" { Button("Delete account and all server data", role: .destructive) { deleting = true }.disabled(model.isBusy || model.isHealthSyncRunning) }
            }
        }.navigationTitle("Settings").initialTask { await reload() }.refreshable { await reload() }
            .sheet(isPresented: $changingPassword) { NavigationStack { PasswordChangeView(model: model, required: false) } }
            .sheet(isPresented: $managingUsers) { NavigationStack { UserManagementView(model: model) } }
            .sheet(item: $share, onDismiss: { clearExportDownloads() }) { item in NavigationStack { VStack(spacing: 20) { Text("Your complete archive is ready"); ShareLink(item: item.url) { Label("Save or share ZIP", systemImage: MobileSymbol.name("square.and.arrow.up")) } }.padding().toolbar { Button("Done") { share = nil } } } }
            .alert("Delete your account?", isPresented: $deleting) {
                TextField("Type your username", text: $confirmation)
                Button("Cancel", role: .cancel) { confirmation = "" }
                Button("Delete", role: .destructive) { Task { await model.perform {
                    guard let client = model.client else { return }
                    _ = try await model.request("user/delete", body: .object(["confirmation": .string(confirmation)]))
                    try model.clearLocalSession()
                }; confirmation = "" } }
            } message: { Text("Type \(model.session?.username ?? "") to delete all server records and revoke every session. Stored files are then removed by the server. Your separate backups remain your responsibility.") }
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
