import SwiftUI
import Charts

/// Shared presentation of server-computed ranges. No raw-sample aggregation occurs here.
struct MetricChart: View {
    let model: AppModel
    let ranges: JSONValue
    let unit: String
    let divisor: Double
    let sourceKeys: [String]
    let names: [String: String]
    let initialPeriod: ChartPeriod
    let showsHour: Bool
    let heartRate: Bool
    @Environment(\.timeZone) private var timeZone
    @State private var window: ChartWindow?
    @State private var selected: JSONValue?
    @State private var hiddenSources: Set<String> = []

    private var calendar: Calendar {
        var value = Calendar(identifier: .gregorian)
        value.timeZone = timeZone
        value.firstWeekday = 1
        return value
    }
    private var timestamps: [Double] {
        ranges["D"].arrayValue.filter { $0["unit"].stringValue == unit }.compactMap { $0["last_at"].numberValue }
    }
    private var active: ChartWindow { window ?? ChartWindow(timestamps: timestamps, period: initialPeriod, calendar: calendar) }
    private var rows: [JSONValue] {
        ranges[active.period.rawValue].arrayValue.filter {
            !hiddenSources.contains($0["source"].stringValue) && $0["unit"].stringValue == unit && ($0["end_at"].numberValue ?? 0) > active.start && ($0["start_at"].numberValue ?? 0) < active.start + active.length
        }
    }
    private var summary: JSONValue? {
        // Display the envelope of the visible server-computed ranges, not raw samples.
        guard let lower = rows.compactMap({ $0["min"].numberValue }).min(),
              let upper = rows.compactMap({ $0["max"].numberValue }).max() else { return nil }
        return .object(["min": .number(lower), "max": .number(upper)])
    }
    private var latest: JSONValue? {
        ranges["Y"].arrayValue.filter {
            $0["unit"].stringValue == unit && !hiddenSources.contains($0["source"].stringValue)
        }.max { ($0["last_at"].numberValue ?? 0) < ($1["last_at"].numberValue ?? 0) }
    }
    private var displayUnit: String { heartRate ? "BPM" : divisor == 3600 ? "Hours" : unit }
    private var colors: [Color] {
        sourceKeys.enumerated().map { heartRate && $0.offset == 0 ? .pink : SourceLegend.colors[$0.offset % SourceLegend.colors.count] }
    }
    private func color(_ row: JSONValue) -> Color {
        guard let index = sourceKeys.firstIndex(of: row["source"].stringValue) else { return .primary }
        return colors[index]
    }
    private func number(_ value: Double) -> String {
        (value / divisor).formatted(.number.precision(.fractionLength(0...(heartRate ? 0 : 2))))
    }
    private func valueLabel(_ row: JSONValue) -> String {
        let lower = row["min"].numberValue ?? 0
        let upper = row["max"].numberValue ?? lower
        return lower == upper ? number(lower) : "\(number(lower))–\(number(upper))"
    }
    private func intervalLabel(_ row: JSONValue?) -> String {
        let start = row?["start_at"].numberValue ?? active.start
        let end = row?["end_at"].numberValue ?? active.start + active.length
        let formatter = DateIntervalFormatter()
        formatter.calendar = calendar
        formatter.timeZone = timeZone
        formatter.dateStyle = .medium
        formatter.timeStyle = active.period == .hour || (selected != nil && active.period == .day) ? .short : .none
        if selected != nil && active.period == .hour {
            let single = DateFormatter()
            single.calendar = calendar; single.timeZone = timeZone
            single.dateStyle = .medium; single.timeStyle = .short
            return single.string(from: Date(timeIntervalSince1970: start))
        }
        let displayedEnd = selected != nil && active.period == .day ? end : end - 1
        return formatter.string(from: Date(timeIntervalSince1970: start), to: Date(timeIntervalSince1970: displayedEnd))
    }
    private func lanes(for row: JSONValue) -> [String] {
        let present = Set(rows.filter { $0["start_at"] == row["start_at"] && $0["end_at"] == row["end_at"] }.map { $0["source"].stringValue })
        return sourceKeys.filter { present.contains($0) }
    }
    private func position(_ row: JSONValue) -> Double {
        let start = row["start_at"].numberValue ?? 0
        let end = row["end_at"].numberValue ?? start
        let keys = lanes(for: row)
        let index = keys.firstIndex(of: row["source"].stringValue) ?? 0
        return start + (end - start) * (0.1 + 0.8 * (Double(index) + 0.5) / Double(max(1, keys.count)))
    }
    private func barWidth(plotWidth: Double) -> Double {
        // Use the tightest visible bucket for every mark, including single-source buckets.
        // This also keeps calendar months and DST days from changing individual widths.
        let slots = rows.map { row in
            let duration = (row["end_at"].numberValue ?? 0) - (row["start_at"].numberValue ?? 0)
            return plotWidth * duration / active.length * 0.8 / Double(max(1, lanes(for: row).count))
        }
        return max(0.8, min(8, (slots.min() ?? 8 / 0.85) * 0.85))
    }
    private var tickDates: [Date] {
        let step = active.period == .hour ? 900.0 : active.period == .day ? 21600.0 : active.length / 4
        return (0..<4).map { Date(timeIntervalSince1970: active.start + Double($0) * step) }
    }
    private func move(_ direction: Int) {
        var value = active
        value.move(direction, calendar: calendar)
        window = value
        selected = nil
    }
    private func choose(at point: CGPoint, proxy: ChartProxy, geometry: GeometryProxy) {
        guard let anchor = proxy.plotFrame else { return }
        let frame = geometry[anchor]
        guard frame.contains(point), let date: Date = proxy.value(atX: point.x - frame.minX) else { return }
        let at = date.timeIntervalSince1970
        let candidates = rows.filter { ($0["start_at"].numberValue ?? 0) <= at && at < ($0["end_at"].numberValue ?? 0) }
        selected = candidates.min {
            abs(position($0) - at) < abs(position($1) - at)
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Picker("Time range", selection: Binding(get: { active.period }, set: { period in
                var value = active; value.select(period, calendar: calendar); window = value; selected = nil
            })) {
                ForEach(ChartPeriod.allCases.filter { showsHour || $0 != .hour }, id: \.self) { period in
                    Text(period.rawValue).accessibilityLabel(period.title).tag(period)
                }
            }.pickerStyle(.segmented)
            VStack(alignment: .leading, spacing: 3) {
                Text(selected == nil ? "RANGE" : "SELECTED RANGE").font(.caption).foregroundStyle(.secondary)
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Text((selected ?? summary).map(valueLabel) ?? (hiddenSources.count == sourceKeys.count && !sourceKeys.isEmpty ? "No sources selected" : "No data"))
                        .font(.system(size: 34, weight: .regular, design: .rounded)).minimumScaleFactor(0.6)
                        .accessibilityIdentifier("chart-range-value")
                    Text(displayUnit).font(.headline).foregroundStyle(.secondary)
                }
                Text(intervalLabel(selected)).font(.subheadline).foregroundStyle(.secondary)
                    .accessibilityIdentifier("chart-date-range")
                if let selected {
                    HStack(spacing: 5) {
                        Circle().fill(color(selected)).frame(width: 7, height: 7)
                        Text(model.sourceLabel(selected["source"].stringValue, name: names[selected["source"].stringValue] ?? selected["source"].stringValue)).font(.caption)
                    }
                }
            }.frame(maxWidth: .infinity, alignment: .leading)
                .frame(minHeight: 112, alignment: .topLeading)
                .padding(10)
                .background(selected == nil ? Color.clear : Color(.tertiarySystemFill), in: RoundedRectangle(cornerRadius: 12))
            GeometryReader { geometry in
                let width = barWidth(plotWidth: max(1, geometry.size.width - 36))
                Chart {
                    ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                        let lower = (row["min"].numberValue ?? 0) / divisor
                        let upper = (row["max"].numberValue ?? 0) / divisor
                        let date = Date(timeIntervalSince1970: position(row))
                        if lower != upper {
                            RuleMark(x: .value("Time", date), yStart: .value("Minimum", lower), yEnd: .value("Maximum", upper))
                                .lineStyle(StrokeStyle(lineWidth: width, lineCap: .round)).foregroundStyle(color(row))
                                .accessibilityLabel(model.sourceLabel(row["source"].stringValue))
                                .accessibilityValue("\(valueLabel(row)) \(displayUnit)")
                        } else {
                            PointMark(x: .value("Time", date), y: .value("Value", lower))
                                .symbolSize(width * width).foregroundStyle(color(row))
                                .accessibilityValue("\(valueLabel(row)) \(displayUnit)")
                        }
                    }
                    if let selected {
                        RuleMark(x: .value("Selected time", Date(timeIntervalSince1970: position(selected))))
                            .foregroundStyle(.secondary.opacity(0.5)).lineStyle(StrokeStyle(lineWidth: 1))
                    }
                }
                .chartXScale(domain: Date(timeIntervalSince1970: active.start)...Date(timeIntervalSince1970: active.start + active.length))
                .chartYScale(domain: .automatic(includesZero: true))
                .chartLegend(.hidden)
                .chartPlotStyle { $0.clipped() }
                .chartXAxis {
                    AxisMarks(values: tickDates) { value in
                        AxisGridLine(stroke: StrokeStyle(lineWidth: 0.5, dash: [2, 3]))
                        if let date = value.as(Date.self) {
                            AxisValueLabel {
                                switch active.period {
                                case .hour: Text(date, format: .dateTime.hour().minute())
                                case .day: Text(date, format: .dateTime.hour())
                                case .week: Text(date, format: .dateTime.weekday(.abbreviated))
                                case .month: Text(date, format: .dateTime.day())
                                case .halfYear, .year: Text(date, format: .dateTime.month(.abbreviated))
                                }
                            }
                        }
                    }
                }
                .chartYAxis { AxisMarks(position: .trailing, values: .automatic(desiredCount: 4)) }
                .chartOverlay { proxy in
                    GeometryReader { overlay in
                        Color.clear.contentShape(Rectangle())
                            .onTapGesture { point in choose(at: point, proxy: proxy, geometry: overlay) }

                    }
                }
                .simultaneousGesture(DragGesture(minimumDistance: 24).onEnded { drag in
                    guard abs(drag.translation.width) > abs(drag.translation.height) * 1.5 else { return }
                    move(drag.translation.width > 0 ? -1 : 1)
                })
                .accessibilityIdentifier("metric-range-chart")
                .accessibilityAdjustableAction { move($0 == .increment ? 1 : -1) }
            }.frame(height: 250)
            if let latest, let at = latest["last_at"].numberValue, let value = latest["latest_value"].numberValue {
                HStack {
                    Text("Latest: \(Date(timeIntervalSince1970: at).formatted(date: calendar.isDateInToday(Date(timeIntervalSince1970: at)) ? .omitted : .abbreviated, time: .shortened))")
                    Spacer()
                    Text("\(number(value)) \(displayUnit)")
                }.font(.subheadline).padding(12).background(Color(.tertiarySystemFill), in: Capsule())
            }
            ForEach(Array(sourceKeys.enumerated()), id: \.element) { index, key in
                Button {
                    if hiddenSources.contains(key) { hiddenSources.remove(key) }
                    else { hiddenSources.insert(key) }
                    selected = nil
                } label: {
                    HStack(spacing: 8) {
                        Image(systemName: hiddenSources.contains(key) ? "circle" : "checkmark.circle.fill")
                            .foregroundStyle(hiddenSources.contains(key) ? Color.secondary : colors[index])
                        Text(model.sourceLabel(key, name: names[key] ?? key))
                            .foregroundStyle(hiddenSources.contains(key) ? .secondary : .primary)
                        Spacer(minLength: 0)
                    }.font(.subheadline).frame(minHeight: 44).contentShape(Rectangle())
                }.buttonStyle(.borderless)
                    .accessibilityIdentifier("chart-source-" + key)
                    .accessibilityValue(hiddenSources.contains(key) ? "Hidden" : "Shown")
                    .accessibilityHint("Double tap to toggle this source in the chart")
            }

        }.onChange(of: timestamps) { _, _ in
            window = ChartWindow(timestamps: timestamps, period: active.period, calendar: calendar); selected = nil
        }
    }
}
