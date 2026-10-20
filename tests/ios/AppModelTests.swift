import Foundation
import UIKit
import Testing
@testable import LongLiveHealth

@MainActor @Suite(.serialized) struct AppModelTests {
    @Test func sourceAliasesPersistForTheirAccountAndUpdateRetainedLabels() async throws {
        try SessionVault.clear()
        let model = AppModel()
        await model.login(server: "http://localhost:18765", username: "simulator", password: "synthetic-test-password")
        let current = try await model.request("wellness/preferences/get", body: .object([:]))
        var preferences = current["preferences"]
        preferences["source_aliases"] = .object(["apple_health:watch": .string("Personal Watch")])
        _ = try await model.request("wellness/preferences/save", body: .object(["expected_version": current["version"], "batch_id": .string(UUID().uuidString), "preferences": preferences]))
        #expect(model.sourceLabel("apple_health:watch") == "Personal Watch")
        let restored = AppModel()
        #expect(restored.sourceLabel("apple_health:watch") == "Personal Watch")
        try restored.clearLocalSession()
        #expect(restored.sourceLabel("apple_health:watch") == "apple_health:watch")
        let isolated = StoredSession(server: URL(string: "http://localhost:18765")!, userID: UUID().uuidString, username: "other", token: "other-token")
        try SessionVault.save(isolated)
        let other = AppModel()
        #expect(other.sourceLabel("apple_health:watch") == "apple_health:watch")
        try other.clearLocalSession()
        let control = APIClient(server: URL(string: "http://localhost:18765")!, token: nil)
        _ = try await control.post("test/reset", body: .object([:]))
    }

    @Test func automaticConnectivityFailureRetriesWithoutAlertOrDiscardingLogin() async throws {
        try SessionVault.clear()
        let model = AppModel()
        await model.login(server: "http://localhost:18765", username: "simulator", password: "synthetic-test-password")
        let saved = try #require(model.session)
        let archive = try #require(model.archive)
        try archive.saveBytes(Data("retained".utf8), name: "recovery-checkpoint")
        var attempts = 0
        await model.performRecoverable(automatic: true) {
            attempts += 1
            throw URLError(.networkConnectionLost)
        }
        #expect(attempts == 3)
        #expect(model.errorMessage == nil && model.offlineNotice != nil)
        #expect(model.session?.token == saved.token)
        #expect(try archive.bytes(name: "recovery-checkpoint") == Data("retained".utf8))
        await model.refresh(automatic: true)
        #expect(model.offlineNotice == nil)
        attempts = 0
        await model.performRecoverable(automatic: true) {
            attempts += 1
            if attempts == 1 { throw URLError(.networkConnectionLost) }
        }
        #expect(attempts == 2 && model.errorMessage == nil)
        await model.performRecoverable(automatic: false) { throw URLError(.networkConnectionLost) }
        #expect(model.errorMessage != nil)
        model.errorMessage = nil
        await model.performRecoverable(automatic: true) { throw CancellationError() }
        #expect(model.errorMessage == nil && !model.isBusy)
        await model.performRecoverable(automatic: true) { throw APIError.status(403, "Denied") }
        #expect(model.errorMessage != nil)
        try model.clearLocalSession()
    }

    @Test func concurrentForegroundTriggersOnlyRefreshOnce() async throws {
        try SessionVault.clear()
        let model = AppModel()
        await model.login(server: "http://localhost:18765", username: "simulator", password: "synthetic-test-password")
        let client = try #require(model.client)
        try model.archive?.save(false, name: "health-enabled.json")
        _ = try await client.post("test/reset-counts", body: .object([:]))
        async let first: Void = model.resumeForeground()
        async let second: Void = model.resumeForeground()
        _ = await (first, second)
        let counts = try await client.post("test/counts", body: .object([:]))
        #expect(counts["user/get"].numberValue == 1)
        #expect(counts["reports/list"].numberValue == 1)
        #expect(model.errorMessage == nil)
        try model.clearLocalSession()
    }

