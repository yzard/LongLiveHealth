import Foundation

/// Formats server-derived laboratory evidence without implementing conversion rules on the phone.
public nonisolated enum LaboratoryPresentation {
    public static func nameSortsBefore(_ name: String, id: String, _ otherName: String, otherID: String) -> Bool {
        let order = name.localizedStandardCompare(otherName)
        return order == .orderedSame ? id < otherID : order == .orderedAscending
    }

    public static func resultTitle(_ observation: JSONValue, renamedMetrics: [String: String]) -> String {
        let metricID = ReportReview.savedMetricID(observation)
        if let metricID, let name = renamedMetrics[metricID] {
            return name
        }
        let printed = observation["payload"]["raw_name"].stringValue
        let display = observation["display_name"].stringValue
        let name = metricID == nil ? (printed.isEmpty ? display : printed) : (display.isEmpty ? printed : display)
        return printedName(name, code: observation["lab_test_code"].stringValue)
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
