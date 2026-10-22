import SwiftUI
import Charts

struct BrowseChart<Content: View>: View {
    private let chart: (ClosedRange<Double>, ChartPeriod) -> Content
    let timestamps: [Double]
    let unit: String
    @Environment(\.calendar) private var calendar
    @Environment(\.timeZone) private var timeZone
    @State private var window: ChartWindow
    @State private var configured = false
    init(timestamps: [Double], unit: String, initialPeriod: ChartPeriod, @ViewBuilder chart: @escaping (ClosedRange<Double>, ChartPeriod) -> Content) {
        self.chart = chart
        self.timestamps = timestamps
        self.unit = unit
        _window = State(initialValue: ChartWindow(timestamps: timestamps, period: initialPeriod, calendar: .current))
    }
    private var chartCalendar: Calendar {
        var value = calendar
        value.timeZone = timeZone
        return value
    }
    private var selection: Binding<ChartPeriod> {
        Binding(get: { window.period }, set: { window.select($0, calendar: chartCalendar) })
    }
    private var dateRange: String {
        let formatter = DateIntervalFormatter()
        formatter.calendar = chartCalendar
        formatter.timeZone = timeZone
        formatter.dateStyle = .medium
        formatter.timeStyle = .none
        return formatter.string(from: Date(timeIntervalSince1970: window.start), to: Date(timeIntervalSince1970: window.start + window.length - 1))
    }
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Picker("Time range", selection: selection) {
                ForEach(ChartPeriod.allCases.filter { $0 != .hour }, id: \.self) { period in
                    Text(period.rawValue).accessibilityLabel(period.title).tag(period)
                }
            }.pickerStyle(.segmented)
            HStack(alignment: .firstTextBaseline) {
                Text(dateRange).font(.subheadline.weight(.semibold))
                    .accessibilityIdentifier("chart-date-range")
                Spacer()
                Text(unit.isEmpty ? "Unit unavailable" : unit).font(.caption).foregroundStyle(.secondary)
            }
            chart(window.start...(window.start + window.length), window.period)
                .chartXScale(domain: Date(timeIntervalSince1970: window.start)...Date(timeIntervalSince1970: window.start + window.length))
                .chartPlotStyle { $0.clipped() }
                .chartXAxis {
                    AxisMarks(values: .automatic(desiredCount: 4)) { value in
                        AxisGridLine(stroke: StrokeStyle(lineWidth: 0.5, dash: [3, 3]))
                        if let date = value.as(Date.self) {
                            AxisValueLabel {
                                switch window.period {
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
                .frame(height: 260)
                .simultaneousGesture(DragGesture(minimumDistance: 24).onEnded { value in
                    guard abs(value.translation.width) > abs(value.translation.height) * 1.5 else { return }
                    window.move(value.translation.width > 0 ? -1 : 1, calendar: chartCalendar)
                })
                .accessibilityAdjustableAction { direction in
                    window.move(direction == .increment ? 1 : -1, calendar: chartCalendar)
                }
        }
        .onAppear {
            guard !configured else { return }
            window.select(window.period, calendar: chartCalendar)
            configured = true
        }
        .onChange(of: timestamps) { _, values in window = ChartWindow(timestamps: values, period: window.period, calendar: chartCalendar) }
    }
}
