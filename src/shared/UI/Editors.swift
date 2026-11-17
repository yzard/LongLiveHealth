import SwiftUI

struct ObservationEditor: View {
    let observation: JSONValue
    let metrics: [JSONValue]
    let pageCount: Int
    let source: (Int) async throws -> DocumentLocation
    let save: (JSONValue) async throws -> Void
    @State var payload: JSONValue
    @State var printedName: String
    @State var loincCode: String
    @State var choosingMetric: Bool
    @State var creatingMetric: Bool
    @State var metricSelection: String
    @State var newMetricName: String
    @State var newMetricUnit: String
    @State var saving = false
    @State var error: String?
    @State var preview: DocumentLocation?
    @State var confirmAsPrinted = false
    @Environment(\.dismiss) var dismiss
    init(observation: JSONValue, metrics: [JSONValue], pageCount: Int, choosingMetric: Bool, source: @escaping (Int) async throws -> DocumentLocation, save: @escaping (JSONValue) async throws -> Void) {
        self.observation = observation; self.metrics = metrics; self.pageCount = pageCount; self.source = source; self.save = save
        _payload = State(initialValue: observation == .null ? .object(["source": .object(["page": .number(1), "quote": .string("")])]) : observation["payload"])
        let code = observation["lab_test_code"].stringValue
        _printedName = State(initialValue: LaboratoryPresentation.printedName(observation["payload"]["raw_name"].stringValue, code: code))
        _loincCode = State(initialValue: code)
        _choosingMetric = State(initialValue: choosingMetric)
        let savedChoice = observation["payload"]["unmapped_choice"]
        let proposal = savedChoice["kind"] == .string("new")
            ? savedChoice["definition"] : observation["payload"]["ocr_new_metric"]
        _creatingMetric = State(initialValue: savedChoice["kind"] == .string("new"))
        _metricSelection = State(initialValue: ReportReview.defaultMetricID(observation) ?? "")
        let originalName = LaboratoryPresentation.printedName(observation["payload"]["raw_name"].stringValue, code: code)
        _newMetricName = State(initialValue: proposal["name"].stringValue.isEmpty ? originalName : proposal["name"].stringValue)
        _newMetricUnit = State(initialValue: proposal == .null ? observation["payload"]["raw_unit"].stringValue : proposal["standard_unit"].stringValue)
    }
    var body: some View {
        NavigationStack {
            Form {
                if choosingMetric {
                    metricOptions
                } else {
                    reviewSections
                }
            }.mobileNavigationTitle(choosingMetric ? "Choose a Browse metric" : observation == .null ? "Add result" : "Review result")
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() }.disabled(saving) }
                    if !choosingMetric {
                        ToolbarItem(placement: .confirmationAction) {
                            Button(status == "pending" ? "Save Draft" : "Save") { Task { await submit(as: status) } }.disabled(saveDisabled)
                        }
                    }
                }.interactiveDismissDisabled()
                .alert("Could not save", isPresented: Binding(
                    get: { error != nil }, set: { if !$0 { error = nil } }
                )) {
                    Button("OK", role: .cancel) { error = nil }
                } message: {
                    Text(error ?? "")
                }
                .originalReportPreview($preview)
                .alert("Confirm as printed?", isPresented: $confirmAsPrinted) {
                    Button("Cancel", role: .cancel) {}
                    Button("Confirm for History") { Task { await submit(as: "confirmed") } }
                } message: { Text("The result is saved as printed. It may be unavailable in metric charts.") }
        }
    }
    private var status: String {
        let saved = observation["status"].stringValue
        return saved.isEmpty ? "pending" : saved
    }

    private var canMarkAsNew: Bool {
        status == "pending" || (status == "confirmed" && ReportReview.savedMetricID(observation) == nil)
    }

    private var saveDisabled: Bool {
        saving || !validLOINC || printedName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || payload["raw_result"].stringValue.isEmpty
            || (creatingMetric && (newMetricName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty))
    }

    private var validLOINC: Bool {
        let code = loincCode.trimmingCharacters(in: .whitespacesAndNewlines)
        if code.isEmpty { return true }
        let parts = code.components(separatedBy: "-")
        return parts.count == 2 && (1...7).contains(parts[0].count) && parts[1].count == 1
            && parts.allSatisfy { part in part.allSatisfy { "0123456789".contains($0) } }
    }

    private func submit(as nextStatus: String) async {
        guard !saving else { return }
        saving = true
        defer { saving = false }
        do {
            let originalCode = observation["lab_test_code"].stringValue
            let originalName = LaboratoryPresentation.printedName(payload["raw_name"].stringValue, code: originalCode)
            var reviewedPayload = payload
            reviewedPayload["metric_id"] = .optional(metricSelection)
            if creatingMetric {
                reviewedPayload["unmapped_choice"] = .object([
                    "kind": .string("new"),
                    "definition": .object(["name": .string(newMetricName), "standard_unit": .string(newMetricUnit)]),
                ])
            } else {
                reviewedPayload["unmapped_choice"] = metricSelection.isEmpty
                    ? .object(["kind": .string("unmapped")]) : .null
            }
            if printedName != originalName || loincCode != originalCode {
                let code = loincCode.trimmingCharacters(in: .whitespacesAndNewlines)
                reviewedPayload["raw_name"] = .string(code.isEmpty ? printedName : "\(printedName) (\(code))")
            }
            let change: JSONValue = .object([
                "observation_id": observation["observation_id"],
                "expected_revision": observation["revision"],
                "status": .string(nextStatus),
                "payload": reviewedPayload,
            ])
            try await save(change)
            dismiss()
        } catch { self.error = error.localizedDescription }
    }

    @ViewBuilder private var reviewSections: some View {
        Section("Mapping") {
            recommendedName
            HStack {
                Text("Metric maps to")
                SelectionPicker(title: "Metric maps to", selection: mapping, placeholder: "Unmapped",
                    choices: mappingChoices)
                    .labelsHidden()
                    .frame(maxWidth: .infinity, alignment: .leading)
                Button("Mark as New", action: markAsNew)
                    .buttonStyle(.bordered)
                    .accessibilityIdentifier("mark-as-new-metric")
                    .disabled(!canMarkAsNew || creatingMetric || saving)
            }
            if creatingMetric {
                TextField("Metric name", text: $newMetricName)
                TextField("Standard unit", text: $newMetricUnit)
                Button("Keep unmapped") { creatingMetric = false }
            } else if !observation["metric_suggestions"].arrayValue.isEmpty {
                Button(suggestedMetricID == nil ? "Closest matches" : "Possible match") {
                    choosingMetric = true
                }.font(.caption)
            }
            let suggestion = ReportReview.suggestionNote(observation)
            if !suggestion.isEmpty {
                Text("OCR suggestion: \(suggestion)").font(.caption).foregroundStyle(.secondary)
            }
            mappingIssues
        }
        Section("As printed on the report") {
            TextField("Name", text: $printedName, axis: .vertical).autocorrectionDisabled()
            TextField("LOINC code", text: $loincCode).autocorrectionDisabled()
            if !validLOINC { Text("Enter a LOINC code such as 50560-2.").font(.caption).foregroundStyle(.red) }
            field("Result, including < or >", "raw_result"); field("Unit", "raw_unit")
            field("Reference range", "reference_range"); field("Report flag", "report_flag")
            field("Collection date: YYYY-MM-DD or ISO timestamp", "sampled_at")
        }
        if status == "pending" {
            Section {
                Button("Confirm") {
                    let unitConflict = payload["raw_unit"] == observation["payload"]["raw_unit"]
                        && observation["browse_issues"].arrayValue.contains { $0["code"] == .string("unsupported_unit") }
                    if !creatingMetric && (metricSelection.isEmpty || unitConflict) { confirmAsPrinted = true }
                    else { Task { await submit(as: "confirmed") } }
                }
                    .accessibilityIdentifier("confirm-reviewed-result")
                    .disabled(saveDisabled)
            }
        }
        Section("Source") {
            if observation != .null {
                Button("View Original Page") {
                    Task {
                        do { preview = try await source(Int(payload["source"]["page"].numberValue ?? 1)) }
                        catch { self.error = error.localizedDescription }
                    }
                }
            }
            Stepper("Page \(Int(payload["source"]["page"].numberValue ?? 1))", value: Binding(get: { Int(payload["source"]["page"].numberValue ?? 1) }, set: { payload["source"]["page"] = .number(Double($0)) }), in: 1...max(1, pageCount))
            TextField("Exact source text", text: Binding(get: { payload["source"]["quote"].stringValue }, set: { payload["source"]["quote"] = .string($0) }), axis: .vertical)
            field("Your notes", "notes")
        }
    }

    @ViewBuilder private var metricOptions: some View {
        Section("As printed on the report") {
            Text(printedName)
            if !loincCode.isEmpty {
                Text("LOINC \(loincCode)").font(.caption).foregroundStyle(.secondary)
            }
            recommendedName
            Text([payload["raw_result"].stringValue, payload["raw_unit"].stringValue]
                .filter { !$0.isEmpty }.joined(separator: " "))
            mappingIssues
        }
        Section {
            Button("Mark as New", action: markAsNew)
                .accessibilityIdentifier("create-browse-metric")
                .disabled(!canMarkAsNew)
            let proposal = observation["payload"]["ocr_new_metric"]
            if proposal != .null {
                let proposed = [proposal["name"].stringValue, proposal["standard_unit"].stringValue]
                    .filter { !$0.isEmpty }.joined(separator: " · ")
                Text("OCR suggestion: \(proposed)").font(.caption).foregroundStyle(.secondary)
            }
        }
        Section("Closest matches") {
            let suggestions = observation["metric_suggestions"].arrayValue
            if suggestions.isEmpty {
                Text("No similar Browse metrics found.").foregroundStyle(.secondary)
            }
            ForEach(Array(suggestions.enumerated()), id: \.offset) { _, suggestion in
                Button {
                    mapping.wrappedValue = suggestion["metric_id"].stringValue
                    creatingMetric = false
                    choosingMetric = false
                } label: {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(suggestion["name"].stringValue)
                        Text([suggestion["standard_unit"].stringValue, matchLabel(suggestion["unit_match"].stringValue),
                              suggestion["metric_id"] == observation["payload"]["ocr_metric_candidate"] ? "OCR suggestion" : ""]
                            .filter { !$0.isEmpty }.joined(separator: " · "))
                            .font(.caption).foregroundStyle(.secondary)
                    }
                }
            }
            Button("Review with the full metric list") { choosingMetric = false }
        }
    }

    @ViewBuilder private var recommendedName: some View {
        let name = observation["lab_test_name"].stringValue
        if !name.isEmpty && loincCode == observation["lab_test_code"].stringValue {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Recommended name (LOINC)").font(.caption).foregroundStyle(.secondary)
                    Text(name).accessibilityIdentifier("review-result-loinc-name")
                }.frame(maxWidth: .infinity, alignment: .leading)
                if !choosingMetric {
                    Button("Use") { printedName = name }
                        .buttonStyle(.bordered)
                        .accessibilityLabel(Text("Use recommended name"))
                        .accessibilityIdentifier("review-result-use-loinc-name")
                        .disabled(saving || printedName == name)
                }
            }
        }
    }

    @ViewBuilder private var mappingIssues: some View {
        let messages = observation["browse_issues"].arrayValue
            .filter { issue in
                let code = issue["code"].stringValue
                if suggestedMetricID != nil && ["unmapped_metric", "metric_identity_uncertain"].contains(code) {
                    return false
                }
                return issue["scope"] == .string("browse") || issue["scope"] == .string("review")
                    || ["unsupported_unit", "missing_unit"].contains(code)
            }
            .map { $0["message"].stringValue }
        if !messages.isEmpty {
            Text(messages.joined(separator: "\n")).font(.caption).foregroundStyle(.secondary)
        }
    }

    private var suggestedMetricID: String? { ReportReview.suggestedMetricID(observation) }

    private var mappingChoices: [MenuChoice] {
        var ids: Set<String> = []
        return (metrics + observation["metric_suggestions"].arrayValue).compactMap { metric -> MenuChoice? in
            let id = metric["metric_id"].stringValue
            guard !id.isEmpty, ids.insert(id).inserted else { return nil }
            return MenuChoice(id: id, title: metric["name"].stringValue)
        }.sorted { left, right in
            LaboratoryPresentation.nameSortsBefore(left.title, id: left.id, right.title, otherID: right.id)
        }
    }

    private var mapping: Binding<String> {
        Binding(get: { metricSelection }, set: { value in
            metricSelection = value
            creatingMetric = false
        })
    }

    private func markAsNew() {
        metricSelection = ""
        if newMetricName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            newMetricName = printedName
        }
        creatingMetric = true
        choosingMetric = false
    }

    private func matchLabel(_ match: String) -> String {
        switch match {
        case "exact": "Unit matches"
        case "convertible": "Unit can be converted"
        case "incompatible": "Unit does not match"
        default: "Unit not reported"
        }
    }

    private func text(_ key: String) -> Binding<String> { Binding(get: { payload[key].stringValue }, set: { payload[key] = key == "raw_result" ? .string($0) : .optional($0) }) }
    private func field(_ label: String, _ key: String) -> some View { TextField(label, text: text(key), axis: .vertical).autocorrectionDisabled() }
}

