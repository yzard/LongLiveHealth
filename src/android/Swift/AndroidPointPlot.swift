import SwiftUI

struct AndroidPlotPoint: Identifiable {
    let id: Int
    let x: Double
    let y: Double
    let label: String
    static func values(_ rows: [JSONValue], x: String, y: String, multiplier: Double) -> [AndroidPlotPoint] {
        let date = DateFormatter(); date.locale = Locale(identifier: "en_US_POSIX"); date.dateFormat = "yyyy-MM-dd"
        return rows.enumerated().compactMap { index, row in
            guard let value = row[y].numberValue,
                  let at = row[x].numberValue ?? date.date(from: row[x].stringValue)?.timeIntervalSince1970 else { return nil }
            return AndroidPlotPoint(id: index, x: at, y: value * multiplier, label: x == "time_since_start" ? "\(at.formatted(.number.precision(.fractionLength(0...2)))) s" : row[x].stringValue.isEmpty ? Date(timeIntervalSince1970: at).formatted(date: .abbreviated, time: .shortened) : row[x].stringValue)
        }
    }
}

struct AndroidPointPlot: View {
    let points: [AndroidPlotPoint]
    let unit: String
    let select: ((AndroidPlotPoint) -> Void)?
    let connected: Bool
    let events: [JSONValue]
    @State var selected: AndroidPlotPoint?
    var body: some View {
        VStack(alignment: .leading) {
            if let selected { Text("\(selected.label) · \(selected.y.formatted()) \(unit)").font(.caption) }
            GeometryReader { geometry in
                let lower = points.map(\.x).min() ?? 0
                let upper = points.map(\.x).max() ?? 1
                let xSpan = max(1, upper - lower)
                let y = ChartValueScale.domain(values: points.map(\.y))
                let w = max(1, geometry.size.width - 45)
                let h = max(1, geometry.size.height - 22)
                ZStack(alignment: .topLeading) {
                    ForEach(0..<4, id: \.self) { index in
                        let ordinate = h * Double(index) / 3
                        Rectangle().fill(Color.secondary.opacity(0.2)).frame(width: w, height: 0.5).offset(y: ordinate)
                        Text((y.upperBound - (y.upperBound - y.lowerBound) * Double(index) / 3).formatted(.number.precision(.fractionLength(0...1))))
                            .font(.caption2).offset(x: w + 3, y: ordinate - 6)
                    }
                    if connected {
                        Path { path in
                            for (index, point) in points.sorted(by: { $0.x < $1.x }).enumerated() {
                                let position = CGPoint(x: (point.x - lower) / xSpan * w, y: (y.upperBound - point.y) / (y.upperBound - y.lowerBound) * h)
                                if index == 0 { path.move(to: position) } else { path.addLine(to: position) }
                            }
                        }.stroke(Color.teal, lineWidth: 1.5)
                    }
                    ForEach(Array(events.enumerated()), id: \.offset) { _, event in
                        if let at = event["at"].numberValue, at >= lower, at <= upper {
                            Rectangle().fill(event["entry"]["kind"] == .string("nutrition") ? Color.orange : Color.purple)
                                .frame(width: 1, height: h).offset(x: (at - lower) / xSpan * w)
                        }
                    }
                    ForEach(points) { point in
                        Circle().fill(Color.teal).frame(width: 5, height: 5)
                            .offset(x: (point.x - lower) / xSpan * w - 2.5, y: (y.upperBound - point.y) / (y.upperBound - y.lowerBound) * h - 2.5)
                    }
                    if let first = points.min(by: { $0.x < $1.x }), let last = points.max(by: { $0.x < $1.x }) {
                        HStack { Text(first.label); Spacer(); Text(last.label) }.font(.caption2).frame(width: w).offset(y: h + 5)
                    }
                }.frame(width: geometry.size.width, height: geometry.size.height).background(Color.black.opacity(0.001))
                    .gesture(DragGesture(minimumDistance: 0).onEnded { drag in
                        let at = lower + drag.location.x / w * xSpan
                        selected = points.min { abs($0.x - at) < abs($1.x - at) }
                        if let selected { select?(selected) }
                    })
            }
        }
    }
}

struct AndroidSleepPlot: View {
    let records: [JSONValue]
    let range: ClosedRange<Double>
    var body: some View {
        GeometryReader { geometry in
            ZStack(alignment: .topLeading) {
                ForEach(Array(records.enumerated()), id: \.offset) { _, row in
                    let start = max(range.lowerBound, row["start_at"].numberValue ?? 0)
                    let end = min(range.upperBound, row["end_at"].numberValue ?? 0)
                    let stage = max(0, min(5, row["category"].numberValue ?? 0))
                    if end > start {
                        RoundedRectangle(cornerRadius: 3).fill(SourceLegend.colors[Int(stage) % SourceLegend.colors.count])
                            .frame(width: max(1, (end - start) / (range.upperBound - range.lowerBound) * geometry.size.width), height: 22)
                            .offset(x: (start - range.lowerBound) / (range.upperBound - range.lowerBound) * geometry.size.width, y: stage * 30)
                    }
                }
            }
        }.frame(height: 180)
    }
}
