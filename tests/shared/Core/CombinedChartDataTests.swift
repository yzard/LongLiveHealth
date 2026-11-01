import Testing
@testable import LongLiveHealthCore

@Test func combinedChartKeepsOverlappingMeasurementsAndSeparatesUnits() {
    let row: JSONValue = .object(["source":.string("watch"),"unit":.string("mmHg"),"start_at":.number(0),"end_at":.number(60),"last_at":.number(1),"min":.number(80),"max":.number(120)])
    let components: [JSONValue] = ["systolic", "diastolic"].map { id in
        .object(["id":.string(id),"unit":.string("mmHg"),"chart_ranges":.object(["D":.array([row])])])
    } + [.object(["id":.string("pulse"),"unit":.string("count/min"),"chart_ranges":.object(["D":.array([row])])])]
    let response: JSONValue = .object(["components":.array(components)])
    let ranges = CombinedChartData.ranges(response, unit: "mmHg")
    #expect(ranges["D"].arrayValue.count == 2)
    #expect(CombinedChartData.units(response).count == 2)
    let segments = ChartRangeSegments.segments(rows: ranges["D"].arrayValue, names: ["watch":"Apple Watch"])
    #expect(segments.count == 2)
    #expect(Set(segments.map { $0["component"].stringValue }).count == 2)
}

@Test func ecgRetainsVoltageAndBreaksMissingSamples() {
    let samples: [JSONValue] = [0.0, 0.002, 1.0, 1.002].map {
        .object(["time_since_start":.number($0),"lead_i_volts":.number(0.001)])
    }
    var trace = ECGWaveform.Trace(frequency: 500)
    trace.append(samples)
    let segments = trace.segments
    #expect(segments.count == 2)
    #expect(segments[0][0].millivolts == 1)
    #expect(segments[1][0].time == 1)
}