struct ContextEditor: View {
    let save: (JSONValue) async throws -> Void
    @State var context: JSONValue
    @State var fasting: String
    @State var error: String?
    @State var saving = false
    @Environment(\.dismiss) var dismiss
    init(context: JSONValue, save: @escaping (JSONValue) async throws -> Void) {
        self.save = save; _context = State(initialValue: context)
        _fasting = State(initialValue: context["fasting"] == .null ? "unknown" : (context["fasting"].boolValue ? "yes" : "no"))
    }
    var body: some View {
        NavigationStack { Form {
            LabeledContent {
                TextField("Source name", text: text("source_name"))
                    .multilineTextAlignment(.trailing).autocorrectionDisabled()
            } label: {
                InfoHelp(title: "Source", message: "The clinic, laboratory, or organization that issued this report.")
            }
            LabeledContent("Report Title") {
                TextField("Report title", text: text("report_title"), axis: .vertical)
                    .multilineTextAlignment(.trailing).autocorrectionDisabled()
            }
            Picker("Fasting", selection: $fasting) { Text("Unknown").tag("unknown"); Text("Yes").tag("yes"); Text("No").tag("no") }
            field("Recent exercise", "recent_exercise"); field("Illness", "illness"); field("Medications", "medications"); field("Notes", "notes")
            if let error { Text(error).foregroundStyle(.red) }
        }.mobileNavigationTitle("Report Info").toolbar {
            ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() }.disabled(saving) }
            ToolbarItem(placement: .confirmationAction) { Button("Save") { Task {
                saving = true; defer { saving = false }
                context["fasting"] = fasting == "unknown" ? .null : .bool(fasting == "yes")
                do { try await save(context); dismiss() } catch { self.error = error.localizedDescription }
            } }.disabled(saving) }
        }.interactiveDismissDisabled() }
    }
    private func text(_ key: String) -> Binding<String> { Binding(get: { context[key].stringValue }, set: { context[key] = .optional($0) }) }
    private func field(_ label: String, _ key: String) -> some View { TextField(label, text: text(key), axis: .vertical) }
}
