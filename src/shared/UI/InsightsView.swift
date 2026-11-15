import SwiftUI

struct InsightPanelSelection: Identifiable {
    let panel: JSONValue
    var id: String { panel["id"].stringValue }
}

/// Both mobile clients render the same server-owned assessment and evidence.
struct InsightsView: View {
    var model: AppModel
    @State var response: JSONValue = .null
    @State var responseRevision: Int?
    @State var loading = false
    @State var refreshRequested = false
    @State var failure: String?
    @State var selectedPanel: InsightPanelSelection?
    private var bodyRequest: JSONValue { .object(["timezone": .string(TimeZone.current.identifier)]) }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                if loading { ProgressView("Reviewing your health data") }
                if let failure { Text(failure).foregroundStyle(.secondary) }
                ForEach(Array((responseRevision == model.reportDataRevision ? response["dashboard"].arrayValue : []).enumerated()), id: \.offset) { _, panel in
                    Button {
                        selectedPanel = InsightPanelSelection(panel: panel)
                    } label: {
                        InsightDashboardCard(model: model, panel: panel, response: response, expanded: false)
                    }.buttonStyle(.plain).accessibilityIdentifier("insight-card-\(panel["id"].stringValue)")
                }
            }.frame(maxWidth: .infinity, alignment: .leading).padding(16)
        }
        .mobileNavigationTitle("Insights")
        .initialTask(id: model.reportDataRevision) { await reload() }
        .refreshable { await reload() }
        .sheet(item: $selectedPanel) { selection in NavigationStack { InsightDetail(model: model, panel: selection.panel, response: response) } }
        .onChange(of: model.sourcePreferences["preferences"]["cardiovascular"]) { _, _ in Task { await reload() } }
        .onChange(of: model.sourcePreferences["preferences"]["reference_profile"]) { _, _ in Task { await reload() } }
        .onChange(of: model.sourcePreferences["preferences"]["sleep_target_minutes"]) { _, _ in Task { await reload() } }
    }

    private func reload() async {
        if loading { refreshRequested = true; return }
        guard let token = model.session?.token else { return }
        let revision = model.reportDataRevision
        loading = true
        defer {
            loading = false
            if refreshRequested {
                refreshRequested = false
                Task { await reload() }
            }
        }
        do {
            if response == .null {
                response = try model.cachedResponse("insights/get", body: bodyRequest) ?? .null
                responseRevision = revision
            }
            let next = try await model.request("insights/get", body: bodyRequest)
            guard !Task.isCancelled, model.session?.token == token, model.reportDataRevision == revision else { return }
            response = next; responseRevision = revision; failure = nil
        } catch { if !Task.isCancelled, model.session?.token == token, model.reportDataRevision == revision { failure = error.localizedDescription } }
    }
}

struct InsightHeading: View {
    let insight: JSONValue
    @State var showingMethod = false
    var body: some View {
        HStack {
            Text(insight["title"].stringValue).font(.title2.bold())
            Spacer()
            Button { showingMethod = true } label: {
                Image(systemName: MobileSymbol.name("info.circle")).frame(minWidth: 44, minHeight: 44)
            }.buttonStyle(.borderless).accessibilityLabel("Assessment method and sources")
        }
        .sheet(isPresented: $showingMethod) {
            NavigationStack {
                List {
                    Text(insight["method"].stringValue)
                    ForEach(Array(insight["references"].arrayValue.enumerated()), id: \.offset) { _, reference in
                        if let url = URL(string: reference["url"].stringValue) {
                            Link(reference["title"].stringValue, destination: url)
                        }
                    }
                }.mobileNavigationTitle("Method & Sources")
                    .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { showingMethod = false } } }
            }
        }
    }
}

struct CardiovascularInsight: View {
    let model: AppModel
    let insight: JSONValue
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            InsightHeading(insight: insight)
            Text(insight["risk_label"].stringValue).font(.subheadline).foregroundStyle(.secondary)
            if let risk = insight["risk_percent"].numberValue {
                Text("\(risk.formatted(.number.precision(.fractionLength(1))))%")
                    .font(.largeTitle.bold())
            } else {
                Text("Not calculated").font(.title2.weight(.semibold))
            }
            Text(insight["explanation"].stringValue).font(.subheadline)
            ForEach(Array(insight["findings"].arrayValue.enumerated()), id: \.offset) { _, finding in
                VStack(alignment: .leading, spacing: 4) {
                    Text(finding["title"].stringValue).font(.headline)
                    Text(finding["text"].stringValue).foregroundStyle(.orange)
                }
            }
            InsightMissing(values: insight["missing"])
            ForEach(insight["actions"].arrayValue, id: \.stringValue) { Text($0.stringValue).font(.subheadline) }
            DisclosureGroup("Measurements used") {
                ForEach(Array(insight["inputs"].arrayValue.enumerated()), id: \.offset) { _, input in
                    VStack(alignment: .leading, spacing: 4) {
                        Text(input["title"].stringValue.isEmpty ? input["metric_id"].stringValue : input["title"].stringValue)
                            .font(.headline).fixedSize(horizontal: false, vertical: true)
                        if let value = input["value"].numberValue {
                            Text("\(value.formatted(.number.precision(.fractionLength(0...2)))) \(HistoricalValue.displayUnit(input["unit"].stringValue, kind: input["metric_id"].stringValue))")
                            Text("\(HistoricalValue.collectionDate(timestamp: input["at"].numberValue, text: input["sampled_at"].stringValue, timeZone: .current)) · \(model.sourceLabel(input["source"].stringValue, name: input["source_name"].stringValue))")
                                .font(.caption).foregroundStyle(.secondary)
                        } else { Text(input["reason"].stringValue).font(.caption).foregroundStyle(.secondary) }
                    }.frame(maxWidth: .infinity, alignment: .leading).padding(.vertical, 6)
                }
            }
            DisclosureGroup("Cardiovascular history") {
                ForEach(Array(insight["cards"].arrayValue.enumerated()), id: \.offset) { _, card in
                    if !card["series"].arrayValue.isEmpty {
                        InsightEvidencePlot(model: model, card: card)
                    }
                }
            }
        }.padding(.vertical, 8)
    }
}

