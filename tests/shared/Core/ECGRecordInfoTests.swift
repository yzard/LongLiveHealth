import Testing
@testable import LongLiveHealthCore

@Test func ecgSourceEnumsHaveReadableLabelsWithoutInventingInterpretations() {
    #expect(ECGRecordInfo.classification(.number(1)) == "Sinus rhythm")
    #expect(ECGRecordInfo.classification(.number(2)) == "Atrial fibrillation")
    #expect(ECGRecordInfo.classification(.number(5)) == "Inconclusive — poor reading")
    #expect(ECGRecordInfo.classification(.number(100)) == "Unrecognized")
    #expect(ECGRecordInfo.classification(.number(77)) == "Not available")
    #expect(ECGRecordInfo.classification(.null) == "Not available")
    #expect(ECGRecordInfo.symptoms(.number(0)) == "Not reported")
    #expect(ECGRecordInfo.symptoms(.number(1)) == "None reported")
    #expect(ECGRecordInfo.symptoms(.number(2)) == "Reported")
    #expect(ECGRecordInfo.symptoms(.null) == "Not available")
}

@Test func heartRateUnitAliasesDoNotChangeOtherRatesOrStoredValues() {
    for kind in ["heart_rate", "resting_heart_rate", "HKQuantityTypeIdentifierHeartRate", "HKQuantityTypeIdentifierWalkingHeartRateAverage"] {
        for unit in ["count/min", "count per minute", "beats/min", "bpm"] {
            #expect(HistoricalValue.displayUnit(unit, kind: kind) == "BPM")
        }
    }
    #expect(HistoricalValue.displayUnit("count/min", kind: "respiratory_rate") == "count/min")
    #expect(HistoricalValue.displayUnit("ms", kind: "heart_rate_variability") == "ms")
    let record: JSONValue = .object(["record_type":.string("HKQuantityTypeIdentifierHeartRate"),"values":.array([.object(["value":.number(70),"unit":.string("count/min")])])])
    #expect(HistoricalValue.record(record) == "70 BPM")
    #expect(record["values"].arrayValue[0]["unit"] == .string("count/min"))
}
