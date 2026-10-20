import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func sleepPresentationUsesHoursMinutesAndNeverSeconds() {
    #expect(OverviewSummary.duration(27000) == "7 hr 30 min")
    #expect(OverviewSummary.duration(3599) == "1 hr 0 min")
    #expect(OverviewSummary.duration(0) == "0 hr 0 min")
    #expect(OverviewSummary.duration(.nan) == "Not available")
    #expect(OverviewSummary.duration(-1) == "Not available")
}

@Test func summariesKeepSourcesSeparateAndExcludeMissingAndOutOfWindowDays() throws {
    let metric = try JSONDecoder().decode(JSONValue.self, from: Data(#"""
    {"record_type":"sleep","history":[
      {"date":"2026-03-07","alternatives":[{"source":"Watch","value":3600,"unit":"seconds"}]},
      {"date":"2026-03-08","alternatives":[{"source":"Watch","value":25200,"unit":"seconds"}]},
      {"date":"2026-03-09","alternatives":[{"source":"Watch","value":null,"unit":"seconds"}]},
      {"date":"2026-03-14","alternatives":[{"source":"Watch","value":28800,"unit":"seconds"},{"source":"Other","value":21600,"unit":"seconds"}]},
      {"date":"2026-03-15","alternatives":[{"source":"Watch","value":3600,"unit":"seconds"}]}
    ]}
    """#.utf8))
    let week = OverviewSummary.sources(metric: metric, endDate: "2026-03-14", days: 7, sleepTargetMinutes: 450)
    let watch = try #require(week.first { $0.source == "Watch" })
    #expect(watch.average == 27000)
    #expect(watch.days == 2)
    #expect(watch.targetDays == 1)
    #expect(week.count == 2)
    let yesterday = OverviewSummary.sources(metric: metric, endDate: "2026-03-14", days: 1, sleepTargetMinutes: nil)
    #expect(yesterday.first { $0.source == "Watch" }?.average == 28800)
    #expect(yesterday.allSatisfy { $0.targetDays == nil && $0.days == 1 })
    #expect(OverviewSummary.startDate(endDate: "2024-03-02", days: 7) == "2024-02-25")
}