    @Test func renewalCoalescesRequestsAndPersistsAcrossRelaunch() async throws {
        try SessionVault.clear()
        let control = APIClient(server: URL(string: "http://localhost:18765")!, token: nil)
        _ = try await control.post("test/reset", body: .object([:]))
        var saved = StoredSession(server: control.server, userID: "simulator-only", username: "simulator", token: "expired-access")
        saved.refreshToken = String(repeating: "a", count: 64); saved.refreshAfter = 0
        try SessionVault.save(saved)
        let owner = SessionController(saved)
        async let first = owner.credential(force: false)
        async let second = owner.credential(force: false)
        let tokens = try await (first, second)
        #expect(tokens.0 == tokens.1 && tokens.0 != saved.token)
        let restored = try #require(try SessionVault.load())
        #expect(restored.token == tokens.0 && restored.pendingRefreshToken == nil)
        let relaunched = SessionController(restored)
        #expect(try await relaunched.credential(force: false) == tokens.0)
        let counts = try await control.post("test/counts", body: .object([:]))
        #expect(counts["session/refresh"].numberValue == 1)
        owner.retire(); relaunched.retire(); try SessionVault.clear()
    }

    @Test func renewalFailureRetainsPendingSuccessorAndRetirementCannotRestoreLogin() async throws {
        try SessionVault.clear()
        let control = APIClient(server: URL(string: "http://localhost:18765")!, token: nil)
        _ = try await control.post("test/reset", body: .object([:]))
        _ = try await control.post("test/fail-refresh", body: .object([:]))
        var saved = StoredSession(server: control.server, userID: "simulator-only", username: "simulator", token: "expired-access")
        saved.refreshToken = String(repeating: "b", count: 64); saved.refreshAfter = 0
        try SessionVault.save(saved)
        let owner = SessionController(saved)
        do { _ = try await owner.credential(force: false); Issue.record("Expected transient refresh failure") }
        catch APIError.status(503, _) { }
        let pending = try #require(try SessionVault.load())
        #expect(pending.token == saved.token && pending.refreshToken == saved.refreshToken)
        let successor = try #require(pending.pendingRefreshToken)
        let relaunched = SessionController(pending)
        _ = try await relaunched.credential(force: false)
        #expect(try SessionVault.load()?.refreshToken == successor)
        let inFlight = Task { try await relaunched.credential(force: true) }
        try await Task.sleep(for: .milliseconds(50))
        relaunched.retire(); try SessionVault.clear()
        do { _ = try await inFlight.value; Issue.record("Retired login must not return credentials") } catch { }
        #expect(try SessionVault.load() == nil)
        owner.retire()
    }

    @Test func revokedSessionRetainsPendingAccountImports() async throws {
        try SessionVault.clear()
        let model = AppModel()
        await model.login(server: "http://localhost:18765", username: "simulator", password: "synthetic-test-password")
        let archive = try #require(model.archive)
        let marker = Data("pending account checkpoint".utf8)
        try archive.saveBytes(marker, name: "session-regression-checkpoint")
        do { _ = try await model.request("test/unauthorized", body: .object([:])); Issue.record("Expected revoked session") }
        catch APIError.status(401, _) { }
        #expect(model.session == nil)
        #expect(try SessionVault.load() == nil)
        #expect(try archive.bytes(name: "session-regression-checkpoint") == marker)
        try archive.remove(name: "session-regression-checkpoint")
    }

    @Test func biometricPreferenceRestoresLockedWithoutDiscardingBackgroundSession() throws {
        try SessionVault.clear()
        var saved = StoredSession(server: URL(string: "http://localhost:18765")!, userID: "simulator-only", username: "simulator", token: "synthetic-token")
        saved.biometricUnlock = true
        try SessionVault.save(saved)
        let model = AppModel()
        #expect(model.biometricUnlock && model.isLocked)
        #expect(model.client != nil)
        AppLockShield.shared.update(model: model)
        let windows = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.flatMap(\.windows)
        #expect(windows.contains { !$0.isHidden && $0.windowLevel > .alert })
        try model.clearLocalSession()
        AppLockShield.shared.update(model: model)
        #expect(!model.isLocked && model.session == nil)
        #expect(windows.filter { $0.windowLevel > .alert }.allSatisfy { $0.isHidden })
    }

