import Foundation

public nonisolated enum ChartPeriod: String, CaseIterable, Sendable {
    case hour = "H", day = "D", week = "W", month = "M", halfYear = "6M", year = "Y"

    public var title: String {
        switch self {
        case .hour: "Hour"
        case .day: "Day"
        case .week: "Week"
        case .month: "Month"
        case .halfYear: "Six months"
        case .year: "Year"
        }
    }

    public func interval(containing date: Date, calendar: Calendar) -> DateInterval {
        let component: Calendar.Component = switch self {
        case .hour: .hour
        case .day: .day
        case .week: .weekOfYear
        case .month, .halfYear: .month
        case .year: .year
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

public nonisolated struct ChartWindow: Equatable, Sendable {
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
        let interval = period.interval(containing: Date(timeIntervalSince1970: last), calendar: calendar)
        start = interval.start.timeIntervalSince1970
        length = interval.duration
        lower = min(first, start)
        upper = max(last + 1, interval.end.timeIntervalSince1970)
    }

    public mutating func select(_ period: ChartPeriod, calendar: Calendar) {
        let anchor = min(last, max(first, start + length - 1))
        let interval = period.interval(containing: Date(timeIntervalSince1970: anchor), calendar: calendar)
        self.period = period
        start = interval.start.timeIntervalSince1970
        length = interval.duration
        lower = min(first, start)
        upper = max(last + 1, interval.end.timeIntervalSince1970)
    }

    public mutating func move(_ direction: Int, calendar: Calendar) {
        guard direction == -1 || direction == 1 else { return }
        let component: Calendar.Component = switch period {
        case .hour: .hour
        case .day: .day
        case .week: .weekOfYear
        case .month, .halfYear: .month
        case .year: .year
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

    public static func day(_ text: String) -> Date? {
        ISO8601DateFormatter().date(from: text + "T00:00:00Z")
    }
}
