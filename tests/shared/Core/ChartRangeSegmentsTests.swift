import Testing
@testable import LongLiveHealthCore

private func range(_ source: String, _ low: Double, _ high: Double, last: Double) -> JSONValue {
    .object(["source": .string(source), "unit": .string("unit"), "start_at": .number(0), "end_at": .number(100), "last_at": .number(last), "min": .number(low), "max": .number(high)])
}

@Test func sharedColumnPreservesDisjointOldAndNewWatchRanges() {
    let rows = [range("new", 38, 41, last: 90), range("old", 35, 36, last: 10)]
    let segments = ChartRangeSegments.segments(rows: rows, names: ["new": "Apple Watch", "old": "Apple Watch"])
    #expect(segments.count == 2)
    #expect(segments.map { $0["start_at"] } == [.number(0), .number(0)])
    #expect(segments.map { $0["display_min"] } == [.number(35), .number(38)])
    #expect(segments.map { $0["display_max"] } == [.number(36), .number(41)])
}

@Test func overlapUsesDevicePriorityAndRetainsOtherColorOutsideOverlap() {
    let watch = range("watch", 32, 38, last: 10)
    let phone = range("phone", 35, 40, last: 90)
    let names = ["watch": "Apple Watch", "phone": "iPhone"]
    let segments = ChartRangeSegments.segments(rows: [phone, watch], names: names)
    #expect(segments.count == 2)
    #expect(segments[0]["source"] == .string("watch"))
    #expect(segments[0]["display_max"] == .number(38))
    #expect(segments[1]["display_min"] == .number(38))
    #expect(segments[1]["max"] == .number(40))
    #expect(ChartRangeSegments.segments(rows: [phone], names: names)[0]["display_min"] == .number(35))
    let sameLevel = ChartRangeSegments.segments(rows: [watch, range("new", 35, 40, last: 90)], names: ["watch": "Apple Watch", "new": "Apple Watch"])
    #expect(sameLevel[0]["display_max"] == .number(35))
    #expect(sameLevel[1]["source"] == .string("new"))
}

@Test func distinctBucketsAndSinglePointsDoNotInventConnectingRanges() {
    let point = range("watch", 36, 36, last: 10)
    var next = range("watch", 38, 40, last: 120)
    next["start_at"] = .number(100); next["end_at"] = .number(200)
    let segments = ChartRangeSegments.segments(rows: [point, next], names: ["watch": "Apple Watch"])
    #expect(segments.count == 2)
    #expect(segments[0]["display_min"] == segments[0]["display_max"])
    #expect(segments[1]["start_at"] == .number(100))
    #expect(ChartRangeSegments.segments(rows: [], names: [:]).isEmpty)
}
