import SwiftUI

/// Same server buckets, selection and source priority as the native iOS chart.
struct AndroidRangePlot: View {
    let rows: [JSONValue]
    let domain: ClosedRange<Double>
    let start: Double
    let length: Double
    let divisor: Double
    let width: Double
    let ticks: [(Double, String)]
    let color: (JSONValue) -> Color
    let exposedEnd: (JSONValue, Bool) -> Bool
    @Binding var selected: JSONValue?
    let move: (Int) -> Void

    var body: some View {
        GeometryReader { geometry in
            let w = max(1, geometry.size.width - 38)
            let h = max(1, geometry.size.height - 26)
            ZStack(alignment: .topLeading) {
                ForEach(0..<4, id: \.self) { index in
                    let value = domain.lowerBound + (domain.upperBound - domain.lowerBound) * Double(index) / 3
                    let y = h * (1 - Double(index) / 3)
                    Rectangle().fill(Color.secondary.opacity(0.2)).frame(width: w, height: 0.5).offset(y: y)
                    Text(value.formatted(.number.precision(.fractionLength(0...1)))).font(.caption2)
                        .foregroundStyle(.secondary).offset(x: w + 4, y: y - 7)
                }
                ForEach(Array(ticks.enumerated()), id: \.offset) { _, tick in
                    let x = w * (tick.0 - start) / length
                    Rectangle().fill(Color.secondary.opacity(0.2)).frame(width: 0.5, height: h).offset(x: x)
                    Text(tick.1).font(.caption2).foregroundStyle(.secondary).offset(x: x, y: h + 6)
                }
                ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                    let lower = (row["display_min"].numberValue ?? 0) / divisor
                    let upper = (row["display_max"].numberValue ?? 0) / divisor
                    let at = ((row["start_at"].numberValue ?? 0) + (row["end_at"].numberValue ?? 0)) / 2
                    let x = w * (at - start) / length
                    let top = h * (domain.upperBound - upper) / (domain.upperBound - domain.lowerBound)
                    let bottom = h * (domain.upperBound - lower) / (domain.upperBound - domain.lowerBound)
                    if lower != upper {
                        Rectangle().fill(color(row)).frame(width: width, height: max(0.5, bottom - top)).offset(x: x - width / 2, y: top)
                        if exposedEnd(row, true) { Circle().fill(color(row)).frame(width: width, height: width).offset(x: x - width / 2, y: bottom - width / 2) }
                        if exposedEnd(row, false) { Circle().fill(color(row)).frame(width: width, height: width).offset(x: x - width / 2, y: top - width / 2) }
                    } else {
                        Circle().fill(color(row)).frame(width: width, height: width).offset(x: x - width / 2, y: top - width / 2)
                    }
                }
                if let selected {
                    let at = ((selected["start_at"].numberValue ?? 0) + (selected["end_at"].numberValue ?? 0)) / 2
                    Rectangle().fill(Color.secondary.opacity(0.5)).frame(width: 1, height: h).offset(x: w * (at - start) / length)
                }
            }.frame(width: geometry.size.width, height: geometry.size.height)
                .background(Color.black.opacity(0.001))
                .gesture(DragGesture(minimumDistance: 0).onEnded { drag in
                    if abs(drag.translation.width) > 24 && abs(drag.translation.width) > abs(drag.translation.height) * 1.5 {
                        move(drag.translation.width > 0 ? -1 : 1)
                    } else if abs(drag.translation.height) < 15 {
                        let at = start + drag.location.x / w * length
                        let y = domain.upperBound - drag.location.y / h * (domain.upperBound - domain.lowerBound)
                        selected = rows.filter { ($0["start_at"].numberValue ?? 0) <= at && at < ($0["end_at"].numberValue ?? 0) }.min {
                            distance($0, y) < distance($1, y)
                        }
                    }
                })
        }
    }
    private func distance(_ row: JSONValue, _ value: Double) -> Double {
        max((row["display_min"].numberValue ?? 0) / divisor - value, value - (row["display_max"].numberValue ?? 0) / divisor, 0)
    }
}
