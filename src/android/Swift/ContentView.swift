import SwiftUI
import SkipFuse

@MainActor @Observable final class AndroidApplication {
    static let shared = AndroidApplication()
    let model = AppModel()
    private init() {}
}

struct ContentView: View {
    @State var model = AndroidApplication.shared.model
    @State var languageSettings = InterfaceLanguageSettings.shared
    @State var dataPage: DataPage?

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
                    DataCenterView(model: model, selected: $dataPage)
                        .tabItem { Label(uiText("Data"), systemImage: MobileSymbol.name("externaldrive")) }.tag("Data")
                }
                // Skip's navigation host retains the initial text in tab items and toolbars.
                // Keep the active tab and Data destination outside that host when language changes.
                .id(languageSettings.language.rawValue)
                .initialTask { await model.resumeForeground() }
            }
        }.tint(.teal)
            .environment(\.locale, languageSettings.language.locale)
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
