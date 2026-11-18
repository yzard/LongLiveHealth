import SwiftUI
import SkipFuse

@MainActor @Observable final class AndroidApplication {
    static let shared = AndroidApplication()
    let model = AppModel()
    var dataPage: DataPage?
    private init() {}
}

struct ContentView: View {
    @State var model = AndroidApplication.shared.model
    @State var application = AndroidApplication.shared

    var body: some View {
        Group {
            if model.session == nil {
                LoginScreen(isBusy: model.isBusy, onLogin: { server, user, password in
                    await model.login(server: server, username: user, password: password)
                })
            } else if model.isLocked || model.privacyCovered {
                VStack(spacing: 20) {
                    Image(systemName: MobileSymbol.name("lock.fill")).font(.largeTitle)
                    Text("Long Live Health").font(.title)
                    Button(uiText("Unlock")) { Task { await model.unlock() } }
                }
            } else if model.mustChangePassword {
                NavigationStack { PasswordChangeView(model: model, required: true) }
            } else {
                TabView(selection: $model.selectedTab) {
                    NavigationStack { InsightsView(model: model) }
                        .tabItem { Label(uiText("Insights"), systemImage: MobileSymbol.name("sparkles")) }.tag("Insights")
                    BrowseView(model: model)
                        .tabItem { Label(uiText("Browse"), systemImage: MobileSymbol.name("list.bullet")) }.tag("Browse")
                    DataCenterView(model: model, selected: $application.dataPage)
                        .tabItem { Label(uiText("Data"), systemImage: MobileSymbol.name("externaldrive")) }.tag("Data")
                }
                .initialTask { await model.resumeForeground() }
            }
        }.tint(.teal)
            .environment(\.locale, InterfaceLanguageSettings.shared.language.locale)
            .onChange(of: model.session?.userID) { _, _ in application.dataPage = nil }
            .sheet(isPresented: Binding(get: { model.showSharedReportImport && model.session != nil && !model.isLocked && !model.privacyCovered }, set: { model.showSharedReportImport = $0 })) { SharedReportImportView(model: model) }
            .alert(uiText("Could not complete the action"), isPresented: Binding(get: { model.errorMessage != nil }, set: { if !$0 { model.errorMessage = nil } })) {
                Button(uiText("OK")) { model.errorMessage = nil }
            } message: { Text(uiMessage(model.errorMessage ?? "")) }
            .task {
                while !Task.isCancelled {
                    await model.renewSessionIfNeeded()
                    do { try await Task.sleep(for: .seconds(30)) } catch { return }
                }
            }
    }
}
