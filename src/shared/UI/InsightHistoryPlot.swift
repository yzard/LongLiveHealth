import SwiftUI

struct InsightHistoryPlot: View {
    let series: [JSONValue]
    private var allPoints: [JSONValue] { series.flatMap { $0["points"].arrayValue } }
    var body: some View {
        GeometryReader { geometry in
            let times = allPoints.compactMap { $0["at"].numberValue }
            let start = times.min() ?? 0
            let length = max(1, (times.max() ?? 0) - start)
            let domain = ChartValueScale.domain(values: allPoints.compactMap { $0["value"].numberValue })
            let width = max(1, geometry.size.width - 44)
            let height = max(1, geometry.size.height - 18)
            ZStack(alignment: .topLeading) {
                ForEach(0..<3, id: \.self) { index in
                    let y = Double(index) * height / 2
                    let number = domain.upperBound - Double(index) * (domain.upperBound - domain.lowerBound) / 2
                    Rectangle().fill(Color.secondary.opacity(0.2)).frame(width: width, height: 0.5).offset(y: y)
                    Text(number.formatted(.number.precision(.fractionLength(0...ChartValueScale.tickDecimals(domain: domain)))))
                        .font(.caption2).foregroundStyle(.secondary).offset(x: width + 4, y: y - 6)
                }
                ForEach(Array(series.enumerated()), id: \.offset) { index, line in
                    let color = index == 0 ? Color.accentColor : Color.orange
                    ForEach(Array(line["points"].arrayValue.enumerated()), id: \.offset) { _, point in
                        if let at = point["at"].numberValue, let value = point["value"].numberValue {
                            Circle().fill(color).frame(width: 6, height: 6)
                                .offset(x: (at - start) / length * width - 3,
                                        y: (domain.upperBound - value) / (domain.upperBound - domain.lowerBound) * height - 3)
                        }
                    }
                }
                HStack {
                    Text(Date(timeIntervalSince1970: start), format: .dateTime.year().month(.abbreviated))
                    Spacer()
                    Text(Date(timeIntervalSince1970: start + length), format: .dateTime.year().month(.abbreviated))
                }
                .font(.caption2).foregroundStyle(.secondary)
                .frame(width: width).offset(y: height + 3)
            }
        }
        .accessibilityLabel(Text("Long-term measurements plotted by collection date"))
    }
}
