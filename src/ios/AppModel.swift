import UserNotifications
import UIKit
import Foundation
import Observation
import CryptoKit
import LocalAuthentication

@MainActor @Observable
final class AppModel {
    private(set) var session: StoredSession?
    private var credentials: SessionController?
    private(set) var biometricUnlock = false
    private(set) var isLocked = false
    private(set) var isAuthenticating = false
    var privacyCovered = false
    private(set) var archive: LocalArchive?
    var entryMutation: EntryMutation?
    var reports: [JSONValue] = []
    var jobs: [JSONValue] = []
    var drafts: [UploadDraft] = []
    var metrics: [JSONValue] = []
    private(set) var sourcePreferences: JSONValue = .null
    private(set) var sourceCatalog: [JSONValue] = []
    var capabilities: JSONValue = .null
    var errorMessage: String?
    var selectedTab = "Overview"
    var dataPath: [DataDestination] = []
    var sharedReports: [SharedReport] = []
    var showSharedReportImport = false
    var isBusy = false
    var lastRefresh: Date?
    var offlineNotice: String?
    let health = HealthBridge()
    let backgroundHealth = HealthBackgroundDelivery()
    var mustChangePassword: Bool { session?.mustChangePassword == true }
    var client: APIClient? { mustChangePassword ? nil : credentials?.client() }

    init() {
        do { if let saved = try SessionVault.load() { try activate(saved) } }
        catch { errorMessage = error.localizedDescription }
        loadSharedReports()
    }

    private func activate(_ saved: StoredSession) throws {
        let key = SHA256.hash(data: Data("\(saved.server.absoluteString)|\(saved.userID)".utf8)).map { String(format: "%02x", $0) }.joined()
        let root = try FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
        let cache = try LocalArchive(directory: root.appendingPathComponent(key))
        reports = try cache.load([JSONValue].self, name: "reports.json") ?? []
        drafts = try cache.load([UploadDraft].self, name: "drafts.json") ?? []
        metrics = try cache.load([JSONValue].self, name: "metrics.json") ?? []
        sourcePreferences = try cache.load(JSONValue.self, name: "wellness-preferences.json") ?? .null
        sourceCatalog = try cache.load([JSONValue].self, name: "source-catalog.json") ?? []
        lastRefresh = try cache.load(Date.self, name: "refresh.json")
        credentials?.retire()
        credentials = SessionController(saved)
        biometricUnlock = saved.biometricUnlock == true
        isLocked = biometricUnlock
        session = saved; archive = cache
    }

