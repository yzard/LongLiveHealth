import Foundation

public nonisolated enum ReportReview {
    public static func pendingChanges(in document: JSONValue) -> [JSONValue] {
        document["observations"].arrayValue.filter { $0["status"] == .string("pending") }.map {
            change($0, to: "confirmed")
        }
    }
    public static func change(_ observation: JSONValue, to status: String) -> JSONValue {
        .object(["observation_id": observation["observation_id"], "expected_revision": observation["revision"],
                 "status": .string(status), "payload": observation["payload"]])
    }
}
