import SwiftUI

@main
struct LongLiveHealthApp: App {
    @UIApplicationDelegateAdaptor(HealthAppDelegate.self) private var delegate
    private var model: AppModel { delegate.model }
    @Environment(\.scenePhase) private var phase
    var body: some Scene {
        WindowGroup {
            RootView(model: model)
                .tint(.teal)
                .onOpenURL { model.receiveReportURL($0) }
                .task {
                    while !Task.isCancelled {
                        await model.renewSessionIfNeeded()
                        do { try await Task.sleep(for: .seconds(30)) } catch { return }
                    }
                }
                .onChange(of: phase) { _, next in
                    model.privacyCovered = next != .active && model.biometricUnlock
                    if next == .background { model.lockForBackground(); model.suspendForegroundRecovery() }
                    AppLockShield.shared.update(model: model)
                    if next == .active { Task { await model.resumeForeground() } }
                }
        }
    }
}

struct RootView: View {
    @Bindable var model: AppModel
    var body: some View {
        Group {
            if model.session == nil { LoginView(model: model) }
            else if model.mustChangePassword { NavigationStack { PasswordChangeView(model: model, required: true) } }
            else {
                TabView(selection: $model.selectedTab) {
                    NavigationStack { OverviewView(model: model) }.tabItem { Label("Overview", systemImage: "square.grid.2x2") }.tag("Overview")
                    NavigationStack { BrowseView(model: model) }.tabItem { Label("Browse", systemImage: "list.bullet") }.tag("Browse")
                    NavigationStack { TrendsView(model: model) }.tabItem { Label("Trends", systemImage: "chart.xyaxis.line") }.tag("Trends")
                    NavigationStack { AnalysisView(model: model) }.tabItem { Label("Insights", systemImage: "sparkles") }.tag("Insights")
                    NavigationStack(path: $model.dataPath) { DataCenterView(model: model) }.tabItem { Label("Data", systemImage: "externaldrive") }.tag("Data")
                }
                .id(model.session?.userID)
                .initialTask { await model.resumeForeground() }
            }
        }
        .onAppear { AppLockShield.shared.update(model: model) }
        .onChange(of: model.isLocked) { _, _ in AppLockShield.shared.update(model: model) }
        .onChange(of: model.privacyCovered) { _, _ in AppLockShield.shared.update(model: model) }
        .sheet(isPresented: Binding(get: { model.showSharedReportImport && !model.isLocked && !model.privacyCovered }, set: { model.showSharedReportImport = $0 })) { SharedReportImportView(model: model) }
        .safeAreaInset(edge: .bottom) {
            if let notice = model.offlineNotice, model.session != nil {
                Label(notice, systemImage: "wifi.slash").font(.caption).padding(8).frame(maxWidth: .infinity).background(.regularMaterial)
            }
        }
        .overlay(alignment: .top) { if model.isBusy && !model.isHealthSyncRunning {
            ProgressView("Working…").padding(10).background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12)).accessibilityLabel("Processing your request")
        } }
        .alert("Could not complete the action", isPresented: Binding(get: { model.errorMessage != nil }, set: { if !$0 { model.errorMessage = nil } })) {
            Button("OK") { model.errorMessage = nil }
        } message: { Text(model.errorMessage ?? "") }
    }
}

private struct LoginView: View {
    var model: AppModel
    @State private var server = UserDefaults.standard.string(forKey: "lastServer") ?? ""
    @State private var username = ""
    @State private var password = ""
    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Image(systemName: "heart.text.clipboard").font(.system(size: 44)).foregroundStyle(.teal)
                    Text("Your health. Your history.").font(.title2.bold())
                    Text("Connect to your own Long Live Health server.").foregroundStyle(.secondary)
                }
                Section("Server") { TextField("http://192.168.1.20:8080", text: $server).keyboardType(.URL).textInputAutocapitalization(.never).autocorrectionDisabled() }
                Section("Account") {
                    TextField("Username", text: $username).textContentType(.username).textInputAutocapitalization(.never).autocorrectionDisabled()
                    SecureField("Password", text: $password).textContentType(.password)
                }
                Button {
                    Task { await model.login(server: server, username: username, password: password); password = "" }
                } label: { if model.isBusy { ProgressView() } else { Text("Sign in") } }
                .disabled(model.isBusy || server.isEmpty || username.isEmpty || password.isEmpty)
                Section { Text("Accounts are created by your server administrator. Reports and AI processing use the services configured on that server.").font(.footnote).foregroundStyle(.secondary) }
            }.navigationTitle("Long Live Health")
        }
    }
}
