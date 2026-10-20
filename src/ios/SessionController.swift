import Foundation
import Security

/// One renewal owner for every API client belonging to this login, including uploads.
@MainActor final class SessionController {
    private(set) var saved: StoredSession
    private var renewal: Task<StoredSession, Error>?
    private var retired = false

    init(_ saved: StoredSession) { self.saved = saved }

    func client() -> APIClient {
        APIClient(server: saved.server, token: saved.token, authorization: { [self] force in
            try await credential(force: force)
        })
    }

    func retire() { retired = true; renewal?.cancel(); renewal = nil }

    func setBiometricUnlock(_ enabled: Bool) throws {
        var next = saved; next.biometricUnlock = enabled
        try SessionVault.save(next); saved = next
    }

    func credential(force: Bool) async throws -> String {
        guard !retired else { throw CancellationError() }
        if let renewal { return try await renewal.value.token }
        guard let refresh = saved.refreshToken else { return saved.token } // Previously shipped sessions.
        guard force || saved.pendingRefreshToken != nil || Date().timeIntervalSince1970 >= (saved.refreshAfter ?? 0) else { return saved.token }
        var pending = saved
        if pending.pendingRefreshToken == nil {
            var bytes = [UInt8](repeating: 0, count: 32)
            guard SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) == errSecSuccess else { throw APIError.invalidResponse }
            pending.pendingRefreshToken = bytes.map { String(format: "%02x", $0) }.joined()
            try SessionVault.save(pending); saved = pending
        }
        guard let successor = pending.pendingRefreshToken else { throw APIError.invalidResponse }
        let server = saved.server
        let task = Task { @MainActor [self] in
            let response = try await APIClient(server: server, token: nil).post("session/refresh", body: .object([
                "refresh_token": .string(refresh), "next_refresh_token": .string(successor)
            ]))
            guard !retired, !Task.isCancelled else { throw CancellationError() }
            guard response["refresh_token"].stringValue == successor,
                  response["user"]["user_id"].stringValue == saved.userID,
                  !response["token"].stringValue.isEmpty,
                  let expires = response["expires_at"].numberValue,
                  let after = response["refresh_after"].numberValue else { throw APIError.invalidResponse }
            var next = saved
            next.token = response["token"].stringValue
            next.refreshToken = successor; next.pendingRefreshToken = nil
            next.expiresAt = expires; next.refreshAfter = after
            try SessionVault.save(next); saved = next
            return next
        }
        renewal = task
        defer { renewal = nil }
        return try await task.value.token
    }
}
