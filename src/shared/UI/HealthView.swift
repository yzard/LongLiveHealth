import SwiftUI

struct HealthView: View {
    var model: AppModel
    @State var coverage: [JSONValue] = []
    #if os(iOS)
    @Environment(\.scenePhase) var scenePhase
    #else
    @State var now = Date()
    #endif
    @State var isVisible = false
    private var keepsScreenAwake: Bool {
        #if os(iOS)
        isVisible && scenePhase == .active && model.keepHealthScreenAwake
        #else
        isVisible && model.keepHealthScreenAwake
        #endif
    }
    var body: some View {
        List {
            Section {
                HStack(spacing: 16) {
                    Button {
                        Task { await model.synchronizeHealth(requestAccess: true, requestSelectedAccess: false, automatic: false); await reload() }
                    } label: {
                        HStack(spacing: 8) {
                            Text(uiText("Sync Now"))
                            if model.isHealthSyncRunning { ProgressView().accessibilityHidden(true) }
                        }.frame(minHeight: 44)
                    }.disabled(model.isHealthSyncRunning).accessibilityValue(model.isHealthSyncRunning ? "Syncing" : "")
                    Spacer(minLength: 8)
                    if model.isHealthSyncRunning {
                        Button { model.pauseHealthSync() } label: {
                            Text(uiText("Pause Sync")).frame(minHeight: 44)
                        }
                        if model.keepHealthScreenAwake {
                            InfoHelpButton(title: uiText("Screen awake"), message: uiText("Screen stays awake while syncing on this page."))
                        }
                    }
                }.buttonStyle(.borderless)
                Text(uiMessage(model.health.status)).font(.caption).fixedSize(horizontal: false, vertical: true)
                if let activity = model.health.activity, model.health.isSyncing {
                    #if os(iOS)
                    TimelineView(.periodic(from: .now, by: 1)) { context in
                        VStack(alignment: .leading, spacing: 4) {
                            ForEach(activity.lines(at: context.date, provider: MobilePlatform.healthProviderName), id: \.self) { Text($0) }
                        }.font(.caption).foregroundStyle(.secondary)
                    }
                    #else
                    VStack(alignment: .leading, spacing: 4) {
                        ForEach(activity.lines(at: now, provider: MobilePlatform.healthProviderName), id: \.self) { Text($0) }
                    }.font(.caption).foregroundStyle(.secondary).task {
                        while !Task.isCancelled {
                            now = Date()
                            do { try await Task.sleep(for: .seconds(1)) } catch { return }
                        }
                    }
                    #endif
                }
                #if os(iOS)
                Button(uiText("Select prescriptions and medications")) { Task { await model.synchronizeHealth(requestAccess: true, requestSelectedAccess: true, automatic: false); await reload() } }.disabled(model.isHealthSyncRunning)
                #endif
                if let note = model.backgroundHealth.registrationError { Text(note).font(.caption).foregroundStyle(.secondary) }
            }
            Section(uiText("Sync Error")) {
                let errors = coverage.filter { $0["status"] == .string("error") }
                if errors.isEmpty { Text(uiText("No sync errors")).foregroundStyle(.secondary) }
                ForEach(Array(errors.enumerated()), id: \.offset) { _, entry in
                    SyncCoverageRow(entry: entry)
                }
            }
            Section {
                NavigationLink(uiText("All Sync Coverage Categories")) {
                    List {
                        ForEach(Array(coverage.filter { $0["status"] == .string("error") }.enumerated()), id: \.offset) { _, entry in SyncCoverageRow(entry: entry) }
                        ForEach(Array(coverage.filter { $0["status"] != .string("error") }.enumerated()), id: \.offset) { _, entry in SyncCoverageRow(entry: entry) }
                    }.mobileNavigationTitle(uiText("Sync Coverage"))
                }
            }
        }.mobileNavigationTitle(uiLabel(MobilePlatform.healthProviderName)).initialTask { await reload() }.refreshable { await reload() }
            .onChange(of: model.isHealthSyncRunning) { _, running in if !running { Task { await reload() } } }
            .onAppear { isVisible = true }
            #if os(iOS)
            .onChange(of: keepsScreenAwake, initial: true) { _, enabled in UIApplication.shared.isIdleTimerDisabled = enabled }
            .onDisappear { isVisible = false; UIApplication.shared.isIdleTimerDisabled = false }
            #else
            .onChange(of: keepsScreenAwake, initial: true) { _, enabled in setAwake(enabled) }
            .onDisappear { isVisible = false; setAwake(false) }
            #endif
    }
    #if os(Android)
    private func setAwake(_ enabled: Bool) {
        do { _ = try AndroidSystem.invoke("screen.awake", enabled ? "true" : "false") }
        catch { model.errorMessage = error.localizedDescription }
    }
    #endif
    private func reload() async { do { if model.client != nil {
        let entries = try await model.request("health/coverage", body: .object([:]))["coverage"].arrayValue
        let installation = try model.archive?.load(UUID.self, name: "installation.json")
        coverage = HealthSyncCoverage.currentDevice(entries, platform: MobilePlatform.healthPlatform, installationID: installation)
    } } catch { model.errorMessage = error.localizedDescription } }
}

struct SyncCoverageRow: View {
    let entry: JSONValue
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(HealthSyncProgress.name(entry["record_type"].stringValue))
            Text(uiState(entry["status"].stringValue))
                .font(.caption).foregroundStyle(entry["status"] == .string("error") ? Color.red : Color.secondary)
            if let last = entry["last_success_at"].numberValue {
                Text(uiText("Last successful query: \(Date(timeIntervalSince1970: last).formatted())")).font(.caption)
            }
            if let first = entry["visible_start_at"].numberValue, let last = entry["visible_end_at"].numberValue {
                Text(uiText("Archived platform range: \(uiDate(Date(timeIntervalSince1970: first), dateStyle: .abbreviated, timeStyle: .omitted)) – \(uiDate(Date(timeIntervalSince1970: last), dateStyle: .abbreviated, timeStyle: .omitted))")).font(.caption)
            }
        }
    }
}
