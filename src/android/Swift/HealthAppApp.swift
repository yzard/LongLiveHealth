import Foundation
#if canImport(Android)
import Android
#elseif canImport(Glibc)
import Glibc
#endif
import SkipFuse
import SwiftUI

/* SKIP @bridge */public struct HealthAppRootView: View {
    /* SKIP @bridge */public init() {}
    public var body: some View { ContentView() }
}

/* SKIP @bridge */public final class HealthAppAppDelegate: Sendable {
    /* SKIP @bridge */public static let shared = HealthAppAppDelegate()
    private init() {}
    /* SKIP @bridge */public func onInit(temporaryDirectory: String) -> Bool {
        // Set the environment inside the native runtime as well: translated
        // ARM64 libraries do not share the JVM libc environment on emulators.
        setenv("TMPDIR", temporaryDirectory, 1) == 0
            && FileManager.default.temporaryDirectory.path == temporaryDirectory
    }
    /* SKIP @bridge */public func onLaunch() {}
    /* SKIP @bridge */public func onResume() {
        Task { @MainActor in
            let model = AndroidApplication.shared.model
            model.privacyCovered = false
            await model.resumeForeground()
        }
    }
    /* SKIP @bridge */public func receiveSharedPDF(path: String) async -> Bool {
        await MainActor.run { AndroidApplication.shared.model.receiveReportURL(URL(fileURLWithPath: path)) }
    }
    /* SKIP @bridge */public func sharedPDFError(message: String) {
        Task { @MainActor in AndroidApplication.shared.model.errorMessage = message }
    }
    /* SKIP @bridge */public func onPause() {
        Task { @MainActor in
            let model = AndroidApplication.shared.model
            model.privacyCovered = model.biometricUnlock
        }
    }
    /* SKIP @bridge */public func onStop() {
        Task { @MainActor in AndroidApplication.shared.model.lockForBackground() }
    }
    /* SKIP @bridge */public func pauseForSystem() {
        Task { @MainActor in AndroidApplication.shared.model.pauseHealthSync() }
    }
    /* SKIP @bridge */public func onDestroy() {}
    /* SKIP @bridge */public func onLowMemory() {}
    /* SKIP @bridge */public func syncInBackground() async -> Bool {
        await backgroundSync()
    }
    @MainActor private func backgroundSync() async -> Bool {
        let model = AndroidApplication.shared.model
        guard model.session != nil else { return true }
        await model.renewSessionIfNeeded()
        await model.sendDrafts(automatic: true)
        await model.automaticHealthSync()
        return !model.health.status.contains("paused")
    }
}
