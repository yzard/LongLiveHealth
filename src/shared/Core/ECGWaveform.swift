import Foundation

public nonisolated enum ECGWaveform {
    public struct Point: Equatable, Sendable {
        public let time: Double
        public let millivolts: Double
    }
    public struct Trace: Sendable {
        public private(set) var segments: [[Point]] = []
        public private(set) var first: Double = 0
        public private(set) var last: Double = 0
        public private(set) var low: Double = 0
        public private(set) var high: Double = 0
        public private(set) var count = 0
        private let maximumGap: Double?
        private var previous: Double?
        public init(frequency: Double?) {
            maximumGap = frequency.flatMap { $0.isFinite && $0 > 0 ? 1.5 / $0 : nil }
        }
        public mutating func append(_ samples: [JSONValue]) {
            for sample in samples {
                guard let time = sample["time_since_start"].numberValue, time.isFinite,
                      let volts = sample["lead_i_volts"].numberValue, volts.isFinite else {
                    previous = nil
                    continue
                }
                let value = volts * 1000
                guard value.isFinite else { previous = nil; continue }
                if count == 0 { first = time; last = time; low = value; high = value }
                first = min(first, time); last = max(last, time)
                low = min(low, value); high = max(high, value)
                let continuous = previous.map { previous in
                    time > previous && (maximumGap.map { time - previous <= $0 } ?? true)
                } ?? false
                if !continuous { segments.append([]) }
                segments[segments.count - 1].append(Point(time: time, millivolts: value))
                previous = time; count += 1
            }
        }
        /// Keep actual extrema in time order for each screen column, without smoothing peaks.
        public func visibleSegments(in window: Window, columns: Int) -> [[Point]] {
            let width = max(1, columns)
            return segments.compactMap { segment in
                guard let first = segment.first, let last = segment.last,
                      first.time <= window.end, last.time >= window.start else { return nil }
                func lowerBound(_ value: Double) -> Int {
                    var left = 0, right = segment.count
                    while left < right {
                        let middle = (left + right) / 2
                        if segment[middle].time < value { left = middle + 1 } else { right = middle }
                    }
                    return left
                }
                let begin = max(0, lowerBound(window.start) - 1)
                let end = min(segment.count, lowerBound(window.end) + 1)
                var output: [Point] = []
                var bucket: [Int] = []
                var column: Int?
                func flush() {
                    guard let first = bucket.first, let last = bucket.last else { return }
                    let low = bucket.min { segment[$0].millivolts < segment[$1].millivolts }!
                    let high = bucket.max { segment[$0].millivolts < segment[$1].millivolts }!
                    for index in Set([first, low, high, last]).sorted() { output.append(segment[index]) }
                    bucket.removeAll(keepingCapacity: true)
                }
                for index in begin..<end {
                    let next = Int(((segment[index].time - window.start) / window.span * Double(width)).rounded(.down))
                    if column != next { flush(); column = next }
                    bucket.append(index)
                }
                flush()
                return output.isEmpty ? nil : output
            }
        }
    }
    public struct Window: Equatable, Sendable {
        public let start: Double
        public let end: Double
        public var span: Double { end - start }
        public init(start: Double, end: Double) {
            self.start = start.isFinite ? start : 0
            self.end = end.isFinite ? max(self.start + 0.01, end) : self.start + 0.01
        }
        public func zoom(_ magnification: Double, anchor: Double, bounds: Window) -> Window {
            guard magnification.isFinite, magnification > 0 else { return self }
            let fraction = min(1, max(0, anchor))
            let length = min(bounds.span, max(0.02, span / magnification))
            let origin = start + span * fraction - length * fraction
            return clamped(start: origin, length: length, bounds: bounds)
        }
        public func pan(fraction: Double, bounds: Window) -> Window {
            guard fraction.isFinite else { return self }
            return clamped(start: start - fraction * span, length: min(span, bounds.span), bounds: bounds)
        }
        private func clamped(start: Double, length: Double, bounds: Window) -> Window {
            let origin = max(bounds.start, min(bounds.end - length, start))
            return Window(start: origin, end: origin + length)
        }
    }
}
