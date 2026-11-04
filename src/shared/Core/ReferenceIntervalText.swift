import Foundation

/// Display-only reference bounds; classification and unit conversion remain server-owned.
public nonisolated enum ReferenceIntervalText {
    public static func interval(_ rule: JSONValue, unit: String, sleepDuration: Bool) -> String {
        func bound(_ value: JSONValue) -> String {
            guard let number = value.numberValue else { return "—" }
            return sleepDuration ? OverviewSummary.duration(number) : number.formatted(.number.precision(.fractionLength(0...3)))
        }
        let suffix = sleepDuration || unit.isEmpty ? "" : " " + HistoricalValue.displayUnit(unit, kind: "")
        if rule["lower"] == .null { return "\(rule["upper_inclusive"] == .bool(false) ? "<" : "≤") \(bound(rule["upper"]))\(suffix)" }
        if rule["upper"] == .null { return "\(rule["lower_inclusive"] == .bool(false) ? ">" : "≥") \(bound(rule["lower"]))\(suffix)" }
        return "\(bound(rule["lower"])) \(rule["lower_inclusive"] == .bool(false) ? "<" : "≤") value \(rule["upper_inclusive"] == .bool(false) ? "<" : "≤") \(bound(rule["upper"]))\(suffix)"
    }
}
