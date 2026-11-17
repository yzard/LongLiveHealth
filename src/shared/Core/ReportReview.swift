import Foundation

public nonisolated enum ReportReview {
    public static func canBulkConfirm(_ observation: JSONValue) -> Bool {
        observation["status"] == .string("pending")
            && !observation["payload"]["ocr_mapping_note"].stringValue.contains("Previously excluded. Review before confirming.")
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

    public static func suggestedMetricID(_ observation: JSONValue) -> String? {
        guard observation["payload"]["metric_id"].stringValue.isEmpty,
              observation["payload"]["unmapped_choice"] == .null,
              observation["resolved_metric_id"].stringValue.isEmpty else { return nil }
        let candidate = observation["payload"]["ocr_metric_candidate"].stringValue
        guard !candidate.isEmpty, observation["metric_suggestions"].arrayValue.contains(where: {
            $0["metric_id"] == .string(candidate)
        }) else { return nil }
        return candidate
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
        let selected = observation["payload"]["metric_id"].stringValue
        if !selected.isEmpty { return selected }
        let resolved = observation["resolved_metric_id"].stringValue
        return resolved.isEmpty ? suggestedMetricID(observation) : resolved
    }

    public static func suggestionNote(_ observation: JSONValue, includeMetricName: Bool) -> String {
        let note = observation["payload"]["ocr_mapping_note"].stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        if observation["browse_issues"].arrayValue.contains(where: {
            $0["code"] == .string("inconsistent_subcount")
                && ($0["message"] == .string(note) || note == "The printed subcount exceeds its total. Verify the unit and scale.")
        }) {
            return ""
        }
        if !includeMetricName { return note }
        return note.isEmpty ? observation["ocr_suggestion_name"].stringValue : note
    }

    public static func change(_ observation: JSONValue, to status: String) -> JSONValue {
        .object(["observation_id": observation["observation_id"], "expected_revision": observation["revision"],
                 "status": .string(status), "payload": observation["payload"]])
    }
}