    private static let cachedReads: Set<String> = [
        "wellness/day", "wellness/preferences/get", "wellness/sources", "wellness/series", "wellness/hrv",
        "wellness/sleep", "wellness/sleep/regularity", "wellness/review", "wellness/timeline", "wellness/report",
        "wellness/entries/list", "wellness/library", "wellness/nutrition/day", "wellness/diet", "wellness/meals",
        "wellness/meal-glucose", "wellness/import/list", "wellness/records/list", "wellness/records/get", "trends/get", "metrics/history/get", "metrics/browse"
    ]
    func request(_ path: String, body: JSONValue) async throws -> JSONValue {
        guard let client, let archive, let token = session?.token else { throw APIError.invalidResponse }
        let encoder = JSONEncoder(); encoder.outputFormatting = [.sortedKeys]
        let payload = try encoder.encode(body)
        var keyData = Data(path.utf8); keyData.append(0); keyData.append(payload)
        let key = "read-" + SHA256.hash(data: keyData).map { String(format: "%02x", $0) }.joined() + ".json"
        let cacheable = Self.cachedReads.contains(path)
        do {
            let result = try await client.post(path, body: body)
            guard !Task.isCancelled, session?.token == token else { throw CancellationError() }
            if path == "wellness/preferences/get" || path == "wellness/preferences/save" {
                if (result["version"].numberValue ?? 0) >= (sourcePreferences["version"].numberValue ?? -1) {
                    sourcePreferences = result
                    try archive.save(result, name: "wellness-preferences.json")
                }
            } else if path == "wellness/sources" {
                sourceCatalog = result["catalog"].arrayValue
                try archive.save(sourceCatalog, name: "source-catalog.json")
            }
            if cacheable {
                let saved: JSONValue = .object(["saved_at": .number(Date().timeIntervalSince1970), "result": result])
                if let bytes = try? encoder.encode(saved), bytes.count <= 2 * 1024 * 1024 {
                    try? archive.saveBytes(bytes, name: key)
                    try? pruneReadCache()
                }
            } else if ["save", "delete", "update", "review", "confirm", "sync"].contains(path.split(separator: "/").last.map(String.init) ?? "") || path.hasPrefix("wellness/import/") && !path.hasSuffix("/list") {
                try? clearReadCache()
            }
            if path == "wellness/entries/save" || path == "wellness/entries/delete" {
                entryMutation = EntryMutation(recordID: result["record_id"].stringValue, record: result["record"] == .null ? nil : result["record"])
            }
            return result
        } catch {
            guard !Task.isCancelled, session?.token == token else { throw CancellationError() }
            if case APIError.status(401, _) = error { try expireLocalSession(); throw error }
            if cacheable, let network = error as? URLError,
               [.timedOut, .notConnectedToInternet, .networkConnectionLost, .cannotFindHost, .cannotConnectToHost, .dnsLookupFailed].contains(network.code),
               let snapshot = try? archive.load(JSONValue.self, name: key), let savedAt = snapshot["saved_at"].numberValue {
                offlineNotice = "Cached view from " + Date(timeIntervalSince1970: savedAt).formatted() + (network.code == .timedOut ? ". Timeout." : ". Server unavailable.")
                return snapshot["result"]
            }
            throw error
        }
    }
    func sourceLabel(_ key: String) -> String {
        sourceLabel(key, name: key)
    }
    func sourceLabel(_ key: String, name: String) -> String {
        SourceNames.label(key: key, fallback: name, aliases: sourcePreferences["preferences"]["source_aliases"], catalog: sourceCatalog)
    }
    func sourceLabel(record: JSONValue) -> String {
        sourceLabel(SourceNames.key(record), name: record["source_name"].stringValue)
    }

    private func readCacheFiles() throws -> [URL] {
        guard let archive else { return [] }
        return try FileManager.default.contentsOfDirectory(at: archive.directory, includingPropertiesForKeys: [.contentModificationDateKey, .fileSizeKey]).filter { $0.lastPathComponent.hasPrefix("read-") && $0.pathExtension == "json" }
    }
    private func clearReadCache() throws {
        for file in try readCacheFiles() { try FileManager.default.removeItem(at: file) }
    }
    private func pruneReadCache() throws {
        let files = try readCacheFiles().sorted { ((try? $0.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate) ?? .distantPast) > ((try? $1.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate) ?? .distantPast) }
        var bytes = 0
        for (index, file) in files.enumerated() {
            bytes += (try? file.resourceValues(forKeys: [.fileSizeKey]).fileSize) ?? 0
            if index >= 64 || bytes > 32 * 1024 * 1024 { try FileManager.default.removeItem(at: file) }
        }
    }

    func login(server: String, username: String, password: String) async {
        await perform {
            let address = try APIClient.serverURL(server)
            let response = try await APIClient(server: address, token: nil).post("session/login", body: .object(["username": .string(username), "password": .string(password)]))
            var saved = StoredSession(server: address, userID: response["user"]["user_id"].stringValue, username: response["user"]["username"].stringValue, token: response["token"].stringValue)
            saved.mustChangePassword = response["user"]["must_change_password"].boolValue
            saved.refreshToken = response["refresh_token"].stringValue.isEmpty ? nil : response["refresh_token"].stringValue
            saved.expiresAt = response["expires_at"].numberValue
            saved.refreshAfter = response["refresh_after"].numberValue
            guard !saved.userID.isEmpty, !saved.token.isEmpty else { throw APIError.invalidResponse }
            try SessionVault.save(saved); try activate(saved)
            UserDefaults.standard.set(address.absoluteString, forKey: "lastServer")
        }
        if session != nil && !mustChangePassword { await refresh(automatic: false) }
    }

