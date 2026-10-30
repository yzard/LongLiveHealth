import SwiftUI
import SkipFuse

@MainActor @Observable final class AndroidApplication {
    static let shared = AndroidApplication()
    let model = AppModel()
    private init() {}
}

struct ContentView: View {
    @State var model = AndroidApplication.shared.model
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
                    Button("Unlock") { Task { await model.unlock() } }
                }
            } else if model.mustChangePassword {
                NavigationStack { PasswordChangeView(model: model, required: true) }
            } else {
                TabView(selection: $model.selectedTab) {
                    NavigationStack { OverviewView(model: model) }.tabItem { Label("Overview", systemImage: MobileSymbol.name("square.grid.2x2")) }.tag("Overview")
                    BrowseView(model: model).tabItem { Label("Browse", systemImage: MobileSymbol.name("list.bullet")) }.tag("Browse")
                    NavigationStack { TrendsView(model: model) }.tabItem { Label("Trends", systemImage: MobileSymbol.name("chart.xyaxis.line")) }.tag("Trends")
                    NavigationStack { AnalysisView(model: model) }.tabItem { Label("Insights", systemImage: MobileSymbol.name("sparkles")) }.tag("Insights")
                    DataCenterView(model: model).tabItem { Label("Data", systemImage: MobileSymbol.name("externaldrive")) }.tag("Data")
                }.initialTask { await model.resumeForeground() }
            }
        }.tint(.teal)
            .sheet(isPresented: Binding(get: { model.showSharedReportImport && model.session != nil && !model.isLocked && !model.privacyCovered }, set: { model.showSharedReportImport = $0 })) { SharedReportImportView(model: model) }
            .alert("Could not complete the action", isPresented: Binding(get: { model.errorMessage != nil }, set: { if !$0 { model.errorMessage = nil } })) {
                Button("OK") { model.errorMessage = nil }
            } message: { Text(model.errorMessage ?? "") }
            .task {
                while !Task.isCancelled {
                    await model.renewSessionIfNeeded()
                    do { try await Task.sleep(for: .seconds(30)) } catch { return }
                }
            }
    }
}
