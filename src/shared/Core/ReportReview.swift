import Foundation

public nonisolated enum ReportReview {
    public static func pendingChanges(in document: JSONValue) -> [JSONValue] {
        document["observations"].arrayValue.filter { $0["status"] == .string("pending") }.map {
            .object(["observation_id": $0["observation_id"], "expected_revision": $0["revision"],
                     "status": .string("confirmed"), "payload": $0["payload"]])
        }
    }
}