    func refresh(automatic: Bool) async {
        guard let client, let archive, let identity = session?.token, !isBusy else { return }
        await performRecoverable(automatic: automatic) {
            let account = try await client.post("user/get", body: .object([:]))
            if account["must_change_password"].boolValue {
                guard session?.token == identity else { return }
                session?.mustChangePassword = true
                if var latest = try SessionVault.load() { latest.mustChangePassword = true; try SessionVault.save(latest) }
                return
            }
            let status = try await client.post("server/status", body: .object([:]))
            var loaded: [JSONValue] = []; var cursor: JSONValue = .null
            repeat {
                let page = try await client.post("reports/list", body: .object(["limit": .number(100), "after_id": cursor]))["reports"].arrayValue
                loaded.append(contentsOf: page)
                guard let last = page.last else { break }
                cursor = last["report_id"]
                if page.count < 100 { break }
            } while true
            let definitions = try await client.post("metrics/list", body: .object([:]))["metrics"].arrayValue
            _ = try await request("wellness/preferences/get", body: .object([:]))
            let tasks = try await client.post("jobs/list", body: .object(["limit": .number(100)]))["jobs"].arrayValue
            guard session?.token == identity else { return }
            let retained = Set(loaded.map { $0["report_id"].stringValue })
            for previous in reports where !retained.contains(previous["report_id"].stringValue) {
                let id = previous["report_id"].stringValue
                try archive.remove(name: "report-\(id).json"); try archive.remove(name: "source-\(id)")
            }
            offlineNotice = nil; capabilities = status["capabilities"]; reports = loaded; jobs = tasks; metrics = definitions; lastRefresh = Date()
            try archive.save(reports, name: "reports.json"); try archive.save(metrics, name: "metrics.json"); try archive.save(lastRefresh, name: "refresh.json")
        }
    }

    private func reportInbox() throws -> ReportImportInbox {
        let root = try FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
        return try ReportImportInbox(directory: root.appendingPathComponent("shared-report-inbox"))
    }

    func loadSharedReports() {
        do { sharedReports = try reportInbox().reports() }
        catch { errorMessage = error.localizedDescription }
    }

    func receiveReportURL(_ url: URL) {
        do {
            guard url.isFileURL else { throw APIError.status(400, "Share a PDF file, not a web link") }
            let access = url.startAccessingSecurityScopedResource()
            defer { if access { url.stopAccessingSecurityScopedResource() } }
            let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize
            guard let size, size <= 20 * 1024 * 1024 else { throw APIError.status(413, "The document exceeds 20 MiB") }
            _ = try reportInbox().receive(Data(contentsOf: url), filename: url.lastPathComponent)
            loadSharedReports()
            showSharedReportImport = true
        } catch { errorMessage = error.localizedDescription }
    }

    func discardSharedReport(_ report: SharedReport) {
        do { try reportInbox().remove(report); loadSharedReports() }
        catch { errorMessage = error.localizedDescription }
    }

    func importSharedReports(sourceNames: [UUID: String]) async {
        guard let archive, client != nil, !isBusy else { return }
        do {
            let inbox = try reportInbox()
            let namedReports = try sharedReports.map { report in
                (report, try validatedReportSource(sourceNames[report.id] ?? ""))
            }
            for (report, sourceName) in namedReports {
                let draft = UploadDraft(id: report.id, filename: report.filename, contentType: "application/pdf", createdAt: report.createdAt, sourceName: sourceName, originalFilename: nil, originalContentType: nil)
                try enqueueDocument(draft, bytes: inbox.bytes(for: report), originalBytes: nil, archive: archive)
                try inbox.remove(report)
            }
            loadSharedReports()
            showSharedReportImport = false
            selectedTab = "Data"
            dataPath = [.reports]
        } catch { errorMessage = error.localizedDescription; return }
        await sendDrafts(automatic: false)
    }

    func addDocument(_ bytes: Data, filename: String, contentType: String, sourceName: String, originalBytes: Data?, originalFilename: String?, originalContentType: String?) async {
        guard let archive else { return }
        do {
            let draft = UploadDraft(id: UUID(), filename: filename, contentType: contentType, createdAt: Date(), sourceName: try validatedReportSource(sourceName), originalFilename: originalFilename, originalContentType: originalContentType)
            try enqueueDocument(draft, bytes: bytes, originalBytes: originalBytes, archive: archive)
        } catch { errorMessage = error.localizedDescription; return }
        await sendDrafts(automatic: false)
    }

    private func validatedReportSource(_ name: String) throws -> String {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, trimmed.utf8.count <= 160,
              trimmed.rangeOfCharacter(from: .controlCharacters) == nil,
              trimmed.caseInsensitiveCompare("Apple Health") != .orderedSame else {
            throw APIError.status(400, "Enter the clinic or laboratory name (up to 160 bytes). Apple Health is reserved.")
        }
        return trimmed
    }