    @Test func backgroundUploadSurvivesCallerCancellationAndCoalescesRetries() async throws {
        let control = APIClient(server: URL(string: "http://localhost:18765")!, token: nil)
        _ = try await control.post("test/reset", body: .object([:]))
        _ = try await control.post("test/slow-upload", body: .object([:]))
        let client = APIClient(server: URL(string: "http://localhost:18765")!, token: "synthetic-background")
        let draft = UploadDraft(id: UUID(), filename: "background.pdf", contentType: "application/pdf", createdAt: Date(), sourceName: "Synthetic Lab", originalFilename: nil, originalContentType: nil)
        let bytes = Data("%PDF-1.4\nsynthetic background transfer".utf8)
        let first = Task { try await client.upload(bytes, draft: draft, originalBytes: nil) }
        try await Task.sleep(for: .milliseconds(100))
        first.cancel()
        let second = Task { try await client.upload(bytes, draft: draft, originalBytes: nil) }
        let firstResult = try await first.value
        let secondResult = try await second.value
        #expect(firstResult == secondResult)
        #expect(firstResult["file"]["file_id"].stringValue == "fixture-report")
        let counts = try await control.post("test/counts", body: .object([:]))
        #expect(counts["files/upload"].numberValue == 1)
    }
    @Test func sharedReportWaitsForAccountAndRetriesWithStableUploadIdentity() async throws {
        try SessionVault.clear()
        let control = APIClient(server: URL(string: "http://localhost:18765")!, token: nil)
        _ = try await control.post("test/reset", body: .object([:]))
        let file = FileManager.default.temporaryDirectory.appendingPathComponent("shared-\(UUID()).pdf")
        try Data("%PDF-1.4\nsynthetic report".utf8).write(to: file)
        defer { try? FileManager.default.removeItem(at: file) }
        let signedOut = AppModel()
        // The system-share test leaves durable, unassigned fixtures if interrupted.
        for report in signedOut.sharedReports where report.filename.range(of: "^shared-(report(-[0-9]+)?|[0-9A-Fa-f-]{36})\\.pdf$", options: .regularExpression) != nil {
            signedOut.discardSharedReport(report)
        }
        signedOut.receiveReportURL(file)
        let shared = try #require(signedOut.sharedReports.first { $0.filename == file.lastPathComponent })
        await signedOut.importSharedReports(sourceNames: [shared.id: "Function Health"])
        #expect(signedOut.sharedReports.contains { $0.id == shared.id })
        let restored = AppModel()
        #expect(restored.sharedReports.contains { $0.id == shared.id })
        await restored.login(server: "http://localhost:18765", username: "simulator", password: "synthetic-test-password")
        await restored.importSharedReports(sourceNames: [shared.id: " "])
        #expect(restored.sharedReports.contains { $0.id == shared.id })
        #expect(restored.drafts.isEmpty)
        _ = try await control.post("test/fail-upload", body: .object([:]))
        await restored.importSharedReports(sourceNames: [shared.id: "Function Health"])
        #expect(restored.sharedReports.isEmpty)
        #expect(restored.drafts.map(\.id) == [shared.id])
        #expect(restored.drafts.first?.sourceName == "Function Health")
        #expect(restored.selectedTab == "Data" && restored.dataPath == [.reports])
        _ = try await control.post("test/fail-review", body: .object([:]))
        await restored.sendDrafts(automatic: false)
        #expect(restored.drafts.map(\.id) == [shared.id])
        #expect(restored.drafts.first?.sourceName == "Function Health")
        await restored.sendDrafts(automatic: false)
        #expect(restored.drafts.isEmpty)
        let state = try await control.post("test/state", body: .object([:]))
        #expect(state["upload_ids"].arrayValue == Array(repeating: .string(shared.id.uuidString), count: 3))
        #expect(state["context"]["source_name"] == .string("Function Health"))
        await restored.logout()
    }

    @Test func unconnectedHealthDoesNotReplaceCompletedWriteStatus() async throws {
        try SessionVault.clear()
        let control = APIClient(server: URL(string: "http://localhost:18765")!, token: nil)
        _ = try await control.post("test/reset", body: .object([:]))
        let model = AppModel()
        await model.login(server: "http://localhost:18765", username: "simulator", password: "synthetic-test-password")
        let archive = try #require(model.archive)
        try archive.save(false, name: "health-enabled.json")
        model.health.status = "Reviewed glucose saved to Apple Health"
        await model.synchronizeHealth(requestAccess: false, requestSelectedAccess: false, automatic: false)
        #expect(model.health.status == "Reviewed glucose saved to Apple Health")
        #expect(!model.isBusy && !model.isHealthSyncRunning)
        let counts = try await control.post("test/counts", body: .object([:]))
        #expect(counts["health/connect"] == .null)
        await model.logout()
    }

