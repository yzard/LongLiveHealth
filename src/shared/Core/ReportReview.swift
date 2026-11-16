import Foundation

public nonisolated enum ReportReview {
    public static func canBulkConfirm(_ observation: JSONValue) -> Bool {
        observation["status"] == .string("pending")
            && hasReviewableMapping(observation)
            && observation["browse_issues"].arrayValue.isEmpty
            && hasRequiredEvidence(observation["payload"])
    }

    private static func hasRequiredEvidence(_ payload: JSONValue) -> Bool {
        !payload["source"]["quote"].stringValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !payload["raw_name"].stringValue.isEmpty
            && !payload["raw_result"].stringValue.isEmpty
            && !payload["sampled_at"].stringValue.isEmpty
    }

    private static func hasReviewableMapping(_ observation: JSONValue) -> Bool {
        if hasSavedMapping(observation) { return true }
        // OCR suggestions have their own fields. A server-resolved ID is a verified
        // match even when the pending payload has no explicit reviewer selection.
        let payload = observation["payload"]
        return payload["metric_id"].stringValue.isEmpty
            && payload["unmapped_choice"] == .null
            && !observation["resolved_metric_id"].stringValue.isEmpty
    }

    private static func hasSavedMapping(_ observation: JSONValue) -> Bool {
        let payload = observation["payload"]
        let mapping = payload["metric_id"].stringValue
        return (!mapping.isEmpty && observation["resolved_metric_id"] == .string(mapping))
            || payload["unmapped_choice"]["kind"] == .string("new")
    }

    public static func canConfirmIndividually(_ observation: JSONValue) -> Bool {
        if canBulkConfirm(observation) { return true }
        let payload = observation["payload"]
        let choice = payload["unmapped_choice"]
        if choice == .null { return hasSavedMapping(observation) }
        return observation["status"] == .string("pending")
            && choice["kind"] == .string("new")
            && hasRequiredEvidence(payload)
            && observation["browse_issues"].arrayValue.allSatisfy { $0["code"] == .string("negative_with_unit") }
    }

    public static func suggestedMetricID(_ observation: JSONValue) -> String? {
        guard observation["payload"]["metric_id"].stringValue.isEmpty,
              observation["payload"]["unmapped_choice"] == .null,
              observation["resolved_metric_id"].stringValue.isEmpty else { return nil }
        return observation["metric_suggestions"].arrayValue
            .first { !$0["metric_id"].stringValue.isEmpty }?["metric_id"].stringValue
    }

    public static func savedMetricID(_ observation: JSONValue) -> String? {
        let selected = observation["payload"]["metric_id"].stringValue
        if !selected.isEmpty { return selected }
        guard observation["status"] == .string("confirmed") else { return nil }
        let resolved = observation["resolved_metric_id"].stringValue
        return resolved.isEmpty ? nil : resolved
    }

    public static func defaultMetricID(_ observation: JSONValue) -> String? {
        guard observation["payload"]["unmapped_choice"] == .null else { return nil }
        if let saved = savedMetricID(observation) { return saved }
        let resolved = observation["resolved_metric_id"].stringValue
        return resolved.isEmpty ? suggestedMetricID(observation) : resolved
    }

    public static func suggestionNote(_ observation: JSONValue) -> String {
        let note = observation["payload"]["ocr_mapping_note"].stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        return note.isEmpty ? observation["ocr_suggestion_name"].stringValue : note
    }

    public static func change(_ observation: JSONValue, to status: String) -> JSONValue {
        .object(["observation_id": observation["observation_id"], "expected_revision": observation["revision"],
                 "status": .string(status), "payload": observation["payload"]])
    }
}