    private func enqueueDocument(_ draft: UploadDraft, bytes: Data, originalBytes: Data?, archive: LocalArchive) throws {
        guard bytes.count <= 20 * 1024 * 1024 else { throw APIError.status(413, "The document exceeds 20 MiB") }
        guard (originalBytes == nil) == (draft.originalFilename == nil), (originalBytes == nil) == (draft.originalContentType == nil) else { throw APIError.invalidResponse }
        if let originalBytes, originalBytes.count > 20 * 1024 * 1024 { throw APIError.status(413, "The original photo exceeds 20 MiB") }
        guard !drafts.contains(where: { $0.id == draft.id }) else { return }
        if let originalBytes { try archive.saveBytes(originalBytes, name: "draft-original-\(draft.id.uuidString)") }
        try archive.saveBytes(bytes, name: "draft-\(draft.id.uuidString)")
        let updated = drafts + [draft]
        try archive.save(updated, name: "drafts.json")
        drafts = updated
    }

    func sendDrafts(automatic: Bool) async {
        guard let client, let archive, let identity = session?.token, !isBusy else { return }
        let lease = AppExecutionLease()
        lease.begin(name: "Report import") { }
        defer { lease.end() }
        await performRecoverable(automatic: automatic) {
            for draft in drafts {
                let uploaded = try await client.upload(archive.bytes(name: "draft-\(draft.id.uuidString)"), draft: draft, originalBytes: draft.originalFilename == nil ? nil : archive.bytes(name: "draft-original-\(draft.id.uuidString)"))
                guard session?.token == identity else { return }
                if let sourceName = draft.sourceName {
                    let reportID = uploaded["file"]["file_id"].stringValue
                    guard !reportID.isEmpty else { throw APIError.invalidResponse }
                    let report = try await client.post("reports/get", body: .object(["report_id": .string(reportID)]))
                    guard session?.token == identity else { return }
                    if report["context"]["source_name"].stringValue != sourceName {
                        var context = report["context"] == .null ? JSONValue.object([:]) : report["context"]
                        context["source_name"] = .string(sourceName)
                        _ = try await request("reports/review", body: .object([
                            "report_id": .string(reportID), "expected_revision": report["report"]["revision"],
                            "context": context, "observations": .array([])
                        ]))
                        guard session?.token == identity else { return }
                    }
                }
                // Persist queue removal only after the server acknowledges this stable upload ID.
                drafts.removeAll { $0.id == draft.id }; try archive.save(drafts, name: "drafts.json")
                try archive.remove(name: "draft-\(draft.id.uuidString)"); try archive.remove(name: "draft-original-\(draft.id.uuidString)")
            }
        }
        guard !Task.isCancelled else { return }
        await refresh(automatic: automatic)
    }

    func report(_ id: String) async throws -> JSONValue {
        guard let client, let archive, let currentSession = session else { throw APIError.invalidResponse }
        do {
            let report = try await client.post("reports/get", body: .object(["report_id": .string(id)]))
            guard session?.token == currentSession.token else { throw APIError.invalidResponse }
            try archive.save(report, name: "report-\(id).json"); return report
        } catch {
            guard session?.token == currentSession.token else { throw APIError.invalidResponse }
            if case APIError.status(404, _) = error { try archive.remove(name: "report-\(id).json"); try archive.remove(name: "source-\(id)"); throw error }
            if case APIError.status(401, _) = error { try expireLocalSession(); throw error }
            if let network = error as? URLError,
               [.timedOut, .notConnectedToInternet, .networkConnectionLost, .cannotFindHost, .cannotConnectToHost, .dnsLookupFailed].contains(network.code),
               let cached = try archive.load(JSONValue.self, name: "report-\(id).json") { return cached }
            throw error
        }
    }

    func source(_ id: String) async throws -> URL {
        guard let client, let archive, let currentSession = session else { throw APIError.invalidResponse }
        let location = try archive.location("source-\(id)")
        if FileManager.default.fileExists(atPath: location.path) { return location }
        try await client.download("files/\(id)/download", destination: location)
        guard session?.token == currentSession.token else { try? archive.remove(name: "source-\(id)"); throw APIError.invalidResponse }
        return location
    }

    private var healthSyncTask: Task<Void, Error>?
    private var deferredHealthSync: Task<Void, Never>?