    @Test func pausingIndependentHealthSyncPreservesProgress() async throws {
        try SessionVault.clear()
        let control = APIClient(server: URL(string: "http://localhost:18765")!, token: nil)
        _ = try await control.post("test/reset", body: .object([:]))
        let model = AppModel()
        await model.login(server: "http://localhost:18765", username: "simulator", password: "synthetic-test-password")
        let archive = try #require(model.archive)
        try archive.save(true, name: "health-enabled.json")
        let marker = Data("saved checkpoint".utf8)
        try archive.save(marker, name: "anchor-pause-test.json")
        let operation = Task { await model.synchronizeHealth(requestAccess: false, requestSelectedAccess: false, automatic: false) }
        for _ in 0..<100 {
            if model.isHealthSyncRunning { break }
            await Task.yield()
        }
        #expect(model.isHealthSyncRunning && !model.isBusy)
        #expect(model.keepHealthScreenAwake)
        model.pauseHealthSync()
        #expect(!model.keepHealthScreenAwake)
        await operation.value
        #expect(!model.isBusy && !model.health.isSyncing)
        #expect(model.errorMessage == nil)
        #expect(try archive.load(Bool.self, name: "health-auto-paused.json") == true)
        _ = try await control.post("test/reset-counts", body: .object([:]))
        let restored = AppModel()
        await restored.automaticHealthSync()
        let counts = try await control.post("test/counts", body: .object([:]))
        #expect(counts["health/connect"] == .null)
        #expect(!restored.isBusy)
        #expect(try archive.load(Data.self, name: "anchor-pause-test.json") == marker)
        await model.refresh(automatic: false)
        #expect(model.errorMessage == nil)
        await model.logout()
    }

    @Test func completedHealthSyncCooldownSurvivesRelaunchAndCoalescesAutomaticTriggers() async throws {
        try SessionVault.clear()
        let control = APIClient(server: URL(string: "http://localhost:18765")!, token: nil)
        _ = try await control.post("test/reset", body: .object([:]))
        let model = AppModel()
        await model.login(server: "http://localhost:18765", username: "simulator", password: "synthetic-test-password")
        let archive = try #require(model.archive)
        try archive.save(true, name: "health-enabled.json")
        try archive.save(false, name: "health-auto-paused.json")
        let deadline = Date().addingTimeInterval(900)
        try archive.save(deadline, name: "health-next-automatic-sync.json")
        let restored = AppModel()
        restored.health.status = "Sync complete"
        async let first: Void = restored.automaticHealthSync()
        async let second: Void = restored.automaticHealthSync()
        _ = await (first, second)
        await restored.automaticHealthSync()
        #expect(!restored.isHealthSyncRunning && !restored.isBusy)
        #expect(restored.health.status == "Sync complete")
        let counts = try await control.post("test/counts", body: .object([:]))
        #expect(counts["health/connect"] == .null)
        #expect(try archive.load(Date.self, name: "health-next-automatic-sync.json") == deadline)
        restored.pauseHealthSync()
        try archive.save(false, name: "health-enabled.json")
        await restored.logout()
    }

    @Test func manualHealthSyncBypassesCooldownAndConcurrentAutomaticRequestDoesNotRunAgain() async throws {
        try SessionVault.clear()
        let control = APIClient(server: URL(string: "http://localhost:18765")!, token: nil)
        _ = try await control.post("test/reset", body: .object([:]))
        let model = AppModel()
        await model.login(server: "http://localhost:18765", username: "simulator", password: "synthetic-test-password")
        let archive = try #require(model.archive)
        try archive.save(true, name: "health-enabled.json")
        try archive.save(false, name: "health-auto-paused.json")
        try archive.save(Date().addingTimeInterval(900), name: "health-next-automatic-sync.json")
        let manual = Task { await model.synchronizeHealth(requestAccess: false, requestSelectedAccess: false, automatic: false) }
        while !model.isHealthSyncRunning { await Task.yield() }
        #expect(!model.isBusy)
        await model.refresh(automatic: false)
        #expect(model.errorMessage == nil)
        await model.addDocument(Data("concurrent report".utf8), filename: "fixture.png", contentType: "image/png", sourceName: "Test Clinic", originalBytes: nil, originalFilename: nil, originalContentType: nil)
        #expect(model.errorMessage == nil && model.drafts.isEmpty)
        await model.automaticHealthSync()
        await manual.value
        await model.automaticHealthSync()
        let counts = try await control.post("test/counts", body: .object([:]))
        #expect(counts["health/connect"].numberValue == 1)
        #expect(!model.isBusy && !model.isHealthSyncRunning)
        let next = try #require(try archive.load(Date.self, name: "health-next-automatic-sync.json"))
        #expect(next.timeIntervalSinceNow > 850)
        model.pauseHealthSync()
        await model.logout()
    }

