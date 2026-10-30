import Foundation
import UserNotifications

@MainActor enum DeviceNotifications {
    static func authorize() async throws -> Bool {
        try await UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound])
    }
    static func clearReminders() async {
        let center = UNUserNotificationCenter.current()
        let pending = await center.pendingNotificationRequests()
        center.removePendingNotificationRequests(withIdentifiers: pending.filter { $0.identifier.hasPrefix("longlivehealth-reminder:") }.map(\.identifier))
    }
    static func schedule(id: String, at: Double, title: String, body: String) async throws {
        let content = UNMutableNotificationContent()
        content.title = title; content.body = body; content.sound = .default
        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: max(1, at - Date().timeIntervalSince1970), repeats: false)
        try await UNUserNotificationCenter.current().add(UNNotificationRequest(identifier: id, content: content, trigger: trigger))
    }
    static func remove(id: String) async { UNUserNotificationCenter.current().removePendingNotificationRequests(withIdentifiers: [id]) }
}
