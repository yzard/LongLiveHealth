import Foundation

public nonisolated enum HealthSyncCoverage {
    /// Coverage belongs to an installation, not to every device signed into the account.
    public static func currentDevice(_ entries: [JSONValue], platform: String, installationID: UUID?) -> [JSONValue] {
        guard let installationID else { return [] }
        return entries.filter {
            $0["platform"].stringValue == platform &&
            UUID(uuidString: $0["installation_id"].stringValue) == installationID
        }
    }
}