    @Test func initialAdminSessionBlocksBackgroundWorkUntilPasswordChange() async throws {
        try SessionVault.clear()
        let control = APIClient(server: URL(string: "http://localhost:18765")!, token: nil)
        _ = try await control.post("test/reset", body: .object([:]))
        let model = AppModel()
        await model.login(server: "http://localhost:18765", username: "admin", password: "admin")
        #expect(model.mustChangePassword)
        #expect(model.client == nil)
        let restored = AppModel()
        #expect(restored.mustChangePassword)
        await restored.refresh(automatic: false)
        await restored.sendDrafts(automatic: false)
        await restored.synchronizeHealth(requestAccess: false, requestSelectedAccess: false, automatic: false)
        let counts = try await control.post("test/counts", body: .object([:]))
        #expect(counts["reports/list"] == .null)
        #expect(counts["health/connect"] == .null)
        await restored.changePassword(current: "admin", replacement: "new-admin-password")
        #expect(restored.session == nil)
        #expect(try SessionVault.load() == nil)
        await restored.login(server: "http://localhost:18765", username: "admin", password: "new-admin-password")
        #expect(!restored.mustChangePassword && restored.client != nil)
        await restored.logout()
    }

    @Test func sessionUploadRetryCacheAndLogout() async throws {
        try SessionVault.clear()
        let control = APIClient(server: URL(string: "http://localhost:18765")!, token: nil)
        _ = try await control.post("test/reset", body: .object([:]))
        let model = AppModel()
        await model.login(server: "http://localhost:18765", username: "simulator", password: "synthetic-test-password")
        #expect(model.errorMessage == nil)
        #expect(model.session?.userID == "simulator-only")
        #expect(try SessionVault.load()?.token == "synthetic-token")
        await model.addDocument(Data(count: 20 * 1024 * 1024 + 1), filename: "oversized.png", contentType: "image/png", sourceName: "Test Clinic", originalBytes: nil, originalFilename: nil, originalContentType: nil)
        #expect(model.errorMessage != nil && model.drafts.isEmpty)
        model.errorMessage = nil
        await model.addDocument(Data(), filename: "invalid-pair.jpg", contentType: "image/jpeg", sourceName: "Test Clinic", originalBytes: Data(), originalFilename: nil, originalContentType: nil)
        #expect(model.errorMessage != nil && model.drafts.isEmpty)
        model.errorMessage = nil
        _ = try await control.post("test/fail-upload", body: .object([:]))
        await model.addDocument(Data("synthetic report".utf8), filename: "fixture.png", contentType: "image/png", sourceName: "Test Clinic", originalBytes: nil, originalFilename: nil, originalContentType: nil)
        #expect(model.drafts.count == 1)
        let draftID = try #require(model.drafts.first?.id)
        let restored = AppModel()
        #expect(restored.drafts.first?.id == draftID)
        await restored.sendDrafts(automatic: false)
        #expect(restored.errorMessage == nil)
        #expect(restored.drafts.isEmpty)
        let state = try await control.post("test/state", body: .object([:]))
        #expect(state["upload_ids"].arrayValue == [.string(draftID.uuidString), .string(draftID.uuidString)])
        #expect(restored.reports.count == 1)
        let report = try await restored.report("fixture-report")
        #expect(report["report"]["report_id"] == .string("fixture-report"))
        let source = try await restored.source("fixture-report")
        #expect(FileManager.default.fileExists(atPath: source.path))
        _ = try await control.post("test/delete-report", body: .object([:]))
        do { _ = try await restored.report("fixture-report"); Issue.record("Deleted report returned from cache") }
        catch { #expect(!FileManager.default.fileExists(atPath: source.path)) }
        await restored.refresh(automatic: false)
        #expect(restored.reports.isEmpty)
        let cache = try #require(restored.archive)
        await restored.logout()
        #expect(restored.session == nil && restored.archive == nil)
        #expect(try SessionVault.load() == nil)
        #expect(try cache.load([UploadDraft].self, name: "drafts.json") == nil)
    }
}
