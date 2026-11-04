import Foundation

public nonisolated enum TrendComparison {
    public struct Window: Sendable {
        public static let recentDays = 7
        public static let baselineDays = 28
        public let first: String
        public let recentStart: String
        public let last: String

        public init(lastCompleteDay: String) {
            last = lastCompleteDay
            recentStart = OverviewSummary.startDate(endDate: last, days: Self.recentDays)
            first = OverviewSummary.startDate(endDate: last, days: Self.recentDays + Self.baselineDays)
        }
    }

    public struct Summary: Identifiable, Sendable {
        public let kind: String
        public let source: String
        public let unit: String
        public let recentDays: Int
        public let baselineDays: Int
        public let recentMedian: Double?
        public let baselineMedian: Double?
        public var id: String { kind + "|" + source + "|" + unit }
    }

    public static func summaries(kind: String, history: [JSONValue], window: Window) -> [Summary] {
        var series: [String: [String: Set<Double>]] = [:]
        var identities: [String: (source: String, unit: String)] = [:]
        for day in history {
            let date = day["date"].stringValue
            guard date >= window.first && date <= window.last else { continue }
            for reading in day["alternatives"].arrayValue {
                let source = reading["source"].stringValue
                let unit = reading["unit"].stringValue
                guard !source.isEmpty, !unit.isEmpty, let value = reading["value"].numberValue, value.isFinite, value >= 0 else { continue }
                let key = source + "|" + unit
                identities[key] = (source, unit)
                series[key, default: [:]][date, default: []].insert(value)
            }
        }
        return series.keys.sorted().compactMap { key in
            guard let identity = identities[key], let days = series[key] else { return nil }
            // Identical duplicates count once. Conflicting duplicates contribute no value.
            let recent = days.filter { $0.key >= window.recentStart }.values.compactMap { $0.count == 1 ? $0.first : nil }
            let baseline = days.filter { $0.key < window.recentStart }.values.compactMap { $0.count == 1 ? $0.first : nil }
            guard !recent.isEmpty || !baseline.isEmpty else { return nil }
            let comparable = recent.count >= 4 && baseline.count >= 14
            return Summary(kind: kind, source: identity.source, unit: identity.unit, recentDays: recent.count, baselineDays: baseline.count,
                           recentMedian: comparable ? median(recent) : nil, baselineMedian: comparable ? median(baseline) : nil)
        }
    }

    private static func median(_ values: [Double]) -> Double {
        let sorted = values.sorted()
        let middle = sorted.count / 2
        return sorted.count.isMultiple(of: 2) ? sorted[middle - 1] / 2 + sorted[middle] / 2 : sorted[middle]
    }
}