    private func deferAutomaticHealthSync(until date: Date, identity: String) {
        guard deferredHealthSync == nil else { return }
        let delay = min(900, max(0, date.timeIntervalSinceNow))
        deferredHealthSync = Task { [weak self] in
            do { try await Task.sleep(for: .seconds(delay)) } catch { return }
            guard let self else { return }
            self.deferredHealthSync = nil
            guard self.session?.token == identity else { return }
            await self.automaticHealthSync()
        }
    }

    var isHealthSyncRunning: Bool { healthSyncTask != nil }
    private(set) var keepHealthScreenAwake = false

    func pauseHealthSync() {
        deferredHealthSync?.cancel()
        deferredHealthSync = nil
        keepHealthScreenAwake = false
        do { try archive?.save(true, name: "health-auto-paused.json") }
        catch { errorMessage = error.localizedDescription }
        healthSyncTask?.cancel()
    }

    func suspendHealthSync() {
        keepHealthScreenAwake = false
        healthSyncTask?.cancel()
    }

    func automaticHealthSync() async {
        if session == nil, UIApplication.shared.isProtectedDataAvailable {
            do { if let saved = try SessionVault.load() { try activate(saved); errorMessage = nil } }
            catch { health.status = "Waiting for secure session access."; return }
        }
        guard let archive, let identity = session?.token, client != nil else { return }
        do {
            guard try archive.load(Bool.self, name: "health-enabled.json") == true,
                  try archive.load(Bool.self, name: "health-auto-paused.json") != true else { return }
            guard UIApplication.shared.isProtectedDataAvailable else {
                health.status = "Waiting for iPhone unlock; sync will resume automatically."
                return
            }
            if let running = healthSyncTask {
                _ = try? await running.value
                while healthSyncTask != nil && !Task.isCancelled { await Task.yield() }
            }
            guard !isBusy, !Task.isCancelled else { return }
            guard session?.token == identity, client != nil,
                  try archive.load(Bool.self, name: "health-auto-paused.json") != true else { return }
            if let next = try archive.load(Date.self, name: "health-next-automatic-sync.json"), next > Date() {
                if health.status == "Not connected" {
                    health.status = "Automatic sync is waiting for the next check. Tap Sync Now to check now."
                }
                deferAutomaticHealthSync(until: next, identity: identity)
                return
            }
            await synchronizeHealth(requestAccess: false, requestSelectedAccess: false, automatic: true)
        } catch { health.status = "Automatic sync paused: " + error.localizedDescription }
    }

    func synchronizeHealth(requestAccess: Bool, requestSelectedAccess: Bool, automatic: Bool) async {
        guard let client, let archive, let identity = session?.token, !isHealthSyncRunning, !Task.isCancelled else { return }
        do {
            if !requestAccess, try archive.load(Bool.self, name: "health-enabled.json") != true { return }
        } catch { errorMessage = error.localizedDescription; return }
        guard UIApplication.shared.isProtectedDataAvailable else {
            health.status = "Waiting for iPhone unlock; sync will resume automatically."
            return
        }
        if !automatic {
            do { try archive.save(false, name: "health-auto-paused.json") }
            catch { errorMessage = error.localizedDescription; return }
        }
        // Persist throttling per account/server so relaunches cannot restart a loop.
        // A failed attempt may retry after one minute; successful runs wait 15 minutes.
        do { try archive.save(Date().addingTimeInterval(60), name: "health-next-automatic-sync.json") }
        catch { errorMessage = error.localizedDescription; return }
        deferredHealthSync?.cancel()
        deferredHealthSync = nil
        health.status = "Preparing Apple Health sync"
        let lease = AppExecutionLease()
        lease.begin(name: "Health archive sync") { [weak self] in self?.suspendHealthSync() }
        let task = Task {
            try Task.checkCancellation()
            if !requestAccess, try archive.load(Bool.self, name: "health-enabled.json") == true {
                await backgroundHealth.enable()
                try Task.checkCancellation()
            }
            try await health.synchronize(client: client, archive: archive, requestAccess: requestAccess, requestSelectedAccess: requestSelectedAccess)
            if requestAccess { await backgroundHealth.enable() }
            try Task.checkCancellation()
        }
        healthSyncTask = task
        keepHealthScreenAwake = true
        defer { keepHealthScreenAwake = false; healthSyncTask = nil; lease.end() }
        do {
            try await withTaskCancellationHandler { try await task.value } onCancel: { task.cancel() }
            let next = Date().addingTimeInterval(900)
            try archive.save(next, name: "health-next-automatic-sync.json")
            deferAutomaticHealthSync(until: next, identity: identity)
            try clearReadCache()
        } catch {
            guard session?.token == identity else { return }
            if task.isCancelled || HealthBridge.isProtectedDataError(error) || UIApplication.shared.applicationState != .active {
                do { try archive.save(Date(), name: "health-next-automatic-sync.json") }
                catch { errorMessage = error.localizedDescription }
                let manuallyPaused = (try? archive.load(Bool.self, name: "health-auto-paused.json")) == true
                health.status = manuallyPaused
                    ? "Sync paused; progress saved. Tap Sync Now to resume automatic sync."
                    : "Sync paused; progress saved. It can resume after unlock or the next background opportunity."
                return
            }
            if case APIError.status(401, _) = error { try? expireLocalSession() }
            if automatic {
                health.status = "Automatic sync paused; will retry. " + error.localizedDescription
                deferAutomaticHealthSync(until: Date().addingTimeInterval(60), identity: identity)
                return
            }
            errorMessage = "Apple Health sync paused. Saved batches are retained; tap Sync Now to resume. " + error.localizedDescription
        }
    }

