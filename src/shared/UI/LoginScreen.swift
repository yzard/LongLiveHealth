import SwiftUI

struct LoginScreen: View {
    let isBusy: Bool
    let onLogin: @MainActor (String, String, String) async -> Void
    @State var server = UserDefaults.standard.string(forKey: "lastServer") ?? ""
    @State var username = ""
    @State var password = ""
    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Image(systemName: MobileSymbol.name("heart.text.clipboard")).font(.system(size: 44)).foregroundStyle(.teal)
                    Text(uiText("Your health. Your history.")).font(.title2.bold())
                    InfoHelp(title: uiText("Your server"), message: uiText("Connect to your own Long Live Health server."))
                }
                Section(uiText("Server")) { TextField("http://192.168.1.20:8080", text: $server).keyboardType(.URL).textInputAutocapitalization(.never).autocorrectionDisabled() }
                Section(uiText("Account")) {
                    TextField(uiText("Username"), text: $username).textContentType(.username).textInputAutocapitalization(.never).autocorrectionDisabled()
                    SecureField(uiText("Password"), text: $password).textContentType(.password)
                }
                Button {
                    Task { await onLogin(server, username, password); password = "" }
                } label: { if isBusy { ProgressView() } else { Text(uiText("Sign in")) } }
                .disabled(isBusy || server.isEmpty || username.isEmpty || password.isEmpty)
                Section { InfoHelp(title: uiText("Accounts and processing"), message: uiText("Accounts are created by your server administrator. Reports and AI processing use the services configured on that server.")) }
            }.mobileNavigationTitle("Long Live Health")
        }
    }
}
