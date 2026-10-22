import SwiftUI

struct HealthView: View {
    var model: AppModel
    @State private var coverage: [JSONValue] = []
    @Environment(\.scenePhase) private var scenePhase
    @State private var isVisible = false
    private var keepsScreenAwake: Bool { isVisible && scenePhase == .active && model.keepHealthScreenAwake }
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
                    }
                }.buttonStyle(.borderless)
                if model.keepHealthScreenAwake { Text("Screen stays awake while syncing on this page.").font(.caption).foregroundStyle(.secondary) }
                Text(model.health.status).font(.caption).fixedSize(horizontal: false, vertical: true)
                if let activity = model.health.activity, model.health.isSyncing {
                    TimelineView(.periodic(from: .now, by: 1)) { context in
                        VStack(alignment: .leading, spacing: 4) {
                            ForEach(activity.lines(at: context.date), id: \.self) { Text($0) }
                        }.font(.caption).foregroundStyle(.secondary)
                    }
                }
                Button("Select prescriptions and medications") { Task { await model.synchronizeHealth(requestAccess: true, requestSelectedAccess: true, automatic: false); await reload() } }.disabled(model.isHealthSyncRunning)
                Text("Vision prescriptions and medications require individual selection. Use this button each time you want to archive those records.").font(.caption).foregroundStyle(.secondary)
                if let note = model.backgroundHealth.registrationError { Text(note).font(.caption).foregroundStyle(.secondary) }
                Text("Sync resumes when the app opens and when iOS grants background time. Locked Health data waits for unlock. Pause disables automatic sync until you tap Sync Now.").font(.caption).foregroundStyle(.secondary)
                Text("Only data visible to this iPhone can sync. An empty result does not tell us whether access was denied or data is absent.").font(.caption).foregroundStyle(.secondary)
            }
            Section { Text("The app archives the HealthKit types available on this device, including sample metadata, ECG and other series, routes, clinical FHIR records, characteristics and activity summaries. Clinical records need separate permission and device support. Read coverage is shown below; hidden data cannot be inferred.").font(.caption).foregroundStyle(.secondary) }
            Section("Sync coverage") { ForEach(Array(coverage.enumerated()), id: \.offset) { _, entry in
                VStack(alignment: .leading) { Text(entry["record_type"].stringValue); Text(entry["status"].stringValue).font(.caption)
                    if let last = entry["last_success_at"].numberValue { Text("Last successful query: \(Date(timeIntervalSince1970: last).formatted())").font(.caption) }
                    if let first = entry["visible_start_at"].numberValue, let last = entry["visible_end_at"].numberValue { Text("Archived platform range: \(Date(timeIntervalSince1970: first).formatted(date: .abbreviated, time: .omitted)) – \(Date(timeIntervalSince1970: last).formatted(date: .abbreviated, time: .omitted))").font(.caption) } }
            } }
        }.navigationTitle("Apple Health").initialTask { await reload() }.refreshable { await reload() }
            .onAppear { isVisible = true }
            .onChange(of: keepsScreenAwake, initial: true) { _, enabled in UIApplication.shared.isIdleTimerDisabled = enabled }
            .onDisappear { isVisible = false; UIApplication.shared.isIdleTimerDisabled = false }
    }
    private func reload() async { do { if model.client != nil {
        coverage = try await model.request("health/coverage", body: .object([:]))["coverage"].arrayValue
    } } catch { model.errorMessage = error.localizedDescription } }
}

func dateScope(days: Int) -> JSONValue {
    let calendar = Calendar.current
    let end = Date(); let start = calendar.date(byAdding: .day, value: -days, to: end) ?? end
    let format = DateFormatter(); format.calendar = Calendar(identifier: .gregorian); format.locale = Locale(identifier: "en_US_POSIX"); format.timeZone = .current; format.dateFormat = "yyyy-MM-dd"
    return .object(["start_date": .string(format.string(from: start)), "end_date": .string(format.string(from: end)), "timezone": .string(TimeZone.current.identifier)])
}
