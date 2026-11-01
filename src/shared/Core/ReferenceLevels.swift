import Foundation

/// Presentation counts only. Eligibility and medical classification come from the API.
public nonisolated enum ReferenceLevels {
    public static func visibleDays(_ days: [JSONValue], start: Double, end: Double, unit: String, hiddenSources: Set<String>) -> [JSONValue] {
        days.filter {
            !hiddenSources.contains($0["source"].stringValue) && $0["unit"].stringValue == unit &&
            ($0["last_at"].numberValue ?? -.infinity) >= start && ($0["last_at"].numberValue ?? .infinity) < end
        }
    }
    public static func latestAssessment(_ days: [JSONValue]) -> JSONValue {
        days.max { ($0["last_at"].numberValue ?? 0) < ($1["last_at"].numberValue ?? 0) }?["reference_assessment"] ?? .null
    }
    public static func levels(days: [JSONValue], names: [String: String]) -> [JSONValue] {
        let groups = Dictionary(grouping: days) { "\($0["start_at"])|\($0["unit"])" }
        var output: [String: JSONValue] = [:]
        for group in groups.values {
            let ranked = group.sorted { a, b in
                let ak = a["source"].stringValue, bk = b["source"].stringValue
                let ap = OverviewSummary.devicePriority(name: names[ak] ?? ak)
                let bp = OverviewSummary.devicePriority(name: names[bk] ?? bk)
                if ap != bp { return ap < bp }
                let at = a["last_at"].numberValue ?? 0, bt = b["last_at"].numberValue ?? 0
                return at == bt ? ak < bk : at > bt
            }
            guard let day = ranked.first else { continue }
            let assessment = day["reference_assessment"]
            let rule = assessment["applicable_rule"]
            guard rule != .null else { continue }
            var bands = rule["bands"].arrayValue
            if bands.isEmpty {
                if rule["lower"] != .null {
                    bands.append(.object(["status": .string("low"), "label": .string("Below range"), "lower": .null, "upper": rule["lower"]]))
                }
                bands.append(.object(["status": .string("in_range"), "label": .string("In range"), "lower": rule["lower"], "upper": rule["upper"]]))
                if rule["upper"] != .null {
                    bands.append(.object(["status": .string("high"), "label": .string("Above range"), "lower": rule["upper"], "upper": .null]))
                }
            }
            for (index, band) in bands.enumerated() {
                let key = "\(rule["id"].stringValue)|\(band["label"].stringValue)|\(band["lower"])|\(band["upper"])"
                var entry = output[key] ?? band
                entry["id"] = .string(key)
                entry["order"] = .number(Double(index))
                entry["rule"] = rule
                entry["start_at"] = .number(min(entry["start_at"].numberValue ?? .infinity, day["start_at"].numberValue ?? 0))
                entry["end_at"] = .number(max(entry["end_at"].numberValue ?? -.infinity, day["end_at"].numberValue ?? ((day["start_at"].numberValue ?? 0) + 86400)))
                entry["days"] = .number((entry["days"].numberValue ?? 0) + (assessment["label"] == band["label"] ? 1 : 0))
                output[key] = entry
            }
        }
        return output.values.sorted {
            let a = $0["rule"]["id"].stringValue, b = $1["rule"]["id"].stringValue
            return a == b ? ($0["order"].numberValue ?? 0) > ($1["order"].numberValue ?? 0) : a < b
        }
    }
}
