import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func chartPeriodsUseCalendarMonthsAndLeapYears() throws {
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = .gmt
    let date = try #require(ChartWindow.day("2024-02-20"))
    var window = ChartWindow(timestamps: [date.timeIntervalSince1970], period: .month, calendar: calendar)
    #expect(window.length == 29 * 86400)
    window.select(.year, calendar: calendar)
    #expect(window.length == 366 * 86400)
    #expect(window.lower <= window.start)
    #expect(window.upper >= window.start + window.length)
    window.select(.halfYear, calendar: calendar)
    #expect(Date(timeIntervalSince1970: window.start) == ChartWindow.day("2024-01-01"))
    #expect(Date(timeIntervalSince1970: window.start + window.length) == ChartWindow.day("2024-07-01"))
}

@Test func chartDayRespectsDSTAndEmptySamplesStayFinite() throws {
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = try #require(TimeZone(identifier: "America/New_York"))
    let date = try #require(ISO8601DateFormatter().date(from: "2026-03-08T16:00:00Z"))
    let window = ChartWindow(timestamps: [date.timeIntervalSince1970], period: .day, calendar: calendar)
    #expect(window.length == 23 * 3600)
    #expect(ChartWindow.day("not-a-date") == nil)
    for timestamps: [Double] in [[], [123], [.nan, .infinity]] {
        for period in ChartPeriod.allCases {
            let value = ChartWindow(timestamps: timestamps, period: period, calendar: calendar)
            #expect(value.length > 0 && value.length.isFinite)
            #expect(value.start >= value.lower && value.start + value.length <= value.upper)
        }
    }
}

@Test func chartPagingKeepsCalendarWindowsAndStopsAtAvailableHistory() throws {
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = .gmt
    let first = try #require(ChartWindow.day("2000-01-01"))
    let last = try #require(ChartWindow.day("2024-03-20"))
    var window = ChartWindow(timestamps: [first.timeIntervalSince1970, last.timeIntervalSince1970], period: .month, calendar: calendar)
    window.move(-1, calendar: calendar)
    #expect(window.length == 29 * 86400)
    #expect(Date(timeIntervalSince1970: window.start) == ChartWindow.day("2024-02-01"))
    window.move(1, calendar: calendar)
    let latest = window
    window.move(1, calendar: calendar)
    #expect(window == latest)
    window.select(.year, calendar: calendar)
    #expect(window.length == 366 * 86400)
}

@Test func allHistoryKeepsFractionalEndpointsAndDoesNotPage() {
    let calendar = Calendar(identifier: .gregorian)
    let timestamps = [1_577_836_800.25, 1_767_225_600.75]
    var window = ChartWindow(timestamps: timestamps, period: .day, calendar: calendar)
    window.move(-1, calendar: calendar)
    window.select(.all, calendar: calendar)
    #expect(window.start == timestamps[0])
    #expect(window.start + window.length == timestamps[1] + 1)
    let archive = window
    window.move(-1, calendar: calendar)
    window.move(1, calendar: calendar)
    #expect(window == archive)
    window.select(.day, calendar: calendar)
    #expect(window.start <= timestamps[1] && window.start + window.length > timestamps[1])
}

@Test func allHistoryAxisUsesCalendarYearBoundaries() throws {
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = .gmt
    let start = try #require(ChartWindow.day("2018-07-14"))
    let end = try #require(ChartWindow.day("2026-11-19"))
    let dates = ChartWindow.historyYearTicks(from: start.timeIntervalSince1970, to: end.timeIntervalSince1970, calendar: calendar)
    #expect(dates == [start] + ["2020-01-01", "2022-01-01", "2024-01-01", "2026-01-01"].compactMap(ChartWindow.day))
}

@Test func allHistoryAxisLabelsShortArchivesAndBothYearsAcrossNewYear() throws {
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = try #require(TimeZone(identifier: "America/New_York"))
    let first = try #require(calendar.date(from: DateComponents(year: 2025, month: 12, day: 31, hour: 23)))
    let newYear = try #require(calendar.date(from: DateComponents(year: 2026, month: 1, day: 1)))
    let sameYear = ChartWindow.historyYearTicks(from: first.timeIntervalSince1970, to: newYear.timeIntervalSince1970, calendar: calendar)
    #expect(sameYear == [first])
    let crossing = ChartWindow.historyYearTicks(from: first.timeIntervalSince1970, to: newYear.timeIntervalSince1970 + 86400, calendar: calendar)
    #expect(crossing.map { calendar.component(.year, from: $0) } == [2025, 2026])
    #expect(crossing[1] > newYear)
}
