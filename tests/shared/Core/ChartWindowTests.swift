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
