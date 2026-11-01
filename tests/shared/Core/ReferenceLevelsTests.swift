import Testing
@testable import LongLiveHealthCore

@Test func referenceLevelsCountDaysNotSourcesAndSeparateEqualStatusBands() {
    let bands: [JSONValue] = [
        .object(["status": .string("high"), "label": .string("Overweight"), "lower": .number(25), "upper": .number(30)]),
        .object(["status": .string("high"), "label": .string("Obesity class 1"), "lower": .number(30), "upper": .number(35)])
    ]
    let rule: JSONValue = .object(["id": .string("adult"), "bands": .array(bands)])
    func day(_ source: String, _ start: Double, _ label: String) -> JSONValue {
        .object(["source": .string(source), "start_at": .number(start), "last_at": .number(start + 100), "unit": .string("count"), "reference_assessment": .object(["status": .string("high"), "label": .string(label), "applicable_rule": rule])])
    }
    let days = [day("watch", 0, "Overweight"), day("phone", 0, "Obesity class 1"), day("watch", 86400, "Obesity class 1")]
    let levels = ReferenceLevels.levels(days: days, names: ["watch": "Apple Watch", "phone": "iPhone"])
    #expect(levels.count == 2)
    #expect(levels.allSatisfy { $0["days"] == .number(1) })
    #expect(ReferenceLevels.levels(days: [], names: [:]).isEmpty)
}

@Test func changingHeightKeepsDistinctWeightBands() {
    func day(_ start: Double, _ upper: Double) -> JSONValue {
        .object(["source": .string("scale"), "start_at": .number(start), "last_at": .number(start + 1), "unit": .string("kg"), "reference_assessment": .object(["label": .string("In range"), "status": .string("in_range"), "applicable_rule": .object(["id": .string("weight"), "lower": .number(50), "upper": .number(upper), "bands": .array([])])])])
    }
    let result = ReferenceLevels.levels(days: [day(0, 70), day(86400, 80)], names: [:])
    let normal = result.filter { $0["status"] == .string("in_range") }
    #expect(normal.count == 2)
    #expect(normal.allSatisfy { $0["days"] == .number(1) })
}

@Test func componentReferenceLevelsUseOnlyVisiblePeriodAndLatestTime() {
    let rows: [JSONValue] = [
        .object(["source":.string("watch"),"unit":.string("mmHg"),"last_at":.number(100),"reference_assessment":.object(["label":.string("old")])]),
        .object(["source":.string("watch"),"unit":.string("mmHg"),"last_at":.number(200),"reference_assessment":.object(["label":.string("latest")])]),
        .object(["source":.string("phone"),"unit":.string("mmHg"),"last_at":.number(250)]),
        .object(["source":.string("watch"),"unit":.string("kPa"),"last_at":.number(240)])
    ]
    let visible = ReferenceLevels.visibleDays(rows, start:150,end:250,unit:"mmHg",hiddenSources:["phone"])
    #expect(visible.count == 1)
    #expect(ReferenceLevels.latestAssessment(Array(rows.prefix(2).reversed()))["label"] == .string("latest"))
    #expect(ReferenceLevels.latestAssessment(visible)["label"] == .string("latest"))
}

@Test func referenceDurationUsesHoursWhileLaboratorySecondsRemainSeconds() {
    let sleep: JSONValue = .object(["lower":.number(25200),"upper":.number(34200)])
    #expect(ReferenceIntervalText.interval(sleep,unit:"s",sleepDuration:true) == "7 hr 0 min ≤ value ≤ 9 hr 30 min")
    #expect(ReferenceIntervalText.interval(sleep,unit:"seconds",sleepDuration:true) == "7 hr 0 min ≤ value ≤ 9 hr 30 min")
    let clotting: JSONValue = .object(["lower":.number(11),"upper":.number(13.5)])
    #expect(ReferenceIntervalText.interval(clotting,unit:"s",sleepDuration:false) == "11 ≤ value ≤ 13.5 s")
}
