import Foundation

public nonisolated enum MetricPresentation {
    public static func devicePriority(name: String) -> Int {
        let text = name.lowercased()
        if text.contains("apple watch") || text.contains("applewatch") { return 0 }
        if ["iphone", "phone", "手机"].contains(where: text.contains) { return 3 }
        if ["watch", "fitbit", "garmin", "polar", "wahoo", "whoop", "oura", "huawei", "华为", "手表", "手环", "心率带", "wrist", "chest strap", "heart rate strap"].contains(where: text.contains) { return 1 }
        return 2
    }

    public static func duration(_ seconds: Double) -> String {
        guard seconds.isFinite, seconds >= 0, seconds < Double(Int.max / 60) else { return "Not available" }
        let minutes = Int((seconds / 60).rounded())
        return "\(minutes / 60) hr \(minutes % 60) min"
    }

    public static func value(_ value: Double, kind: String, unit: String) -> String {
        if kind == "sleep" || kind == "workout" { return duration(value) }
        let displayUnit = unit == "count" ? "" : HistoricalValue.displayUnit(unit, kind: kind)
        return "\(value.formatted(.number.precision(.fractionLength(0...1)))) \(displayUnit)".trimmingCharacters(in: .whitespaces)
    }

}
