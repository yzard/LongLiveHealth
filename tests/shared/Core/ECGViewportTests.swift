import Testing
@testable import LongLiveHealthCore

@Test func ecgPinchKeepsAnchorAndPanCannotLeaveRecording() {
    let full = ECGWaveform.Window(start: 0, end: 30)
    let zoomed = full.zoom(3, anchor: 0.25, bounds: full)
    #expect(zoomed.start == 5)
    #expect(zoomed.end == 15)
    #expect(zoomed.pan(fraction: 10, bounds: full).start == 0)
    #expect(zoomed.pan(fraction: -10, bounds: full).end == 30)
    #expect(zoomed.zoom(0.01, anchor: 0.5, bounds: full) == full)
    #expect(zoomed.zoom(.nan, anchor: 0.5, bounds: full) == zoomed)
    #expect(zoomed.zoom(10000, anchor: 0.5, bounds: full).span >= 0.01999)
}

@Test func ecgAutomaticPageAppendKeepsContinuityAndLatePeaks() {
    func sample(_ index: Int) -> JSONValue {
        .object(["time_since_start":.number(Double(index)/500),"lead_i_volts":.number(index == 9001 ? 0.003 : -0.001)])
    }
    var trace = ECGWaveform.Trace(frequency: 500)
    trace.append((0..<5000).map(sample)); trace.append((5000..<10000).map(sample))
    #expect(trace.count == 10000)
    #expect(trace.segments.count == 1)
    #expect(trace.last == 19.998)
    let full = ECGWaveform.Window(start: trace.first, end: trace.last)
    let reduced = trace.visibleSegments(in: full, columns: 100).flatMap { $0 }
    #expect(reduced.count <= 404)
    #expect(reduced.contains { $0.millivolts == 3 })
    #expect(reduced.first?.time == 0)
    #expect(reduced.last?.time == 19.998)
    let later = trace.visibleSegments(in: .init(start: 18, end: 19), columns: 200).flatMap { $0 }
    #expect(later.contains { $0.time == 18.002 && $0.millivolts == 3 })
    #expect(later.first!.time <= 18)
    #expect(later.last!.time >= 19)
}

@Test func ecgDoesNotConnectMalformedOrMissingSamplesAcrossPages() {
    var trace = ECGWaveform.Trace(frequency: 500)
    trace.append([.object(["time_since_start":.number(0),"lead_i_volts":.number(0.001)]),.null])
    trace.append([.object(["time_since_start":.number(0.002),"lead_i_volts":.number(0.002)])])
    #expect(trace.segments.count == 2)
    #expect(trace.visibleSegments(in: .init(start: 50,end: 51), columns: 50).isEmpty)
    #expect(ECGWaveform.Trace(frequency: nil).count == 0)
}
