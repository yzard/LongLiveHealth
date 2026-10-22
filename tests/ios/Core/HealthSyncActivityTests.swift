import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func replayProgressAndConcurrentReadRemainVisible() throws {
    let start = Date(timeIntervalSince1970: 100)
    var activity = HealthSyncActivity()
    activity.readingSince = start
    activity.beginSending(records: 1000, at: start)
    let lines = activity.lines(at: start.addingTimeInterval(8))
    #expect(lines.contains { $0.contains("Reading") && $0.contains("8s") })
    #expect(lines.contains { $0.contains("Sending") && $0.contains("8s") })
    let reply: JSONValue = .object(["total_records": .number(4000000)])
    for _ in 0..<2 {
        activity.beginSending(records: 1000, at: start)
        #expect(try HealthSyncProgress.totalRecords(in: reply) == 4000000)
        activity.confirmSending()
        activity.endSending()
    }
    #expect(activity.confirmedRecords == 2000)
    #expect(activity.readingSince == start)
    activity.beginSending(records: 1000, at: start)
    activity.endSending() // Failed or cancelled upload must not advance progress.
    #expect(activity.confirmedRecords == 2000)
    #expect(activity.sendingSince == nil)
    #expect(HealthSyncActivity().confirmedRecords == 0)
}
