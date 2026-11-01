import SwiftUI

struct SettingsView: View {
    var model: AppModel
    @State var changingPassword = false
    @State var managingUsers = false
    @State var sleepGoalVisible = false
    @State var referenceSetting: ReferenceSetting?
    @State var deleting = false
    @State var confirmation = ""
    var body: some View {
        List {
            Section("Account") {
                LabeledContent("Server URL") { CopyableText(model.session?.server.absoluteString ?? "") }
                LabeledContent("Username", value: model.session?.username ?? "")
                Button("Change Password") { changingPassword = true }
                if model.session?.username == "admin" { Button("Manage Users") { managingUsers = true } }
                Button("Sign Out and Clear This Device") { Task { await model.logout() } }.disabled(model.isBusy || model.isHealthSyncRunning)
                if model.isHealthSyncRunning { Text("Pause \(MobilePlatform.healthProviderName) sync before signing out or deleting your account.").font(.caption) }
                if model.session?.username != "admin" { Button("Delete account and all server data", role: .destructive) { deleting = true }.disabled(model.isBusy || model.isHealthSyncRunning) }
            }
            Section("Security") {
                Toggle(MobilePlatform.biometricLabel, isOn: Binding(get: { model.biometricUnlock }, set: { enabled in Task { await model.setBiometricUnlock(enabled) } }))
                    .disabled(model.isAuthenticating)
            }
            Section("Reference Settings") {
                Button { referenceSetting = .biologicalSex } label: {
                    let sex = model.sourcePreferences["preferences"]["reference_profile"]["reference_sex"].stringValue
                    LabeledContent("Biological Sex", value: sex.isEmpty ? "Not specified" : sex.capitalized)
                }.accessibilityIdentifier("reference-biological-sex")
                Button { referenceSetting = .birthDate } label: {
                    LabeledContent("Date of Birth", value: model.sourcePreferences["preferences"]["reference_profile"]["birth_date"].stringValue.isEmpty ? "Not set" : model.sourcePreferences["preferences"]["reference_profile"]["birth_date"].stringValue)
                }
                .accessibilityIdentifier("reference-birth-date")
                Button { sleepGoalVisible = true } label: {
                    HStack {
                        Text("Sleep Goal")
                        Spacer()
                        if let minutes = model.sourcePreferences["preferences"]["sleep_target_minutes"].numberValue {
                            Text("\(Int(minutes) / 60) hr \(Int(minutes) % 60) min").foregroundStyle(.secondary)
                        } else { Text("Not set").foregroundStyle(.secondary) }
                    }
                }.accessibilityIdentifier("sleep-goal")
            }


        }.mobileNavigationTitle("Settings")
            .sheet(item: $referenceSetting) { setting in NavigationStack { ReferenceSettingEditor(model: model, setting: setting) } }
            .sheet(isPresented: $sleepGoalVisible) { NavigationStack { SleepGoalEditor(model: model) } }
            .sheet(isPresented: $changingPassword) { NavigationStack { PasswordChangeView(model: model, required: false) } }
            .sheet(isPresented: $managingUsers) { NavigationStack { UserManagementView(model: model) } }
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
}