    func changePassword(current: String, replacement: String) async {
        guard let saved = session, let credentials, !isHealthSyncRunning else { return }
        await perform {
            _ = try await credentials.client().post("user/password/change", body: .object(["current_password": .string(current), "new_password": .string(replacement)]))
            guard session?.token == saved.token else { return }
            deferredHealthSync?.cancel(); deferredHealthSync = nil
            credentials.retire(); self.credentials = nil
            try BackgroundTransfer.shared.cancelAll()
            try SessionVault.clear()
            sourcePreferences = .null; sourceCatalog = []; session = nil; archive = nil; reports = []; jobs = []; drafts = []; metrics = []; capabilities = .null
            lastRefresh = nil; offlineNotice = nil
            UNUserNotificationCenter.current().removeAllPendingNotificationRequests()
            UNUserNotificationCenter.current().removeAllDeliveredNotifications()
        }
    }

    func logout() async {
        guard !isBusy, !isHealthSyncRunning else { return }
        let previousSession = credentials?.saved
        let previous = previousSession.map { APIClient(server: $0.server, token: $0.token) }
        do { try clearLocalSession() }
        catch { errorMessage = error.localizedDescription; return }
        // Local removal does not wait for a network timeout. Server revocation is best effort.
        if let previous { Task {
            if let refresh = previousSession?.refreshToken {
                _ = try? await previous.post("session/revoke", body: .object(["refresh_token": .string(refresh)]))
            } else { _ = try? await previous.post("session/logout", body: .object([:])) }
        } }
    }

    func discardDraft(_ id: UUID) {
        guard let archive, !isBusy else { return }
        do {
            drafts.removeAll { $0.id == id }
            try archive.save(drafts, name: "drafts.json")
            try archive.remove(name: "draft-\(id.uuidString)"); try archive.remove(name: "draft-original-\(id.uuidString)")
        } catch { errorMessage = error.localizedDescription }
    }

    func clearLocalSession() throws {
        guard !isHealthSyncRunning else { throw APIError.status(409, "Pause Apple Health sync before clearing this device") }
        deferredHealthSync?.cancel()
        deferredHealthSync = nil
        credentials?.retire(); credentials = nil
        isLocked = false; biometricUnlock = false
        try BackgroundTransfer.shared.cancelAll()
        try SessionVault.clear(); try archive?.clear()
        UNUserNotificationCenter.current().removeAllPendingNotificationRequests()
        UNUserNotificationCenter.current().removeAllDeliveredNotifications()
        entryMutation = nil; offlineNotice = nil; sourcePreferences = .null; sourceCatalog = []; session = nil; archive = nil; reports = []; drafts = []; jobs = []; metrics = []; lastRefresh = nil; capabilities = .null; health.status = "Not connected"
    }

