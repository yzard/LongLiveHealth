import SwiftUI

struct LongTermTrendTile: View {
    let model: AppModel
    let card: JSONValue
    @State var showingMethod = false
    private var series: [JSONValue] { card["series"].arrayValue }
    private var unit: String { HistoricalValue.displayUnit(card["unit"].stringValue, kind: card["metric_id"].stringValue) }
    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(card["title"].stringValue).font(.headline)
                Spacer()
                Button { showingMethod = true } label: {
                    Image(systemName: MobileSymbol.name("info.circle")).frame(minWidth: 44, minHeight: 44)
                }.buttonStyle(.borderless).accessibilityLabel(Text("About \(card["title"].stringValue) trend"))
            }
            if !card["source"].stringValue.isEmpty {
                Text(model.sourceLabel(card["source"].stringValue, name: card["source_name"].stringValue))
                    .font(.subheadline).foregroundStyle(.secondary)
            }
            if card["state"] == .string("ready") {
                LongTermTrendPlot(series: series)
                    .frame(height: 170)
                HStack {
                    Text(card["aggregation"].stringValue).font(.caption)
                    Spacer()
                    Text(unit).font(.caption)
                }.foregroundStyle(.secondary)
                ForEach(card["change"]["comparisons"].arrayValue, id: \.identifier) { comparison in
                    Text("\(comparison["title"].stringValue): \(value(.object(["value": comparison["before"]]))) → \(value(.object(["value": comparison["after"]])))")
                        .font(.subheadline.weight(.semibold))
                    Text(comparison["basis"].stringValue).font(.caption).foregroundStyle(.secondary)
                }
                ForEach(Array(series.enumerated()), id: \.offset) { index, line in
                    if !line["points"].arrayValue.isEmpty {
                        HStack {
                            Circle().fill(index == 0 ? Color.accentColor : Color.orange).frame(width: 8, height: 8)
                            Text(line["title"].stringValue).font(.caption)
                            Spacer()

                        }
                    }
                }
                if let first = card["first_at"].numberValue, let last = card["last_at"].numberValue {
                    Text("\(date(first)) – \(date(last)) · \(Int(card["observed_days"].numberValue ?? 0)) recorded days")
                        .font(.caption).foregroundStyle(.secondary)
                }
            } else {
                Text("More data needed").font(.subheadline.weight(.semibold))
            }
            ForEach(card["missing"].arrayValue.map(\.stringValue), id: \.self) { message in
                Text(message).font(.caption).foregroundStyle(.secondary)
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.primary.opacity(0.06), in: RoundedRectangle(cornerRadius: 18))
        .listRowSeparator(.hidden)
        .sheet(isPresented: $showingMethod) {
            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                    HStack {
                        Text(card["title"].stringValue).font(.headline)
                        Spacer()
                        Button("Done") { showingMethod = false }
                    }
                    Text(card["explanation"].stringValue)
                    Text(card["change"]["comparison"].stringValue)
                    Text(card["change"]["filter_note"].stringValue)
                    ForEach(card["change"]["comparisons"].arrayValue, id: \.identifier) { comparison in
                        Text(comparison["basis"].stringValue)
                        ForEach(comparison["citations"].arrayValue, id: \.identifier) { citation in
                            if let url = URL(string: citation["url"].stringValue) {
                                Link(citation["title"].stringValue, destination: url)
                            }
                        }
                    }
                    if !card["comparison_note"].stringValue.isEmpty { Text(card["comparison_note"].stringValue) }
                    if let url = URL(string: card["citation"].stringValue) { Link("Evidence source", destination: url) }
                }.padding()
            }
        }
    }
    private func value(_ point: JSONValue) -> String {
        OverviewSummary.value(point["value"].numberValue ?? 0, kind: card["metric_id"].stringValue, unit: card["unit"].stringValue)
    }
    private func date(_ at: Double) -> String {
        Date(timeIntervalSince1970: at).formatted(date: .abbreviated, time: .omitted)
    }
}

struct LongTermTrendPlot: View {
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
