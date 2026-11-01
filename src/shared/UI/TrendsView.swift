import SwiftUI
#if canImport(Charts)
import Charts
#endif

struct TrendsView: View {
    var model: AppModel
    @State var selected = ""
    @State var comparing = false
    @State var second = ""
    @State var preview: DocumentLocation?
    var body: some View {
        List {
            Section {
                NavigationLink("Cycle and life stages", destination: CycleHistoryView(model: model))
                NavigationLink("Compare measurements", destination: ComparisonView(model: model))
                NavigationLink("Long-term health report", destination: HealthReportView(model: model))
                NavigationLink("Review daily logs", destination: ArchiveReviewView(model: model))
            }
            Section("Choose metrics") {
                picker("Metric", selection: $selected)
                Toggle("Compare a second metric", isOn: $comparing)
                if comparing { picker("Second metric", selection: $second) }
            }
            if !selected.isEmpty { TrendSeries(model: model, metric: selected, openPreview: { preview = $0 }).id(selected) }
            if comparing && !second.isEmpty && second != selected { TrendSeries(model: model, metric: second, openPreview: { preview = $0 }).id(second) }
            RecentActivitySection(model: model)
        }.mobileNavigationTitle("Trends")
            .onChange(of: model.metrics, initial: true) { _, values in
                if !values.contains(where: { $0["metric_id"].stringValue == selected }) { selected = values.first?["metric_id"].stringValue ?? "" }
                if !values.contains(where: { $0["metric_id"].stringValue == second }) { second = values.dropFirst().first?["metric_id"].stringValue ?? selected }
            }
            .sheet(item: $preview) { DocumentPreview(location: $0) }
    }
    private func picker(_ label: String, selection: Binding<String>) -> some View {
        Picker(label, selection: selection) { ForEach(model.metrics, id: \.identifier) { Text($0["name"].stringValue).tag($0["metric_id"].stringValue) } }
    }
}

struct TrendSeries: View {
    var model: AppModel
    let metric: String
    @State var result: JSONValue = .null
    @State var selectedTime: Double?
    let openPreview: (DocumentLocation) -> Void
    var body: some View {
        Group {
            Section(model.metrics.first(where: { $0["metric_id"].stringValue == metric })?["name"].stringValue ?? metric) {
                if result["points"].arrayValue.isEmpty { Text("No comparable results.").foregroundStyle(.secondary); InfoHelp(title: "Comparable results", message: "Confirm a result with a mapped metric, a recognized unit and collection date.") }
                else {
                    #if os(Android)
                    AndroidPointPlot(points: AndroidPlotPoint.values(result["points"].arrayValue, x: "timestamp", y: "value", multiplier: 1), unit: result["points"].arrayValue.first?["unit"].stringValue ?? "", select: { selectedTime = $0.x }, connected: true, events: []).frame(height: 220)
                    #else
                    Chart(result["points"].arrayValue, id: \.identifier) { point in
                        LineMark(x: .value("Collection date", point["timestamp"].numberValue ?? 0), y: .value("Value", point["value"].numberValue ?? 0))
                        PointMark(x: .value("Collection date", point["timestamp"].numberValue ?? 0), y: .value("Value", point["value"].numberValue ?? 0))
                    }.chartXSelection(value: $selectedTime).chartXAxis { AxisMarks { value in AxisValueLabel { if let seconds = value.as(Double.self) { Text(Date(timeIntervalSince1970: seconds), format: .dateTime.year().month()) } } } }.frame(height: 220)
                    #endif
                    InfoHelp(title: result["points"].arrayValue.first?["unit"].stringValue ?? "Source reports", message: "Tap a point or a dated result to open its original report.")
                }
                if let count = result["incomparable_count"].numberValue, count > 0 { Text("\(Int(count)) confirmed results have an unknown date, unsupported unit or non-exact value. They remain in Reports.").font(.caption) }
            }
            Section("Source reports") {
                Button("Refresh history") { Task { await reload() } }
                ForEach(result["points"].arrayValue, id: \.identifier) { point in
                    Button { show(point) } label: {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(point["sampled_at"].stringValue)
                            Text("\(point["value"].numberValue.map { String($0) } ?? point["value"].stringValue) \(point["unit"].stringValue)").font(.headline)
                            Text(LaboratoryPresentation.standardizedReference(point["reference"])).font(.caption)
                            Text(LaboratoryPresentation.originalReference(point["original"], reference: point["reference"])).font(.caption).foregroundStyle(.secondary)
                            Text("Original: \(point["original"]["raw_result"].stringValue) \(point["original"]["raw_unit"].stringValue) · revision \(Int(point["revision"].numberValue ?? 1))").font(.caption).foregroundStyle(.secondary)
                        }
                    }
                }
            }
        }.initialTask(id: metric) { await reload() }
            .onChange(of: selectedTime) { _, time in
                if let time, let point = result["points"].arrayValue.min(by: { abs(($0["timestamp"].numberValue ?? 0) - time) < abs(($1["timestamp"].numberValue ?? 0) - time) }) { show(point); selectedTime = nil }
            }
    }
    private func reload() async { do { if let client = model.client { result = try model.cachedResponse("trends/get", body: .object(["metric_ids": .array([.string(metric)])])) ?? result; result = try await model.request("trends/get", body: .object(["metric_ids": .array([.string(metric)])])) } } catch { model.errorMessage = error.localizedDescription } }
    private func show(_ point: JSONValue) { Task { do { openPreview(DocumentLocation(url: try await model.source(point["report_id"].stringValue), page: Int(point["original"]["source"]["page"].numberValue ?? 1))) } catch { model.errorMessage = error.localizedDescription } } }
}
