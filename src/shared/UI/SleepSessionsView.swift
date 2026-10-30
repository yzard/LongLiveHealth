import SwiftUI
#if canImport(Charts)
import Charts
#endif

struct SleepSessionsView: View {
    var model: AppModel
    @State var result: JSONValue = .null
    @State var regularity: JSONValue = .null
    @State var correction: SleepCorrectionSelection?
    @State var selectedDate = Date()
    @State var period: ChartPeriod = .month
    @State var loading = false
    private var selectedInterval: DateInterval { period.interval(containing: selectedDate, calendar: .current) }
    private let stages = ["In bed", "Asleep, unspecified", "Awake", "Core", "Deep", "REM"]
    var body: some View {
        MetricDetailScreen(title: "Sleep") {
            MetricChartSection {
                Picker("Time range", selection: $period) {
                    ForEach(ChartPeriod.allCases.filter { $0 != .hour }, id: \.self) { Text($0.rawValue).tag($0) }
                }.pickerStyle(.segmented)
                DatePicker("Date", selection: $selectedDate, displayedComponents: [.date])
                if loading { ProgressView("Loading selected period") }
                InfoHelp(title: "Sleep observations", message: "Gaps do not establish wakefulness. Confirm main sleep or naps without replacing source observations.")
            }
            ForEach(Array(result["sources"].arrayValue.enumerated()), id: \.offset) { _, source in
                Section(model.sourceLabel(source["source"].stringValue)) {
                    ForEach(Array(source["sessions"].arrayValue.enumerated()), id: \.offset) { _, session in
                        DisclosureGroup(Date(timeIntervalSince1970: session["end_at"].numberValue ?? 0).formatted()) {
                            if let seconds = session["asleep_seconds"].numberValue {
                                Text("\(OverviewSummary.duration(seconds)) asleep").font(.headline)
                            } else { Text(session["state"].stringValue.replacingOccurrences(of: "_", with: " ")) }
                            if let efficiency = session["efficiency"].numberValue {
                                Text("Observed sleep efficiency: \((efficiency * 100).formatted())%")
                            } else { Text("Efficiency needs complete in-bed evidence.").font(.caption) }
                            Text(session["classification"].stringValue.replacingOccurrences(of: "_", with: " "))
                            Text(session["correction_state"].stringValue.replacingOccurrences(of: "_", with: " ")).font(.caption)
                            if let seconds = session["user_asleep_seconds"].numberValue { Text("User estimate: \(OverviewSummary.duration(seconds))") }
                            if session["corrections"].arrayValue.count <= 1 {
                                Button("Confirm or correct sleep") { correction = SleepCorrectionSelection(source: source["source"].stringValue, session: session) }
                            } else { Text("Delete duplicate corrections in Daily logs before confirming again.").font(.caption) }
                            BrowseChart(timestamps: session["timeline"].arrayValue.flatMap { [$0["start_at"].numberValue, $0["end_at"].numberValue].compactMap { $0 } }, unit: "Sleep stage", initialPeriod: .day) { range, _ in
                            #if os(Android)
                            AndroidSleepPlot(records: session["timeline"].arrayValue, range: range)
                            #else
                            Chart {
                                ForEach(Array(session["timeline"].arrayValue.enumerated().filter { ($0.element["end_at"].numberValue ?? 0) >= range.lowerBound && ($0.element["start_at"].numberValue ?? 0) <= range.upperBound }), id: \.offset) { _, record in
                                    BarMark(xStart: .value("Start", Date(timeIntervalSince1970: record["start_at"].numberValue ?? 0)), xEnd: .value("End", Date(timeIntervalSince1970: record["end_at"].numberValue ?? 0)), y: .value("Stage", stage(record)))
                                        .foregroundStyle(by: .value("Stage", stage(record)))
                                }
                            }
                            #endif
                            }

                            ForEach(Array(session["timeline"].arrayValue.enumerated()), id: \.offset) { _, record in
                                VStack(alignment: .leading) {
                                    Text(stage(record))
                                    Text("\(Date(timeIntervalSince1970: record["start_at"].numberValue ?? 0).formatted()) to \(Date(timeIntervalSince1970: record["end_at"].numberValue ?? 0).formatted())").font(.caption)
                                }
                            }
                        }
                    }
                }
            }
            if result != .null && result["sources"].arrayValue.isEmpty { Text("No supported sleep records in this range.") }
            Section("About this metric") {
                MetricAboutView(id: "sleep_sessions", name: "Sleep sessions")
            }
            Section("Seven-day sleep regularity") {
                InfoHelp(title: "Sleep regularity", message: "Every minute needs an explicit sleep or wake state. Missing daytime intervals are unknown.")
                ForEach(Array(regularity["sources"].arrayValue.enumerated()), id: \.offset) { _, source in
                    VStack(alignment: .leading) {
                        Text(model.sourceLabel(source["source"].stringValue))
                        Text(source["value"].numberValue.map { "\($0.formatted()) SRI" } ?? "Not available").font(.headline)
                        Text("\(Int(source["known_minutes"].numberValue ?? 0))/10080 known minute states").font(.caption)
                        Text(source["state"].stringValue.replacingOccurrences(of: "_", with: " ")).font(.caption)
                    }
                }
                if !regularity["notes"].arrayValue.isEmpty { InfoHelp(title: "Regularity method", message: regularity["notes"].arrayValue.map(\.stringValue).joined(separator: "\n\n")) }
            }
            Section("Method") {
                InfoHelp(title: "Session grouping", message: "Sessions group intervals separated by at most 90 minutes. This engineering threshold does not classify main sleep or naps.")
                Text(result["algorithm_version"].stringValue).font(.caption)
            }
        }.task(id: selectedInterval) { await reload() }.refreshable { await reload() }
            .onChange(of: model.entryMutation) { _, _ in Task { await reload() } }
            .sheet(item: $correction) { selection in
                NavigationStack { SleepCorrectionEditor(model: model, selection: selection) }
            }
    }
    private func stage(_ record: JSONValue) -> String {
        let index = Int(record["category"].numberValue ?? -1)
        return stages.indices.contains(index) ? stages[index] : "Unknown"
    }
    private func reload() async {
        guard let client = model.client, let token = model.session?.token else { return }
        do {
            loading = true
            defer { loading = false }
            let now = floor(selectedInterval.end.timeIntervalSince1970)
            let response = try await model.request("wellness/sleep", body: .object(["start_at": .number(selectedInterval.start.timeIntervalSince1970), "end_at": .number(now), "timezone": .string(TimeZone.current.identifier)]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            result = response
            regularity = .null
            let end = floor(now / 60) * 60
            let index = try await model.request("wellness/sleep/regularity", body: .object(["start_at": .number(end - 7 * 86400), "end_at": .number(end), "timezone": .string(TimeZone.current.identifier)]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            regularity = index
        } catch { if !Task.isCancelled, model.session?.token == token { model.errorMessage = error.localizedDescription } }
    }
}


struct SleepCorrectionSelection: Identifiable {
    let id = UUID()
    let source: String
    let session: JSONValue
}

struct SleepCorrectionEditor: View {
    var model: AppModel
    let selection: SleepCorrectionSelection
    @Environment(\.dismiss) var dismiss
    @State var classification = "unclassified_session"
    @State var minutes = ""
    @State var note = ""
    @State var recordID = UUID().uuidString
    @State var failure: String?
    @State var saving = false
    @State var initialized = false
    var body: some View {
        Form {
            InfoHelp(title: "Sleep estimate", message: "Source intervals remain unchanged. Your estimate does not create sleep stages or change SRI.")
            Picker("Classification", selection: $classification) {
                Text("Unclassified").tag("unclassified_session")
                Text("Main sleep").tag("main_sleep")
                Text("Nap").tag("nap")
            }
            TextField("User estimated sleep minutes (optional)", text: $minutes).keyboardType(.decimalPad)
            TextField("Reason or context", text: $note, axis: .vertical)
            if let failure { Text(failure).foregroundStyle(.secondary) }
            Button("Save sleep confirmation") { Task { await save() } }.disabled(saving || note.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
        }.navigationTitle("Sleep confirmation")
            .toolbar { Button("Cancel") { dismiss() } }
            .interactiveDismissDisabled()
            .onAppear {
                guard !initialized else { return }; initialized = true
                classification = selection.session["classification"].stringValue
                if let existing = selection.session["corrections"].arrayValue.first {
                    recordID = existing["record_id"].stringValue
                    minutes = existing["entry"]["content"]["corrected_asleep_minutes"].numberValue.map { String($0) } ?? ""
                    note = existing["entry"]["content"]["note"].stringValue
                }
            }
    }
    private func save() async {
        guard !saving, let client = model.client, let token = model.session?.token else { return }
        saving = true
        defer { saving = false }
        do {
            var estimate: JSONValue = .null
            if !minutes.isEmpty {
                guard let value = Double(minutes), value.isFinite else { throw DraftError.message("Enter valid sleep minutes.") }
                estimate = .number(value)
            }
            var basis: [String: JSONValue] = [:]
            for stage in selection.session["timeline"].arrayValue { basis[stage["record_id"].stringValue] = stage["version"] }
            let content: JSONValue = .object(["source": .string(selection.source), "session_start": selection.session["start_at"], "session_end": selection.session["end_at"], "classification": .string(classification), "corrected_asleep_minutes": estimate, "basis_revisions": .object(basis), "note": .string(note)])
            let version = (selection.session["corrections"].arrayValue.first?["version"].numberValue ?? 0) + 1
            _ = try await model.request("wellness/entries/save", body: .object(["record_id": .string(recordID), "version": .number(version), "batch_id": .string(UUID().uuidString), "at": selection.session["start_at"], "timezone": .string(TimeZone.current.identifier), "entry": .object(["kind": .string("sleep_correction"), "content": content])]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            dismiss()
        } catch {
            guard !Task.isCancelled, model.session?.token == token else { return }
            failure = error.localizedDescription
        }
    }
}
