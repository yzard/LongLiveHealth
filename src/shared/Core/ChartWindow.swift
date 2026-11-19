import Foundation

public nonisolated enum ChartPeriod: String, CaseIterable, Sendable {
    case hour = "H", day = "D", week = "W", month = "M", halfYear = "6M", year = "Y", all = "All"

    public var title: String {
        switch self {
        case .hour: "Hour"
        case .day: "Day"
        case .week: "Week"
        case .month: "Month"
        case .halfYear: "Six months"
        case .year: "Year"
        case .all: "All history"
        }
    }

    public func interval(containing date: Date, calendar: Calendar) -> DateInterval {
        precondition(self != .all, "All history requires archive bounds")
        let component: Calendar.Component = switch self {
        case .hour: .hour
        case .day: .day
        case .week: .weekOfYear
        case .month, .halfYear: .month
        case .year, .all: .year
        }
        let interval = calendar.dateInterval(of: component, for: date) ?? DateInterval(start: date, duration: 86400)
        if self == .halfYear {
            let year = calendar.component(.year, from: date)
            let month = calendar.component(.month, from: date) <= 6 ? 1 : 7
            if let start = calendar.date(from: DateComponents(year: year, month: month, day: 1)),
               let end = calendar.date(byAdding: .month, value: 6, to: start) {
                return DateInterval(start: start, end: end)
            }
        }
        return interval
    }
}

public nonisolated struct ChartWindow: Equatable, Hashable, Sendable {
    public private(set) var lower: Double
    public private(set) var upper: Double
    public private(set) var length: Double
    public private(set) var period: ChartPeriod
    public var start: Double
    private let first: Double
    private let last: Double

    public init(timestamps: [Double], period: ChartPeriod, calendar: Calendar) {
        let valid = timestamps.filter { $0.isFinite && abs($0) < 100_000_000_000 }
        first = valid.min() ?? Date().timeIntervalSince1970
        last = valid.max() ?? first
        self.period = period
        start = first
        length = last - first + 1
        lower = first
        upper = last + 1
        apply(period, anchor: last, calendar: calendar)
    }

    public mutating func select(_ period: ChartPeriod, calendar: Calendar) {
        let anchor = min(last, max(first, start + length - 1))
        apply(period, anchor: anchor, calendar: calendar)
    }

    private mutating func apply(_ period: ChartPeriod, anchor: Double, calendar: Calendar) {
        self.period = period
        if period == .all {
            start = first
            length = last - first + 1
            lower = first
            upper = last + 1
            return
        }
        let interval = period.interval(containing: Date(timeIntervalSince1970: anchor), calendar: calendar)
        start = interval.start.timeIntervalSince1970
        length = interval.duration
        lower = min(first, start)
        upper = max(last + 1, interval.end.timeIntervalSince1970)
    }

    public mutating func move(_ direction: Int, calendar: Calendar) {
        guard period != .all, direction == -1 || direction == 1 else { return }
        let component: Calendar.Component = switch period {
        case .hour: .hour
        case .day: .day
        case .week: .weekOfYear
        case .month, .halfYear: .month
        case .year, .all: .year
        }
        let amount = direction * (period == .halfYear ? 6 : 1)
        guard let from = calendar.date(byAdding: component, value: amount, to: Date(timeIntervalSince1970: start)),
              let to = calendar.date(byAdding: component, value: amount, to: Date(timeIntervalSince1970: start + length)),
              from.timeIntervalSince1970 <= last, to.timeIntervalSince1970 > first else { return }
        start = from.timeIntervalSince1970
        length = to.timeIntervalSince(from)
        lower = min(lower, start)
        upper = max(upper, start + length)
    }

    public static func historyYearTicks(from start: Double, to end: Double, calendar: Calendar) -> [Date] {
        let first = Date(timeIntervalSince1970: start)
        let last = Date(timeIntervalSince1970: end.nextDown)
        let firstYear = calendar.component(.year, from: first)
        let lastYear = calendar.component(.year, from: last)
        guard firstYear < lastYear else { return [first] }

        let step = (lastYear - firstYear + 3) / 4
        let firstBoundary = lastYear - ((lastYear - firstYear - 1) / step) * step
        let spacing = (end - start) / 9
        var ticks = [first]
        for year in stride(from: firstBoundary, through: lastYear, by: step) {
            guard let date = calendar.date(from: DateComponents(year: year, month: 1, day: 1)),
                  date.timeIntervalSince(ticks[ticks.count - 1]) >= spacing else { continue }
            ticks.append(date)
        }
        if ticks.count == 1 { ticks.append(last) }
        return ticks
    }

    public static func day(_ text: String) -> Date? {
        ISO8601DateFormatter().date(from: text + "T00:00:00Z")
    }
}
