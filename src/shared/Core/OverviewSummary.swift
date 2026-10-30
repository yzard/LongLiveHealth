import Foundation

public nonisolated enum OverviewSummary {
    public struct SourceSummary: Identifiable, Sendable {
        public let source: String
        public let unit: String
        public let average: Double
        public let days: Int
        public let targetDays: Int?
        public var id: String { source + "|" + unit }
    }

    public static func devicePriority(name: String) -> Int {
        let text = name.lowercased()
        if text.contains("apple watch") || text.contains("applewatch") { return 0 }
        if ["iphone", "phone", "手机"].contains(where: text.contains) { return 3 }
        if ["watch", "fitbit", "garmin", "polar", "wahoo", "whoop", "oura", "huawei", "华为", "手表", "手环", "心率带", "wrist", "chest strap", "heart rate strap"].contains(where: text.contains) { return 1 }
        return 2
    }

    public static func comparison(metric: JSONValue, endDate: String, days: Int, names: [String: String], sleepTargetMinutes: Double?) -> SourceSummary? {
        let start = startDate(endDate: endDate, days: days)
        let history = metric["history"].arrayValue.filter { $0["date"].stringValue >= start && $0["date"].stringValue <= endDate }
        var coverage: [String: Int] = [:]
        for day in metric["history"].arrayValue {
            for source in day["alternatives"].arrayValue where source["value"].numberValue?.isFinite == true {
                coverage[source["source"].stringValue, default: 0] += 1
            }
        }
        let selected = history.compactMap { day -> JSONValue? in
            if metric["record_type"] == .string("steps") {
                return day["combined"]["value"].numberValue?.isFinite == true ? day["combined"] : nil
            }
            return day["alternatives"].arrayValue.filter { $0["value"].numberValue?.isFinite == true }.sorted { a, b in
                let ak = a["source"].stringValue, bk = b["source"].stringValue
                let ap = devicePriority(name: names[ak] ?? ak), bp = devicePriority(name: names[bk] ?? bk)
                if ap != bp { return ap < bp }
                if coverage[ak] != coverage[bk] { return coverage[ak, default: 0] > coverage[bk, default: 0] }
                return ak < bk
            }.first
        }
        guard !selected.isEmpty, Set(selected.map { $0["unit"].stringValue }).count == 1 else { return nil }
        let values = selected.compactMap { $0["value"].numberValue }
        let goal = metric["record_type"] == .string("sleep") ? sleepTargetMinutes : nil
        return SourceSummary(source: "", unit: selected[0]["unit"].stringValue, average: values.reduce(0, +) / Double(values.count), days: values.count, targetDays: goal.map { target in values.filter { $0 >= target * 60 }.count })
    }

    public static func duration(_ seconds: Double) -> String {
        guard seconds.isFinite, seconds >= 0, seconds < Double(Int.max / 60) else { return "Not available" }
        let minutes = Int((seconds / 60).rounded())
        return "\(minutes / 60) hr \(minutes % 60) min"
    }

    public static func value(_ value: Double, kind: String, unit: String) -> String {
        if kind == "sleep" || kind == "workout" { return duration(value) }
        let displayUnit = unit == "count" ? "" : unit == "count/min" ? "bpm" : unit
        return "\(value.formatted(.number.precision(.fractionLength(0...1)))) \(displayUnit)".trimmingCharacters(in: .whitespaces)
    }

    public static func startDate(endDate: String, days: Int) -> String {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = .gmt
        formatter.dateFormat = "yyyy-MM-dd"
        guard days > 0, let end = formatter.date(from: endDate),
              let start = formatter.calendar.date(byAdding: .day, value: 1 - days, to: end) else { return endDate }
        return formatter.string(from: start)
    }

    public static func sources(metric: JSONValue, endDate: String, days: Int, sleepTargetMinutes: Double?) -> [SourceSummary] {
        let start = startDate(endDate: endDate, days: days)
        var groups: [String: [(String, JSONValue)]] = [:]
        for day in metric["history"].arrayValue {
            let date = day["date"].stringValue
            guard date >= start, date <= endDate else { continue }
            for source in day["alternatives"].arrayValue {
                guard let value = source["value"].numberValue, value.isFinite else { continue }
                let key = source["source"].stringValue + "|" + source["unit"].stringValue
                groups[key, default: []].append((date, source))
            }
        }
        return groups.keys.sorted().compactMap { key in
            guard let records = groups[key], let first = records.first else { return nil }
            let values = records.compactMap { $0.1["value"].numberValue }
            let target = metric["record_type"] == .string("sleep") ? sleepTargetMinutes : nil
            return SourceSummary(source: first.1["source"].stringValue, unit: first.1["unit"].stringValue,
                average: values.reduce(0, +) / Double(values.count), days: Set(records.map(\.0)).count,
                targetDays: target.map { goal in values.filter { $0 >= goal * 60 }.count })
        }
    }
}
