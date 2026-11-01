import Foundation
#if canImport(FoundationNetworking)
import FoundationNetworking
#endif
import SkipBridge

/// Small native boundary. Business requests and authentication renewal stay in shared Swift.
enum AndroidSystem {
    static func invoke(_ operation: String, _ input: String) throws -> String {
        let bridge = try AnyDynamicObject(forStaticsOfClassName: "health.app.NativeServices")
        guard let output: String = try bridge.invoke(operation, input) else { throw APIError.invalidResponse }
        let envelope = try JSONDecoder().decode(JSONValue.self, from: Data(output.utf8))
        if let message = envelope["error"].stringValue.nonEmpty {
            throw APIError.status(400, message)
        }
        return envelope["value"].stringValue
    }

    static func perform(_ operation: String, input: JSONValue) async throws -> JSONValue {
        let payload = String(decoding: try JSONEncoder().encode(input), as: UTF8.self)
        let identifier = try invoke("start", String(decoding: try JSONEncoder().encode(JSONValue.object([
            "operation": .string(operation), "input": .string(payload)
        ])), as: UTF8.self))
        do {
            while true {
                try Task.checkCancellation()
                let result = try invoke("poll", identifier)
                if !result.isEmpty { return try JSONDecoder().decode(JSONValue.self, from: Data(result.utf8)) }
                try await Task.sleep(for: .milliseconds(100))
            }
        } catch {
            _ = try? invoke("cancel", identifier)
            throw error
        }
    }
}

private extension String { var nonEmpty: String? { isEmpty ? nil : self } }

enum SessionVault {
    static func save(_ session: StoredSession) throws {
        _ = try AndroidSystem.invoke("session.save", String(decoding: JSONEncoder().encode(session), as: UTF8.self))
    }
    static func load() throws -> StoredSession? {
        let value = try AndroidSystem.invoke("session.load", "")
        return value.isEmpty ? nil : try JSONDecoder().decode(StoredSession.self, from: Data(value.utf8))
    }
    static func clear() throws { _ = try AndroidSystem.invoke("session.clear", "") }
}

@MainActor enum MobilePlatform {
    static var protectedDataAvailable: Bool { (try? AndroidSystem.invoke("unlocked", "")) == "true" }
    static var isActive: Bool { (try? AndroidSystem.invoke("active", "")) == "true" }
    static func copy(_ value: String) { _ = try? AndroidSystem.invoke("clipboard.copy", value) }
    static let biometricLabel = "Biometric unlock"
    static let healthPlatform = "health_connect"
    static let healthProviderName = "Health Connect"
    static let unlockMessage = "Waiting for device unlock; sync will resume automatically."
    static var uploadTransport: APIClient.UploadTransport { { request, bytes, _ in
        try await URLSession.shared.upload(for: request, from: bytes)
    } }
    static func cancelUploads() throws {
        URLSession.shared.getAllTasks { tasks in tasks.forEach { $0.cancel() } }
    }
    static func clearNotifications() { _ = try? AndroidSystem.invoke("notifications.clear", "") }
    static func authenticate(reason: String, requireBiometrics: Bool) async throws -> Bool {
        try await AndroidSystem.perform("authenticate", input: .object([
            "reason": .string(reason), "biometrics": .bool(requireBiometrics)
        ]))["authenticated"].boolValue
    }
    static func refreshSuccessor() throws -> String { try AndroidSystem.invoke("random", "") }
}

@MainActor final class AppExecutionLease {
    private var identifier: String?
    func begin(name: String, expiration: @escaping @MainActor () -> Void) {
        do { identifier = try AndroidSystem.invoke("lease.begin", name) }
        catch { expiration() }
    }
    func end() {
        if let identifier { _ = try? AndroidSystem.invoke("lease.end", identifier) }
        identifier = nil
    }
}
