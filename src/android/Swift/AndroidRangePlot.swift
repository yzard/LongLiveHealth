import SwiftUI

/// Same server buckets, selection and source priority as the native iOS chart.
struct AndroidRangePlot: View {
    let rows: [JSONValue]
    let referenceBand: JSONValue
    let scaleLabels: [String]
    let isBinary: Bool
    let domain: ClosedRange<Double>
    let start: Double
    let length: Double
    let divisor: Double
    let width: Double
    let ticks: [(Double, String)]
    let showsYearTicks: Bool
    let color: (JSONValue) -> Color
    let exposedEnd: (JSONValue, Bool) -> Bool
    @Binding var selected: JSONValue?
    let move: (Int) -> Void

    var body: some View {
        GeometryReader { geometry in
            let w = max(1, geometry.size.width - 38)
            let h = max(1, geometry.size.height - 26)
            ZStack(alignment: .topLeading) {
                if referenceBand != .null {
                    let lower = referenceBand["lower"].numberValue.map { $0 / divisor } ?? domain.lowerBound
                    let upper = referenceBand["upper"].numberValue.map { $0 / divisor } ?? domain.upperBound
                    let top = h * (domain.upperBound - upper) / (domain.upperBound - domain.lowerBound)
                    let bottom = h * (domain.upperBound - lower) / (domain.upperBound - domain.lowerBound)
                    Rectangle().fill(Color.pink.opacity(0.18)).frame(width: w, height: max(0, bottom - top)).offset(y: top)
                    ForEach([referenceBand["lower"].numberValue, referenceBand["upper"].numberValue].compactMap { $0 }, id: \.self) { value in
                        let y = h * (domain.upperBound - value / divisor) / (domain.upperBound - domain.lowerBound)
                        Rectangle().fill(Color.pink).frame(width: w, height: 1).offset(y: y)
                        Text((value / divisor).formatted(.number.precision(.fractionLength(0...2)))).font(.caption2).foregroundStyle(Color.pink).offset(x: w - 34, y: y - 14)
                    }
                }
                let levels = scaleLabels.isEmpty
                    ? (0..<4).map { domain.lowerBound + (domain.upperBound - domain.lowerBound) * Double($0) / 3 }
                    : scaleLabels.indices.map { Double($0) }
                ForEach(Array(levels.enumerated()), id: \.offset) { index, value in
                    let y = h * (domain.upperBound - value) / (domain.upperBound - domain.lowerBound)
                    Rectangle().fill(Color.secondary.opacity(0.2)).frame(width: w, height: 0.5).offset(y: y)
                    if isBinary {
                        BinaryResultSymbol(value: value, positiveColor: .primary).offset(x: w + 4, y: y - 10)
                    } else {
                        let label = scaleLabels.isEmpty ? value.formatted(.number.precision(.fractionLength(0...1))) : uiLabel(scaleLabels[index])
                        Text(label).font(.caption2).foregroundStyle(.secondary).offset(x: w + 4, y: y - 7)
                    }
                }
                ForEach(Array(ticks.enumerated()), id: \.offset) { _, tick in
                    let x = w * (tick.0 - start) / length
                    Rectangle().fill(Color.secondary.opacity(0.2)).frame(width: 0.5, height: h).offset(x: x)
                    if showsYearTicks {
                        let labelWidth = 40.0
                        let labelX = max(0, min(w - labelWidth, x - labelWidth / 2))
                        Text(tick.1).font(.caption2).foregroundStyle(.secondary)
                            .frame(width: labelWidth).offset(x: labelX, y: h + 6)
                    } else {
                        Text(tick.1).font(.caption2).foregroundStyle(.secondary).offset(x: x, y: h + 6)
                    }
                }
                ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                    let lower = (row["display_min"].numberValue ?? 0) / divisor
                    let upper = (row["display_max"].numberValue ?? 0) / divisor
                    let at = ChartRangeSegments.position(row)
                    let x = w * (at - start) / length
                    let top = h * (domain.upperBound - upper) / (domain.upperBound - domain.lowerBound)
                    let bottom = h * (domain.upperBound - lower) / (domain.upperBound - domain.lowerBound)
                    if isBinary {
                        BinaryResultSymbol(value: lower, positiveColor: color(row))
                            .frame(width: 20, height: 20).offset(x: x - 10, y: top - 10)
                    } else if lower != upper {
                        Rectangle().fill(color(row)).frame(width: width, height: max(0.5, bottom - top)).offset(x: x - width / 2, y: top)
                        if exposedEnd(row, true) { Circle().fill(color(row)).frame(width: width, height: width).offset(x: x - width / 2, y: bottom - width / 2) }
                        if exposedEnd(row, false) { Circle().fill(color(row)).frame(width: width, height: width).offset(x: x - width / 2, y: top - width / 2) }
                    } else {
                        Circle().fill(color(row)).frame(width: width, height: width).offset(x: x - width / 2, y: top - width / 2)
                    }
                }
                if let selected {
                    let at = ChartRangeSegments.position(selected)
                    Rectangle().fill(Color.secondary.opacity(0.5)).frame(width: 1, height: h).offset(x: w * (at - start) / length)
                }
            }.frame(width: geometry.size.width, height: geometry.size.height)
                .background(Color.black.opacity(0.001))
                .simultaneousGesture(DragGesture(minimumDistance: 0).onEnded { drag in
                    if abs(drag.translation.width) > 24 && abs(drag.translation.width) > abs(drag.translation.height) * 1.5 {
                        move(drag.translation.width > 0 ? -1 : 1)
                    } else if abs(drag.translation.height) < 15 {
                        guard drag.location.x >= 0, drag.location.x <= w + 8,
                              drag.location.y >= 0, drag.location.y <= h + 8 else { return }
                        let at = start + min(drag.location.x, w) / w * length
                        let y = domain.upperBound - min(drag.location.y, h) / h * (domain.upperBound - domain.lowerBound)
                        selected = ChartRangeSegments.selection(rows: rows, time: at, value: y * divisor,
                            secondsPerPoint: length / w, unitsPerPoint: (domain.upperBound - domain.lowerBound) * divisor / h)
                    }
                })
        }
    }
}
