import SwiftUI
#if canImport(Charts)
import Charts
#endif

enum DataPage: Hashable {
    case reports, health, training, entries, settings, aliases, counts
}

struct DataCenterView: View {
    var model: AppModel
    #if os(Android)
    @State var selected: DataPage?
    var body: some View {
        AdaptiveDetailLayout(selected: selected != nil) {
            NavigationStack { menu }
        } detail: {
            if let selected {
                NavigationStack {
                    destination(selected).toolbar {
                        ToolbarItem(placement: .topBarLeading) {
                            Button { self.selected = nil } label: { Image(systemName: "chevron.left") }
                                .accessibilityLabel(Text("Back"))
                        }
                    }
                }.id(selected)
            }
            else { EmptyContent(title: "Choose a data tool", symbol: "externaldrive") }
        }.onChange(of: model.dataPath) { _, path in
            if let last = path.last { selected = last == .reports ? .reports : .health }
        }
    }
    #else
    var body: some View { menu }
    #endif
    private var menu: some View {
        List {
            Section("Import") {
                link("Blood Work", page: .reports)
                if !model.sharedReports.isEmpty { Button("Import shared PDFs (\(model.sharedReports.count))") { model.showSharedReportImport = true } }
                link(MobilePlatform.healthProviderName, page: .health)
                link("Import GPX, TCX, or FIT activities", page: .training)
                link("Manual Imports", page: .entries)
            }
            Section("Sources") {
                link("Source Aliases", page: .aliases)
                link("Sources Record Count", page: .counts)
            }
            ExportSection(model: model)
        }.mobileNavigationTitle("Data")
            .navigationDestination(for: DataDestination.self) { value in destination(value == .reports ? .reports : .health) }
            .toolbar {
                #if os(Android)
                Button { selected = .settings } label: { Label("Settings", systemImage: MobileSymbol.name("gearshape")) }
                #else
                NavigationLink(destination: SettingsView(model: model)) { Label("Settings", systemImage: MobileSymbol.name("gearshape")) }
                #endif
            }
    }
    @ViewBuilder private func link(_ title: String, page: DataPage) -> some View {
        #if os(Android)
        Button { selected = page } label: { row(title, page: page) }
        #else
        if page == .reports || page == .health {
            NavigationLink(value: page == .reports ? DataDestination.reports : .health) { row(title, page: page) }
        } else { NavigationLink(destination: destination(page)) { row(title, page: page) } }
        #endif
    }
    private func row(_ title: String, page: DataPage) -> some View {
        HStack {
            Text(title)
            Spacer()
            if page == .health && model.isHealthSyncRunning { ProgressView().accessibilityHidden(true) }
        }.accessibilityValue(page == .health && model.isHealthSyncRunning ? "Syncing" : "")
    }
    @ViewBuilder private func destination(_ page: DataPage) -> some View {
        switch page {
        case .reports: ReportsView(model: model)
        case .health: HealthView(model: model)
        case .training: TrainingImportsView(model: model)
        case .entries: DailyLogsView(model: model)
        case .settings: SettingsView(model: model)
        case .aliases: SourceAliasesView(model: model)
        case .counts: SourceRecordCountsView(model: model)
        }
    }
}

