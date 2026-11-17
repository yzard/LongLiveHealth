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
            if model.session == nil { LoginScreen(isBusy: model.isBusy, onLogin: { server, username, password in
                await model.login(server: server, username: username, password: password)
            }) }
            else if model.mustChangePassword { NavigationStack { PasswordChangeView(model: model, required: true) } }
            else {
                TabView(selection: $model.selectedTab) {
                    NavigationStack { InsightsView(model: model) }.tabItem { Label(uiText("Insights"), systemImage: "sparkles") }.tag("Insights")
                    NavigationStack { BrowseView(model: model) }.tabItem { Label(uiText("Browse"), systemImage: "list.bullet") }.tag("Browse")
                    NavigationStack(path: $model.dataPath) { DataCenterView(model: model) }.tabItem { Label(uiText("Data"), systemImage: "externaldrive") }.tag("Data")
                }
                .id(model.session?.userID)
                .initialTask { await model.resumeForeground() }
            }
        }
        .environment(\.locale, InterfaceLanguageSettings.shared.language.locale)
        .onAppear { AppLockShield.shared.update(model: model) }
        .onChange(of: model.isLocked) { _, _ in AppLockShield.shared.update(model: model) }
        .onChange(of: model.privacyCovered) { _, _ in AppLockShield.shared.update(model: model) }
        .sheet(isPresented: Binding(get: { model.showSharedReportImport && !model.isLocked && !model.privacyCovered }, set: { model.showSharedReportImport = $0 })) { SharedReportImportView(model: model) }
        .safeAreaInset(edge: .bottom) {
            if let notice = model.offlineNotice, model.session != nil {
                Label(uiMessage(notice), systemImage: "wifi.slash").font(.caption).padding(8).frame(maxWidth: .infinity).background(.regularMaterial)
            }
        }
        .overlay(alignment: .top) { if model.isBusy && !model.isHealthSyncRunning {
            ProgressView(uiText("Working…")).padding(10).background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12)).accessibilityLabel(uiText("Processing your request"))
        } }
        .alert(uiText("Could not complete the action"), isPresented: Binding(get: { model.errorMessage != nil }, set: { if !$0 { model.errorMessage = nil } })) {
            Button(uiText("OK")) { model.errorMessage = nil }
        } message: { Text(uiMessage(model.errorMessage ?? "")) }
    }
}