struct RecoveryInsight: View {
    let model: AppModel
    let insight: JSONValue
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            InsightHeading(insight: insight)
            Text(insight["status"].stringValue).font(.title3.weight(.semibold))
            ForEach(insight["actions"].arrayValue, id: \.stringValue) { Text($0.stringValue).font(.subheadline) }
            ForEach(Array(insight["signals"].arrayValue.enumerated()), id: \.offset) { _, signal in
                VStack(alignment: .leading, spacing: 8) {
                    InsightEvidencePlot(model: model, card: signal)
                    HStack {
                        VStack(alignment: .leading) {
                            Text("Previous 28 days").font(.caption).foregroundStyle(.secondary)
                            Text(formatted(signal["baseline"], signal: signal)).font(.headline)
                            Text("\(Int(signal["baseline_days"].numberValue ?? 0)) observed days").font(.caption)
                        }
                        Spacer()
                        VStack(alignment: .trailing) {
                            Text("Last 7 days").font(.caption).foregroundStyle(.secondary)
                            Text(formatted(signal["recent"], signal: signal)).font(.headline)
                            Text("\(Int(signal["recent_days"].numberValue ?? 0)) observed days").font(.caption)
                        }
                    }
                    if signal["cue"] == .string("insufficient_comparable_days") {
                        Text("More comparable days needed").font(.caption).foregroundStyle(.secondary)
                    } else if let change = signal["change_percent"].numberValue {
                        Text("\(change > 0 ? "+" : "")\(change.formatted(.number.precision(.fractionLength(1))))% vs personal baseline")
                            .font(.caption).foregroundStyle(.secondary)
                    }
                }.padding(.vertical, 8)
            }
            ForEach(Array(insight["fitness"].arrayValue.enumerated()), id: \.offset) { _, card in
                InsightEvidencePlot(model: model, card: card)
            }
            InsightMissing(values: insight["missing"])
        }.padding(.vertical, 8)
    }
    private func formatted(_ value: JSONValue, signal: JSONValue) -> String {
        guard let number = value.numberValue else { return "—" }
        if signal["unit"] == .string("seconds") {
            let minutes = Int(number / 60)
            return "\(minutes / 60)h \(minutes % 60)m"
        }
        return "\(number.formatted(.number.precision(.fractionLength(0...1)))) \(HistoricalValue.displayUnit(signal["unit"].stringValue, kind: signal["metric_id"].stringValue))"
    }
}

struct InsightMissing: View {
    let values: JSONValue
    var body: some View {
        if !values.arrayValue.isEmpty {
            DisclosureGroup("What’s missing (\(values.arrayValue.count))") {
                ForEach(values.arrayValue, id: \.stringValue) { Text($0.stringValue).font(.subheadline).padding(.vertical, 4) }
            }
        }
    }
}

struct InsightEvidencePlot: View {
    let model: AppModel
    let card: JSONValue
    private var plotSeries: [JSONValue] {
        guard card["unit"] == .string("seconds") else { return card["series"].arrayValue }
        return card["series"].arrayValue.map { series in
            var result = series
            result["points"] = .array(series["points"].arrayValue.map { point in
                var value = point
                if let seconds = point["value"].numberValue { value["value"] = .number(seconds / 3600) }
                return value
            })
            return result
        }
    }
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(card["title"].stringValue).font(.headline)
            Text(model.sourceLabel(card["source"].stringValue, name: card["source_name"].stringValue)).font(.caption).foregroundStyle(.secondary)
            InsightHistoryPlot(series: plotSeries).frame(height: 140)
            Text(card["unit"] == .string("seconds") ? "hours" : HistoricalValue.displayUnit(card["unit"].stringValue, kind: card["metric_id"].stringValue)).font(.caption).foregroundStyle(.secondary)
        }.padding(.vertical, 6)
    }
}

struct CardiovascularContextEditor: View {
    let model: AppModel
    private let fields = [
        ("known_cvd", "Diagnosed cardiovascular disease"),
        ("diabetes", "Diagnosed diabetes"),
        ("current_smoking", "Currently smoking"),
        ("antihypertensive", "Taking blood pressure medication"),
        ("statin", "Taking a statin"),
        ("egfr_2021_confirmed", "eGFR uses CKD-EPI 2021 creatinine")
    ]
    var body: some View {
        AutoSavedPreferencesForm(model: model, title: "Risk Context") { preferences, update in
            Section {
                ForEach(fields, id: \.0) { field in
                    Picker(field.1, selection: Binding(get: {
                        let value = preferences["cardiovascular"][field.0]
                        return value == .null ? "unknown" : value.boolValue ? "yes" : "no"
                    }, set: { value in
                        var context = preferences["cardiovascular"]
                        if context == .null { context = .object([:]) }
                        context[field.0] = value == "unknown" ? .null : .bool(value == "yes")
                        update(["cardiovascular": context])
                    })) {
                        Text("Not specified").tag("unknown")
                        Text("Yes").tag("yes")
                        Text("No").tag("no")
                    }
                }
            }
            Section {
                InfoHelp(title: "Risk context", message: "Use your current known diagnoses and medicines. Do not infer diabetes from one glucose result, or assume no medication because none was imported. Check the kidney-test report for its eGFR equation. Age and biological sex come from Reference Settings.")
            }
        }
    }
}
