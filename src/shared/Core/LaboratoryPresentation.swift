import Foundation

/// Formats server-derived laboratory evidence without implementing conversion rules on the phone.
public nonisolated enum LaboratoryPresentation {
    public static func nameSortsBefore(_ name: String, id: String, _ otherName: String, otherID: String) -> Bool {
        let order = name.localizedStandardCompare(otherName)
        return order == .orderedSame ? id < otherID : order == .orderedAscending
    }

    public static func resultTitle(_ observation: JSONValue, metrics: [JSONValue], renamedMetrics: [String: String]) -> String {
        let payload = observation["payload"]
        let code = observation["lab_test_code"].stringValue
        let printed = printedName(payload["raw_name"].stringValue, code: code)
        guard let metricID = ReportReview.defaultMetricID(observation) else { return printed }
        if let name = renamedMetrics[metricID] { return name }
        let matchingMetric: (JSONValue) -> Bool = {
            $0["metric_id"] == .string(metricID) && !$0["name"].stringValue.isEmpty
        }
        if let metric = metrics.first(where: matchingMetric)
            ?? observation["metric_suggestions"].arrayValue.first(where: matchingMetric) {
            return metric["name"].stringValue
        }
        if payload["ocr_metric_candidate"] == .string(metricID) {
            let name = observation["ocr_suggestion_name"].stringValue
            if !name.isEmpty { return name }
        }
        return printed
    }

    public static func printedName(_ rawName: String, code: String) -> String {
        guard !code.isEmpty else { return rawName }
        let trimmed = rawName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.last == ")", let opening = trimmed.lastIndex(of: "("),
              trimmed[trimmed.index(after: opening)..<trimmed.index(before: trimmed.endIndex)]
                .trimmingCharacters(in: .whitespacesAndNewlines) == code else { return rawName }
        return String(trimmed[..<opening]).trimmingCharacters(in: .whitespacesAndNewlines)
    }

    public static func originalReference(_ payload: JSONValue, reference: JSONValue) -> String {
        let text = payload["reference_range"].stringValue
        guard !text.isEmpty else { return "Original reference: not provided" }
        let unit = reference["original_unit"].stringValue
        if unit.isEmpty { return "Original reference: \(text) · unit unknown" }
        let origin = reference["unit_origin"].stringValue == "reference_explicit" ? "explicit unit" : "result-column unit"
        return "Original reference: \(text) · \(unit) (\(origin))"
    }

    public static func standardizedReference(_ reference: JSONValue) -> String {
        let display = reference["display"].stringValue
        guard !display.isEmpty else {
            let reason = reference["reason"].stringValue
            return reason.isEmpty ? "Standardized reference unavailable" : "Reference retained as printed: \(reason)"
        }
        return "Standardized reference: \(display)"
    }

    public static func resultExplanation(_ interpretation: JSONValue) -> String {
        let display = interpretation["result"]["display"].stringValue
        if !display.isEmpty { return "Standardized result: \(display)" }
        return interpretation["result"]["reason"].stringValue
    }
}
