import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func trendComparisonSeparatesWindowsSourcesAndUnits() {
    let window = TrendComparison.Window(lastCompleteDay: "2026-11-03")
    #expect(window.first == "2026-09-30")
    #expect(window.recentStart == "2026-10-28")
    let dates = (1...35).map { OverviewSummary.startDate(endDate: window.last, days: $0) }
    func row(_ date: String, source: String, unit: String, value: Double) -> JSONValue {
        .object(["date": .string(date), "alternatives": .array([
            .object(["source": .string(source), "unit": .string(unit), "value": .number(value)])
        ])])
    }
    let history = dates.enumerated().map { index, date in
        row(date, source: "watch", unit: "count", value: index < 7 ? 8_000 : 6_000)
    } + [
        row(window.last, source: "watch", unit: "count", value: 8_000),
        row(window.recentStart, source: "watch", unit: "count", value: 5_000),
        row(window.last, source: "phone", unit: "count", value: 12_000),
        row(window.last, source: "watch", unit: "kcal", value: 400),
        row("2026-11-04", source: "watch", unit: "count", value: 40_000)
    ]
    let results = TrendComparison.summaries(kind: "steps", history: history, window: window)
    let watch = results.first { $0.source == "watch" && $0.unit == "count" }
    #expect(watch?.recentDays == 6)
    #expect(watch?.baselineDays == 28)
    #expect(watch?.recentMedian == 8_000)
    #expect(watch?.baselineMedian == 6_000)
    #expect(results.count == 3)
    #expect(results.filter { $0.recentMedian != nil }.count == 1)
}

@Test func trendTilesHaveUniqueKeysAcrossMetricsWithTheSameSourceAndUnit() {
    let window = TrendComparison.Window(lastCompleteDay: "2026-11-03")
    let history: [JSONValue] = [.object(["date": .string(window.last), "alternatives": .array([
        .object(["source": .string("watch"), "unit": .string("seconds"), "value": .number(3600)])
    ])])]
    let cards = ["sleep", "workout"].flatMap {
        TrendComparison.summaries(kind: $0, history: history, window: window)
    }
    #expect(cards.count == 2)
    #expect(Set(cards.map(\.id)).count == 2)
}
