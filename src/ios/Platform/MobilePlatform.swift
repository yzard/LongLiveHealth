import Foundation
import UIKit
import UserNotifications
import LocalAuthentication
import Security

/// System services remain native while the application state and workflows are shared.
@MainActor enum MobilePlatform {
    static var protectedDataAvailable: Bool { UIApplication.shared.isProtectedDataAvailable }
    static var isActive: Bool { UIApplication.shared.applicationState == .active }
    static let biometricLabel = "Face ID / Touch ID unlock"
    static let healthPlatform = "apple_health"
    static let healthProviderName = "Apple Health"
    static let unlockMessage = "Waiting for iPhone unlock; sync will resume automatically."
    static var uploadTransport: APIClient.UploadTransport { BackgroundTransfer.transport }
    static func cancelUploads() throws { try BackgroundTransfer.shared.cancelAll() }
    static func clearNotifications() {
        UNUserNotificationCenter.current().removeAllPendingNotificationRequests()
        UNUserNotificationCenter.current().removeAllDeliveredNotifications()
    }
    static func authenticate(reason: String, requireBiometrics: Bool) async throws -> Bool {
        let context = LAContext()
        var failure: NSError?
        if requireBiometrics && !context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &failure) {
            throw failure ?? APIError.status(400, "Set up Face ID or Touch ID in iPhone Settings first") as NSError
        }
        return try await context.evaluatePolicy(.deviceOwnerAuthentication, localizedReason: reason)
    }
    static func refreshSuccessor() throws -> String {
        var bytes = [UInt8](repeating: 0, count: 32)
        guard SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) == errSecSuccess else { throw APIError.invalidResponse }
        return bytes.map { String(format: "%02x", $0) }.joined()
    }
}
