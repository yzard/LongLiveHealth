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
            Section(uiText("Account")) {
                LabeledContent(uiText("Server URL")) { CopyableText(model.session?.server.absoluteString ?? "") }
                LabeledContent(uiText("Username"), value: model.session?.username ?? "")
                Button(uiText("Change Password")) { changingPassword = true }
                if model.session?.username == "admin" { Button(uiText("Manage Users")) { managingUsers = true } }
                Button(uiText("Sign Out and Clear This Device")) { Task { await model.logout() } }.disabled(model.isBusy || model.isHealthSyncRunning)
                if model.isHealthSyncRunning { Text(uiText("Pause \(MobilePlatform.healthProviderName) sync before signing out or deleting your account.")).font(.caption) }
                if model.session?.username != "admin" { Button(uiText("Delete account and all server data"), role: .destructive) { deleting = true }.disabled(model.isBusy || model.isHealthSyncRunning) }
            }
            Section(uiText("Security")) {
                Toggle(uiLabel(MobilePlatform.biometricLabel), isOn: Binding(get: { model.biometricUnlock }, set: { enabled in Task { await model.setBiometricUnlock(enabled) } }))
                    .disabled(model.isAuthenticating)
            }
            Section {
                Picker(uiText("Language"), selection: Binding(
                    get: { InterfaceLanguageSettings.shared.language },
                    set: { InterfaceLanguageSettings.shared.select($0) }
                )) {
                    ForEach(InterfaceLanguage.allCases, id: \.self) { language in
                        Text(verbatim: language.displayName).tag(language)
                    }
                }
                .pickerStyle(.menu)
                .accessibilityIdentifier("settings-interface-language")
            }
            Section(uiText("Reference Settings")) {
                Button { referenceSetting = .biologicalSex } label: {
                    let sex = model.sourcePreferences["preferences"]["reference_profile"]["reference_sex"].stringValue
                    LabeledContent(uiText("Biological Sex"), value: sex.isEmpty ? uiText("Not specified") : uiLabel(sex.capitalized))
                }.accessibilityIdentifier("reference-biological-sex")
                Button { referenceSetting = .birthDate } label: {
                    LabeledContent(uiText("Date of Birth"), value: model.sourcePreferences["preferences"]["reference_profile"]["birth_date"].stringValue.isEmpty ? uiText("Not set") : model.sourcePreferences["preferences"]["reference_profile"]["birth_date"].stringValue)
                }
                .accessibilityIdentifier("reference-birth-date")
                Button { sleepGoalVisible = true } label: {
                    HStack {
                        Text(uiText("Sleep Goal"))
                        Spacer()
                        if let minutes = model.sourcePreferences["preferences"]["sleep_target_minutes"].numberValue {
                            Text(uiText("\(Int(minutes) / 60) hr \(Int(minutes) % 60) min")).foregroundStyle(.secondary)
                        } else { Text(uiText("Not set")).foregroundStyle(.secondary) }
                    }
                }.accessibilityIdentifier("sleep-goal")
            }


        }.mobileNavigationTitle(uiText("Settings"))
            .sheet(item: $referenceSetting) { setting in NavigationStack { ReferenceSettingEditor(model: model, setting: setting) } }
            .sheet(isPresented: $sleepGoalVisible) { NavigationStack { SleepGoalEditor(model: model) } }
            .sheet(isPresented: $changingPassword) { NavigationStack { PasswordChangeView(model: model, required: false) } }
            .sheet(isPresented: $managingUsers) { NavigationStack { UserManagementView(model: model) } }
            .alert(uiText("Delete your account?"), isPresented: $deleting) {
                TextField(uiText("Type your username"), text: $confirmation)
                Button(uiText("Cancel"), role: .cancel) { confirmation = "" }
                Button(uiText("Delete"), role: .destructive) { Task { await model.perform {
                    guard let client = model.client else { return }
                    _ = try await model.request("user/delete", body: .object(["confirmation": .string(confirmation)]))
                    try model.clearLocalSession()
                }; confirmation = "" } }
            } message: { Text(uiText("Type \(model.session?.username ?? "") to delete all server records and revoke every session. Stored files are then removed by the server. Your separate backups remain your responsibility.")) }
    }
}
