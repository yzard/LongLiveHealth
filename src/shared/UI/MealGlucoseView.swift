import SwiftUI
#if canImport(Charts)
import Charts
#endif

struct MealGlucoseView: View {
    var model: AppModel
    @State var date = Date()
    @State var maximumGap = "900"
    @State var result: JSONValue = .null
    @State var failure: String?
    var body: some View {
        List {
            Section(uiText("Time alignment")) {
                DatePicker(uiText("Date"), selection: $date, displayedComponents: .date)
                TextField(uiText("Maximum sample gap (seconds)"), text: $maximumGap).keyboardType(.numberPad)
                InfoHelp(title: uiText("Sampling interval"), message: uiText("900 seconds is an initial display parameter. Match the actual source sampling protocol."))
                Button(uiText("Apply sampling gap")) { Task { await load() } }
                if let failure { Text(uiMessage(failure)).foregroundStyle(.secondary) }
            }
            ForEach(Array(result["series"]["sources"].arrayValue.enumerated()), id: \.offset) { _, source in
                Section(model.sourceLabel(source["source"].stringValue)) {
                    #if os(Android)
                    AndroidPointPlot(points: AndroidPlotPoint.values(source["points"].arrayValue, x: "at", y: "value", multiplier: 1), unit: "mg/dL", select: nil, connected: false, events: result["events"].arrayValue).frame(height: 230)
                    #else
                    Chart {
                        ForEach(Array(source["points"].arrayValue.enumerated()), id: \.offset) { _, point in
                            if let at = point["at"].numberValue, let value = point["value"].numberValue {
                                PointMark(x: .value("Time", Date(timeIntervalSince1970: at)), y: .value("mg/dL", value)).symbolSize(8)
                            }
                        }
                        ForEach(Array(result["events"].arrayValue.enumerated()), id: \.offset) { _, event in
                            if let at = event["at"].numberValue {
                                RuleMark(x: .value("Event", Date(timeIntervalSince1970: at))).foregroundStyle(event["entry"]["kind"] == .string("nutrition") ? .orange : .purple).lineStyle(StrokeStyle(lineWidth: 1, dash: [3, 3]))
                            }
                        }
                    }.frame(height: 230).accessibilityLabel(Text(uiText("Glucose source measurements with meal and training times. No causal relationship is estimated.")))
                    #endif
                    Text(uiText("Orange: meal · Purple: training · Glucose: mg/dL")).font(.caption)
                    Text(uiState(source["state"].stringValue))
                    if let coverage = source["glucose_summary"]["coverage_fraction"].numberValue { Text(uiText("Observed interval coverage: \((coverage * 100).formatted())%")) }
                }
            }
            Section(uiText("Recorded events")) {
                ForEach(Array(result["events"].arrayValue.enumerated()), id: \.offset) { _, event in
                    VStack(alignment: .leading) {
                        Text(uiDate(Date(timeIntervalSince1970: event["at"].numberValue ?? 0), dateStyle: .omitted, timeStyle: .shortened))
                        Text(event["entry"]["kind"] == .string("nutrition") ? event["entry"]["content"]["food"].stringValue : uiText("Training"))
                        if let carbohydrate = event["entry"]["content"]["carbohydrate_g"].numberValue { Text(uiText("Recorded carbohydrate: \(carbohydrate.formatted()) g")).font(.caption) }
                    }
                }
                NavigationLink(uiText("Correct recorded events"), destination: DailyLogsView(model: model))
            }
            Section { if !result["notes"].arrayValue.isEmpty { InfoHelp(title: uiText("Interpretation"), message: result["notes"].arrayValue.map(\.stringValue).joined(separator: "\n\n")) } }
        }.mobileNavigationTitle(uiText("Meals and glucose")).initialTask(id: date) { await load() }
    }
    private func load() async {
        guard let client = model.client, let token = model.session?.token else { return }
        guard let gap = Double(maximumGap), gap.rounded() == gap, (1...1800).contains(gap) else { failure = "Enter a gap from 1 to 1800 seconds."; return }
        let start = Calendar.current.startOfDay(for: date)
        guard let end = Calendar.current.date(byAdding: .day, value: 1, to: start) else { return }
        do {
            let response = try await model.request("wellness/meal-glucose", body: .object(["start_at": .number(start.timeIntervalSince1970), "end_at": .number(end.timeIntervalSince1970), "maximum_gap_seconds": .number(gap)]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            result = response; failure = nil
        } catch { if !Task.isCancelled, model.session?.token == token { failure = error.localizedDescription } }
    }
}