    private func expireLocalSession() throws {
        healthSyncTask?.cancel()
        deferredHealthSync?.cancel()
        deferredHealthSync = nil
        credentials?.retire(); credentials = nil
        try BackgroundTransfer.shared.cancelAll()
        try SessionVault.clear()
        // Preserve pending reports and Health checkpoints for this account's next login.
        sourcePreferences = .null; sourceCatalog = []; session = nil; archive = nil; reports = []; drafts = []; jobs = []; metrics = []
        capabilities = .null; lastRefresh = nil; isLocked = false; biometricUnlock = false
        UNUserNotificationCenter.current().removeAllPendingNotificationRequests()
        UNUserNotificationCenter.current().removeAllDeliveredNotifications()
    }

    func renewSessionIfNeeded() async {
        let identity = session?.token
        do { _ = try await credentials?.credential(force: false) }
        catch {
            guard session?.token == identity else { return }
            if case APIError.status(401, _) = error {
                try? expireLocalSession()
                errorMessage = "Your session was revoked or expired. Sign in again; your pending imports are retained."
            }
            // Connectivity failures retain credentials and retry on the next request.
        }
    }

    func lockForBackground() { if biometricUnlock && session != nil { isLocked = true } }

    func unlock() async {
        guard isLocked, !isAuthenticating else { return }
        isAuthenticating = true; defer { isAuthenticating = false }
        let identity = session?.token
        do {
            let context = LAContext()
            let success = try await context.evaluatePolicy(.deviceOwnerAuthentication, localizedReason: "Unlock your health archive")
            guard session?.token == identity, success else { return }
            isLocked = false
        } catch { /* Cancellation leaves the privacy screen in place. */ }
    }

    func setBiometricUnlock(_ enabled: Bool) async {
        guard !isAuthenticating, let credentials else { return }
        isAuthenticating = true; defer { isAuthenticating = false }
        let identity = session?.token
        do {
            let context = LAContext()
            var failure: NSError?
            if enabled && !context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &failure) {
                throw failure ?? APIError.status(400, "Set up Face ID or Touch ID in iPhone Settings first") as NSError
            }
            guard try await context.evaluatePolicy(.deviceOwnerAuthentication, localizedReason: "Change biometric unlock for your health archive"), session?.token == identity else { return }
            try credentials.setBiometricUnlock(enabled)
            biometricUnlock = enabled
        } catch { errorMessage = error.localizedDescription }
    }

    private var foregroundRecovery: Task<Void, Never>?

    func resumeForeground() async {
        if let foregroundRecovery { await foregroundRecovery.value; return }
        let task = Task { [weak self] in
            guard let self else { return }
            await self.unlock()
            guard !self.isLocked, !self.privacyCovered, !Task.isCancelled else { return }
            await self.renewSessionIfNeeded()
            guard !Task.isCancelled else { return }
            await self.sendDrafts(automatic: true)
            guard !Task.isCancelled else { return }
            await self.automaticHealthSync()
        }
        foregroundRecovery = task
        defer { foregroundRecovery = nil }
        await task.value
    }

    func suspendForegroundRecovery() {
        foregroundRecovery?.cancel()
    }

    // Retries are limited to replayable refreshes and uploads with persistent IDs.
    // User actions keep their explicit error presentation; automatic work uses status.
    func performRecoverable(automatic: Bool, _ operation: () async throws -> Void) async {
        await perform {
            for attempt in 0..<3 {
                try Task.checkCancellation()
                do {
                    try await operation()
                    return
                } catch let error as URLError where automatic && [
                    .timedOut, .notConnectedToInternet, .networkConnectionLost,
                    .cannotFindHost, .cannotConnectToHost, .dnsLookupFailed
                ].contains(error.code) {
                    try Task.checkCancellation()
                    if attempt == 2 {
                        offlineNotice = "Server unavailable. Your login and pending imports are saved. Reopen the app or retry when connected."
                        return
                    }
                    try await Task.sleep(for: .seconds(attempt == 0 ? 1 : 3))
                }
            }
        }
    }

    func perform(_ operation: () async throws -> Void) async {
        guard !isBusy else { return }
        let identity = session?.token
        isBusy = true; defer { isBusy = false }
        do { try await operation() }
        catch {
            guard session?.token == identity else { return }
            if case APIError.status(401, _) = error { try? expireLocalSession() }
            if error is CancellationError || (error as? URLError)?.code == .cancelled || Task.isCancelled { return }
            errorMessage = error.localizedDescription
        }
    }
}

enum DataDestination: Hashable { case reports, health }
