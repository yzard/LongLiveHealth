import Testing
@testable import LongLiveHealthCore

@Test func historicalValuesKeepDatesAndMissingValuesHonest() {
    #expect(HistoricalValue.date(timestamp: nil, text: "2026-10-31") == "2026-10-31 —")
    #expect(HistoricalValue.date(timestamp: nil, text: "not a date") == "—")
    let timestamp = HistoricalValue.date(timestamp: 1_793_440_800, text: "")
    #expect(timestamp.count == 19)
    #expect(timestamp.hasSuffix("AM") || timestamp.hasSuffix("PM"))
    #expect(HistoricalValue.quantity(.null, unit: "mg/dL") == "—")
    let pair: JSONValue = .object(["record_type":.string("HKCorrelationTypeIdentifierBloodPressure"),"values":.array([
        .object(["id":.string("blood_pressure_systolic"),"value":.number(120)]),
        .object(["id":.string("blood_pressure_diastolic"),"value":.number(80)])])])
    #expect(HistoricalValue.record(pair) == "120 / 80 mmHg")
}

@Test func historyOrdersMixedDatesNewestFirstAndKeepsUndatedRecordsLast() {
    let rows: [JSONValue] = [
        .object(["id":.string("missing")]),
        .object(["id":.string("old"),"date":.string("2025-12-31")]),
        .object(["id":.string("early"),"date":.string("2026-10-31T08:00:00-04:00")]),
        .object(["id":.string("new"),"date":.string("2026-10-31T13:00:00Z")]),
        .object(["id":.string("invalid"),"date":.string("invalid")])
    ]
    let sorted=HistoricalValue.newestFirst(rows,timestamps:["at"],dates:["date"])
    #expect(sorted.map { $0["id"].stringValue } == ["new","early","old","missing","invalid"])
    let numeric=HistoricalValue.newestFirst([.object(["at":.number(10)]),.object(["at":.number(20)])],timestamps:["at"],dates:[])
    #expect(numeric.first?["at"] == .number(20))
}

@Test func dailyHistoriesFlattenSourcesWithoutReusingLazyListKeys() {
    let history: [JSONValue] = [
        .object(["date":.string("2026-10-30"),"alternatives":.array([.object(["source":.string("watch")]),.object(["source":.string("phone")])])]),
        .object(["date":.string("2026-10-31"),"alternatives":.array([.object(["source":.string("watch")])])])
    ]
    let rows = HistoricalValue.dailyRows(history)
    #expect(rows.count == 3)
    #expect(Set(rows.map(\.id)).count == 3)
    #expect(rows.map { $0.value["date"].stringValue } == ["2026-10-31","2026-10-30","2026-10-30"])
    #expect(rows.map { $0.value["source"].stringValue } == ["watch","watch","phone"])
    let first=HistoricalValue.keyedRows([.null],namespace:"first-session")
    let second=HistoricalValue.keyedRows([.null],namespace:"second-session")
    #expect(first.first?.id != second.first?.id)
}

@Test func sleepSessionHistoryIsGlobalAcrossSources() {
    let rows = HistoricalValue.sleepSessions([
        .object(["source":.string("first"),"sessions":.array([.object(["end_at":.number(100)]),.object(["end_at":.number(300)])])]),
        .object(["source":.string("second"),"sessions":.array([.object(["end_at":.number(200)])])])
    ])
    #expect(rows.map { $0.value["end_at"].numberValue } == [300,200,100])
    #expect(rows.map { $0.value["source"].stringValue } == ["first","second","first"])
    #expect(Set(rows.map(\.id)).count == 3)
}
