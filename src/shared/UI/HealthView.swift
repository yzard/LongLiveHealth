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
                            Text("Sync Now")
                            if model.isHealthSyncRunning { ProgressView().accessibilityHidden(true) }
                        }.frame(minHeight: 44)
                    }.disabled(model.isHealthSyncRunning).accessibilityValue(model.isHealthSyncRunning ? "Syncing" : "")
                    Spacer(minLength: 8)
                    if model.isHealthSyncRunning {
                        Button { model.pauseHealthSync() } label: {
                            Text("Pause Sync").frame(minHeight: 44)
                        }
                        if model.keepHealthScreenAwake {
                            InfoHelpButton(title: "Screen awake", message: "Screen stays awake while syncing on this page.")
                        }
                    }
                }.buttonStyle(.borderless)
                Text(model.health.status).font(.caption).fixedSize(horizontal: false, vertical: true)
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
                Button("Select prescriptions and medications") { Task { await model.synchronizeHealth(requestAccess: true, requestSelectedAccess: true, automatic: false); await reload() } }.disabled(model.isHealthSyncRunning)
                #endif
                if let note = model.backgroundHealth.registrationError { Text(note).font(.caption).foregroundStyle(.secondary) }
            }
            Section("Sync coverage") { ForEach(Array(coverage.enumerated()), id: \.offset) { _, entry in
                VStack(alignment: .leading) { Text(entry["record_type"].stringValue); Text(entry["status"].stringValue).font(.caption)
                    if let last = entry["last_success_at"].numberValue { Text("Last successful query: \(Date(timeIntervalSince1970: last).formatted())").font(.caption) }
                    if let first = entry["visible_start_at"].numberValue, let last = entry["visible_end_at"].numberValue { Text("Archived platform range: \(Date(timeIntervalSince1970: first).formatted(date: .abbreviated, time: .omitted)) – \(Date(timeIntervalSince1970: last).formatted(date: .abbreviated, time: .omitted))").font(.caption) } }
            } }
        }.navigationTitle(MobilePlatform.healthProviderName).initialTask { await reload() }.refreshable { await reload() }
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
        coverage = try await model.request("health/coverage", body: .object([:]))["coverage"].arrayValue
    } } catch { model.errorMessage = error.localizedDescription } }
}
