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
    @State var creatingMetric: Bool
    @State var metricSelection: String
    @State var newMetricName: String
    @State var newMetricUnit: String
    @State var saving = false
    @State var error: String?
    @State var preview: DocumentLocation?
    @State var submitAsPrinted = false
    @Environment(\.dismiss) var dismiss
    init(observation: JSONValue, metrics: [JSONValue], pageCount: Int, source: @escaping (Int) async throws -> DocumentLocation, save: @escaping (JSONValue) async throws -> Void) {
        self.observation = observation; self.metrics = metrics; self.pageCount = pageCount; self.source = source; self.save = save
        _payload = State(initialValue: observation == .null ? .object(["source": .object(["page": .number(1), "quote": .string("")])]) : observation["payload"])
        let code = observation["lab_test_code"].stringValue
        _printedName = State(initialValue: LaboratoryPresentation.printedName(observation["payload"]["raw_name"].stringValue, code: code))
        _loincCode = State(initialValue: code)
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
            editorSurface
                .interactiveDismissDisabled()
                .alert(uiText("Could not save"), isPresented: Binding(
                    get: { error != nil }, set: { if !$0 { error = nil } }
                )) {
                    Button(uiText("OK"), role: .cancel) { error = nil }
                } message: {
                    Text(uiMessage(error ?? ""))
                }
                .originalReportPreview($preview)
                .alert(uiText("Submit as printed?"), isPresented: $submitAsPrinted) {
                    Button(uiText("Cancel"), role: .cancel) {}
                    Button(uiText("Submit for History")) { Task { await submit(as: "submitted") } }
                } message: { Text(uiText("The result is saved as printed. It may be unavailable in metric charts.")) }
        }
    }
    private var isReview: Bool { observation != .null }

    @ViewBuilder private var editorSurface: some View {
        #if os(Android)
        if isReview {
            VStack(spacing: 0) {
                HStack(spacing: 8) {
                    cancelButton
                    Text(uiText("Review result"))
                        .font(.headline)
                        .lineLimit(1)
                        .frame(maxWidth: .infinity)
                    saveButton
                    if status == "pending" {
                        submitButton.buttonStyle(.borderedProminent)
                    }
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 6)
                .background(.background)
                Divider()
                Form { reviewSections }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            .toolbar(.hidden, for: .navigationBar)
        } else {
            nativeEditor
        }
        #else
        nativeEditor
        #endif
    }

    private var nativeEditor: some View {
        editorForm.toolbar {
            ToolbarItem(placement: .cancellationAction) { cancelButton }
            #if !os(Android)
            if isReview {
                ToolbarItem(placement: .principal) {
                    Text(uiText("Review result"))
                        .font(.headline)
                        .lineLimit(1)
                        .minimumScaleFactor(0.75)
                        .accessibilityAddTraits(.isHeader)
                }
            }
            #endif
            ToolbarItem(placement: .confirmationAction) {
                HStack(spacing: 12) {
                    saveButton
                    #if !os(Android)
                    if isReview && status == "pending" {
                        submitButton.fontWeight(.semibold)
                    }
                    #endif
                }
            }
        }
    }

    @ViewBuilder private var editorForm: some View {
        let form = Form { reviewSections }
        if isReview {
            form.navigationTitle(uiText("Review result")).navigationBarTitleDisplayMode(.inline)
        } else {
            form.mobileNavigationTitle(uiText("Add result"))
        }
    }

    private var cancelButton: some View {
        Button(uiText("Cancel")) { dismiss() }
            .disabled(saving)
    }

    private var saveButton: some View {
        Button(isReview || status != "pending" ? uiText("Save") : uiText("Save Draft")) { Task { await submit(as: status) } }
            .accessibilityIdentifier("save-reviewed-result")
            .disabled(saveDisabled)
    }

    private var submitButton: some View {
        Button(uiText("Submit"), action: submitResult)
            .accessibilityIdentifier("submit-reviewed-result")
            .disabled(saveDisabled)
    }

    private func submitResult() {
        let unitConflict = payload["raw_unit"] == observation["payload"]["raw_unit"]
            && observation["browse_issues"].arrayValue.contains { $0["code"] == .string("unsupported_unit") }
        if !creatingMetric && (metricSelection.isEmpty || unitConflict) {
            submitAsPrinted = true
        } else {
            Task { await submit(as: "submitted") }
        }
    }

    private var status: String {
        let saved = observation["status"].stringValue
        return saved.isEmpty ? "pending" : saved
    }

    private var canMarkAsNew: Bool {
        status == "pending" || (status == "submitted" && ReportReview.savedMetricID(observation) == nil)
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
        Section(uiText("Mapping")) {
            recommendedName
            HStack {
                Text(uiText("Metric maps to"))
                SelectionPicker(title: uiText("Metric maps to"), selection: mapping, placeholder: uiText("Unmapped"),
                    choices: mappingChoices)
                    .labelsHidden()
                    .frame(maxWidth: .infinity, alignment: .leading)
                Button(uiText("Mark as New"), action: markAsNew)
                    .buttonStyle(.bordered)
                    .accessibilityIdentifier("mark-as-new-metric")
                    .disabled(!canMarkAsNew || creatingMetric || saving)
            }
            if creatingMetric {
                TextField(uiText("Metric name"), text: $newMetricName)
                TextField(uiText("Standard unit"), text: $newMetricUnit)
                Button(uiText("Keep unmapped")) { creatingMetric = false }
            } else {
                candidateOptions
            }
            let suggestion = ReportReview.suggestionNote(observation, includeMetricName: true)
            let proposal = observation["payload"]["ocr_new_metric"]
            let proposed = [proposal["name"].stringValue, proposal["standard_unit"].stringValue]
                .filter { !$0.isEmpty }.joined(separator: " · ")
            let note = suggestion.isEmpty && metricSelection.isEmpty ? proposed : suggestion
            if !note.isEmpty {
                Text(uiText("OCR suggestion: \(note)")).font(.caption).foregroundStyle(.secondary)
            }
            mappingIssues
        }
        Section(uiText("As printed on the report")) {
            printedRows
            if !validLOINC { Text(uiText("Enter a LOINC code such as 50560-2.")).font(.caption).foregroundStyle(.red) }
        }
        if !isReview && status == "pending" {
            Section { submitButton }
        }
        Section(uiText("Source")) {
            if isReview {
                Button(uiText("View Original Page")) {
                    Task {
                        do { preview = try await source(Int(payload["source"]["page"].numberValue ?? 1)) }
                        catch { self.error = error.localizedDescription }
                    }
                }
            }
            Stepper("Page \(Int(payload["source"]["page"].numberValue ?? 1))", value: Binding(get: { Int(payload["source"]["page"].numberValue ?? 1) }, set: { payload["source"]["page"] = .number(Double($0)) }), in: 1...max(1, pageCount))
            TextField(uiText("Exact source text"), text: Binding(get: { payload["source"]["quote"].stringValue }, set: { payload["source"]["quote"] = .string($0) }), axis: .vertical)
            field(uiText("Your notes"), "notes")
        }
    }

    @ViewBuilder private var candidateOptions: some View {
        let suggestions = observation["metric_suggestions"].arrayValue
        if !suggestions.isEmpty {
            DisclosureGroup(uiText("Closest matches")) {
                ForEach(Array(suggestions.enumerated()), id: \.offset) { _, suggestion in
                    Button {
                        mapping.wrappedValue = suggestion["metric_id"].stringValue
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
            }
        }
    }

    @ViewBuilder private var printedRows: some View {
        // Fixed fields share the form's lazy list with the candidate rows on Android.
        printedRow(uiText("Metric name"), value: $printedName)
        printedRow(uiText("Lab code"), value: text("raw_lab_code"))
        printedRow(uiText("LOINC code"), value: $loincCode)
        printedRow(uiText("Result"), value: text("raw_result"))
        printedRow(uiText("Unit"), value: text("raw_unit"))
        printedRow(uiText("Reference range"), value: text("reference_range"))
        printedRow(uiText("Report flag"), value: text("report_flag"))
        printedRow(uiText("Collection date"), value: text("sampled_at"))
    }

    @ViewBuilder private func printedRow(_ label: String, value: Binding<String>) -> some View {
        let field = TextField("—", text: value, axis: .vertical)
            .multilineTextAlignment(.trailing)
            .autocorrectionDisabled()
            .accessibilityLabel(Text(label))
        if isReview {
            HStack(alignment: .firstTextBaseline, spacing: 18) {
                Text(label)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(minWidth: 120, alignment: .leading)
                field.frame(maxWidth: .infinity, alignment: .trailing)
            }
            .padding(.vertical, 3)
        } else {
            LabeledContent(label) { field }
        }
    }

    @ViewBuilder private var recommendedName: some View {
        let name = observation["lab_test_name"].stringValue
        if !name.isEmpty && loincCode == observation["lab_test_code"].stringValue {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(uiText("Recommended name (LOINC)")).font(.caption).foregroundStyle(.secondary)
                    Text(name).accessibilityIdentifier("review-result-loinc-name")
                }.frame(maxWidth: .infinity, alignment: .leading)
                Button(uiText("Use")) { printedName = name }
                    .buttonStyle(.bordered)
                    .accessibilityLabel(Text(uiText("Use recommended name")))
                    .accessibilityIdentifier("review-result-use-loinc-name")
                    .disabled(saving || printedName == name)
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
            Text(messages.map(uiMessage).joined(separator: "\n")).font(.caption).foregroundStyle(.secondary)
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
                TextField(uiText("Source name"), text: text("source_name"))
                    .multilineTextAlignment(.trailing).autocorrectionDisabled()
            } label: {
                InfoHelp(title: uiText("Source"), message: uiText("The clinic, laboratory, or organization that issued this report."))
            }
            LabeledContent(uiText("Report Title")) {
                TextField(uiText("Report title"), text: text("report_title"), axis: .vertical)
                    .multilineTextAlignment(.trailing).autocorrectionDisabled()
            }
            Picker(uiText("Fasting"), selection: $fasting) { Text(uiText("Unknown")).tag("unknown"); Text(uiText("Yes")).tag("yes"); Text(uiText("No")).tag("no") }
            field(uiText("Recent exercise"), "recent_exercise"); field(uiText("Illness"), "illness"); field(uiText("Medications"), "medications"); field(uiText("Notes"), "notes")
            if let error { Text(uiMessage(error)).foregroundStyle(.red) }
        }.mobileNavigationTitle(uiText("Report Info")).toolbar {
            ToolbarItem(placement: .cancellationAction) { Button(uiText("Cancel")) { dismiss() }.disabled(saving) }
            ToolbarItem(placement: .confirmationAction) { Button(uiText("Save")) { Task {
                saving = true; defer { saving = false }
                context["fasting"] = fasting == "unknown" ? .null : .bool(fasting == "yes")
                do { try await save(context); dismiss() } catch { self.error = error.localizedDescription }
            } }.disabled(saving) }
        }.interactiveDismissDisabled() }
    }
    private func text(_ key: String) -> Binding<String> { Binding(get: { context[key].stringValue }, set: { context[key] = .optional($0) }) }
    private func field(_ label: String, _ key: String) -> some View { TextField(label, text: text(key), axis: .vertical) }
}
