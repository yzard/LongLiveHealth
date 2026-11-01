import SwiftUI

/// Shared time-domain viewport and gestures; original sample voltages remain unchanged.
struct ECGWaveformView: View {
    let trace: ECGWaveform.Trace
    @State var window: ECGWaveform.Window?
    @State var pinchStart: ECGWaveform.Window?
    @State var panStart: ECGWaveform.Window?
    private var bounds: ECGWaveform.Window { ECGWaveform.Window(start: trace.first, end: trace.last) }
    private var visible: ECGWaveform.Window { window ?? bounds }
    var body: some View {
        GeometryReader { geometry in
            let width = max(1, geometry.size.width - 45)
            let height = max(1, geometry.size.height - 25)
            let low = trace.low - 0.1
            let high = max(low + 0.2, trace.high + 0.1)
            let active = visible
            let segments = trace.visibleSegments(in: active, columns: Int(width))
            ZStack(alignment: .topLeading) {
                ForEach(0..<5, id: \.self) { tick in
                    let y = Double(tick) * height / 4
                    Rectangle().fill(Color.red.opacity(0.15)).frame(width: width, height: 0.5).offset(y: y)
                    Text((high - Double(tick) * (high-low) / 4).formatted(.number.precision(.fractionLength(1))))
                        .font(.caption2).offset(x: width+4, y: y-5)
                    Rectangle().fill(Color.red.opacity(0.15)).frame(width: 0.5, height: height).offset(x: Double(tick)*width/4)
                }
                ZStack(alignment: .topLeading) {
                    ForEach(Array(segments.enumerated()), id: \.offset) { _, segment in
                        Path { path in
                            for (index, point) in segment.enumerated() {
                                let position = CGPoint(x: (point.time-active.start)/active.span*width,
                                    y: (high-point.millivolts)/(high-low)*height)
                                if index == 0 { path.move(to: position) } else { path.addLine(to: position) }
                            }
                        }.stroke(Color.red, lineWidth: 1)
                    }
                }.frame(width: width, height: height).clipped()
                HStack {
                    Text("\(active.start.formatted(.number.precision(.fractionLength(1)))) s")
                    Spacer()
                    Text("\(active.end.formatted(.number.precision(.fractionLength(1)))) s · mV")
                }.font(.caption2).frame(width: width).offset(y: height+7)
            }.frame(width: geometry.size.width, height: geometry.size.height)
                .background(Color.clear)
                #if !os(Android)
                .contentShape(Rectangle())
                #endif
                .gesture(MagnifyGesture().onChanged { value in
                    if pinchStart == nil { pinchStart = visible; panStart = nil }
                    window = pinchStart?.zoom(Double(value.magnification), anchor: Double(value.startLocation.x / width), bounds: bounds)
                }.onEnded { _ in pinchStart = nil })
                .simultaneousGesture(DragGesture(minimumDistance: 12).onChanged { value in
                    guard pinchStart == nil, abs(value.translation.width) > abs(value.translation.height), visible.span < bounds.span else { return }
                    if panStart == nil { panStart = visible }
                    window = panStart?.pan(fraction: Double(value.translation.width / width), bounds: bounds)
                }.onEnded { _ in panStart = nil })
                .onTapGesture(count: 2) { window = nil; pinchStart = nil; panStart = nil }
        }
        #if !os(Android)
        .accessibilityElement(children: .ignore)
        #endif
        .accessibilityLabel("ECG voltage waveform, seconds and millivolts")
        .accessibilityValue("\(visible.start.formatted(.number.precision(.fractionLength(2))))–\(visible.end.formatted(.number.precision(.fractionLength(2)))) seconds")
        .accessibilityIdentifier("ecg-waveform")
    }
}
