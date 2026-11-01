import SwiftUI
#if canImport(Charts)
import Charts
#endif

struct HRVWindowsView: View {
    var model: AppModel
    @State var result: JSONValue = .null
    @State var failure: String?
    @State var loading = true
    var body: some View {
        List {
            Section {
                InfoHelp(title: "HRV windows", message: "Five-minute normal-to-normal intervals. SDNN stays a separate measurement.")
                InfoHelp(title: "Quality and baseline", message: "Unknown quality and gaps exclude a window. Device or protocol changes start a separate baseline.")
                if let failure { Text(failure).foregroundStyle(.secondary) }
                if loading { ProgressView("Loading HRV windows") }
                else if failure == nil && result["sources"].arrayValue.isEmpty { Text("No archived NN or heartbeat windows in the last 90 days.") }
            }
            ForEach(Array(result["sources"].arrayValue.enumerated()), id: \.offset) { _, source in
                Section(model.sourceLabel(source["source"].stringValue)) {
                    ForEach(["device_model", "firmware", "protocol_id", "context", "posture"], id: \.self) { key in
                        LabeledContent(key.replacingOccurrences(of: "_", with: " ").capitalized, value: source["protocol"][key].stringValue.isEmpty ? "Unknown" : source["protocol"][key].stringValue).font(.caption)
                    }
                    #if os(Android)
                    AndroidPointPlot(points: AndroidPlotPoint.values(source["days"].arrayValue, x: "date", y: "ln_rmssd", multiplier: 1), unit: "lnRMSSD", select: nil, connected: false, events: []).frame(height: 200)
                    #else
                    Chart {
                        ForEach(Array(source["days"].arrayValue.enumerated()), id: \.offset) { _, day in
                            if let value = day["ln_rmssd"].numberValue {
                                PointMark(x: .value("Date", day["date"].stringValue), y: .value("lnRMSSD", value))
                            }
                        }
                    }.frame(height: 200).accessibilityLabel("Daily lnRMSSD, natural logarithm of RMSSD in milliseconds")
                    #endif
                    ForEach(Array(source["days"].arrayValue.enumerated()), id: \.offset) { _, day in
                        VStack(alignment: .leading) {
                            Text(day["date"].stringValue + " · lnRMSSD " + (day["ln_rmssd"].numberValue?.formatted() ?? "Unknown"))
                            if let week = day["seven_day_median"].numberValue { Text("Seven-day median: \(week.formatted())").font(.caption) }
                            Text("Baseline days: \(Int(day["baseline_days"].numberValue ?? 0))/28").font(.caption)
                            if let median = day["baseline"]["median"].numberValue { Text("Prior median: \(median.formatted())").font(.caption) }
                        }
                    }
                    DisclosureGroup("Source windows and exclusion reasons") {
                        ForEach(Array(source["windows"].arrayValue.enumerated()), id: \.offset) { _, window in
                            VStack(alignment: .leading) {
                                Text(window["date"].stringValue + " · " + window["state"].stringValue)
                                Text(window["reason"].stringValue).font(.caption)
                                Text("Record: " + window["record_id"].stringValue).font(.caption)
                            }
                        }
                    }
                }
            }
        }.mobileNavigationTitle("HRV windows").initialTask { await load() }.refreshable { await load() }
    }
    private func load() async {
        guard let client = model.client, let token = model.session?.token else { return }
        loading = true
        defer { loading = false }
        do {
            let now = floor(Date().timeIntervalSince1970)
            let response = try await model.request("wellness/hrv", body: .object(["start_at": .number(now - 90 * 86400), "end_at": .number(now), "timezone": .string(TimeZone.current.identifier)]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            result = response; failure = nil
        } catch { if !Task.isCancelled, model.session?.token == token { failure = error.localizedDescription } }
    }
}
