import Foundation

@MainActor enum DeviceNotifications {
    static func authorize() async throws -> Bool {
        try await AndroidSystem.perform("notifications.authorize", input: .object([:]))["granted"].boolValue
    }
    static func clearReminders() async { _ = try? await AndroidSystem.perform("notifications.clearReminders", input: .object([:])) }
    static func schedule(id: String, at: Double, title: String, body: String) async throws {
        _ = try await AndroidSystem.perform("notifications.schedule", input: .object([
            "id": .string(id), "at": .number(at), "title": .string(title), "body": .string(body)
        ]))
    }
    static func remove(id: String) async { _ = try? await AndroidSystem.perform("notifications.remove", input: .object(["id": .string(id)])) }
}
