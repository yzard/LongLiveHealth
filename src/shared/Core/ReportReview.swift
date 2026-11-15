import Foundation

public nonisolated enum ReportReview {
    public static func canBulkConfirm(_ observation: JSONValue) -> Bool {
        let payload = observation["payload"]
        return observation["status"] == .string("pending")
            && observation["browse_issues"].arrayValue.isEmpty
            && !payload["source"]["quote"].stringValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && payload["ocr_mapping_note"].stringValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !payload["raw_name"].stringValue.isEmpty
            && !payload["raw_result"].stringValue.isEmpty
            && !payload["sampled_at"].stringValue.isEmpty
    }

    public static func suggestedMetricID(_ observation: JSONValue) -> String? {
        guard observation["payload"]["metric_id"].stringValue.isEmpty,
              observation["resolved_metric_id"].stringValue.isEmpty else { return nil }
        return observation["metric_suggestions"].arrayValue
            .first { !$0["metric_id"].stringValue.isEmpty }?["metric_id"].stringValue
    }

    public static func change(_ observation: JSONValue, to status: String) -> JSONValue {
        .object(["observation_id": observation["observation_id"], "expected_revision": observation["revision"],
                 "status": .string(status), "payload": observation["payload"]])
    }
}
