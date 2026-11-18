import SwiftUI

struct ObservationEditor: View {
    let observation: JSONValue
    let metrics: [JSONValue]
    let pageCount: Int
    let source: (Int) async throws -> DocumentLocation
    let lookupLOINCName: (String) async throws -> String
    let save: (JSONValue) async throws -> Void
    @State var payload: JSONValue
    @State var printedName: String
    @State var loincCode: String
    @State var metricSelection: String
    @State var newMetricName: String
    @State var newMetricUnit: String
    @State var newMetricLOINC: String
    @State var saving = false
    @State var error: String?
    @State var preview: DocumentLocation?
    @State var submitAsPrinted = false
    @State var lookedUpLOINCNames: [String: String] = [:]
    @State var unavailableLOINCCode: String?
    @Environment(\.dismiss) var dismiss
    init(observation: JSONValue, metrics: [JSONValue], pageCount: Int,
         source: @escaping (Int) async throws -> DocumentLocation,
         lookupLOINCName: @escaping (String) async throws -> String,
         save: @escaping (JSONValue) async throws -> Void) {
        self.observation = observation; self.metrics = metrics; self.pageCount = pageCount; self.source = source
        self.lookupLOINCName = lookupLOINCName; self.save = save
        _payload = State(initialValue: observation == .null ? .object(["source": .object(["page": .number(1), "quote": .string("")])]) : observation["payload"])
        let code = observation["lab_test_code"].stringValue
        let originalName = LaboratoryPresentation.printedName(observation["payload"]["raw_name"].stringValue, code: code)
        _printedName = State(initialValue: originalName)
        _loincCode = State(initialValue: code)
        let savedChoice = observation["payload"]["unmapped_choice"]
        let proposal = savedChoice["kind"] == .string("new")
            ? savedChoice["definition"] : observation["payload"]["ocr_new_metric"]
        _metricSelection = State(initialValue: ReportReview.defaultMetricID(observation) ?? "")
        _newMetricName = State(initialValue: proposal["name"].stringValue.isEmpty ? originalName : proposal["name"].stringValue)
        _newMetricUnit = State(initialValue: proposal == .null ? observation["payload"]["raw_unit"].stringValue : proposal["standard_unit"].stringValue)
        _newMetricLOINC = State(initialValue: proposal["loinc_code"] == .null ? code : proposal["loinc_code"].stringValue)
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
        .task(id: newDatabaseLOINCCode) { await loadNewLOINCName() }
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
        if !isNewMetric && unitConflict {
            submitAsPrinted = true
        } else {
            Task { await submit(as: "submitted") }
        }
    }

    private var status: String {
        let saved = observation["status"].stringValue
        return saved.isEmpty ? "pending" : saved
    }

    private var isNewMetric: Bool { metricSelection.isEmpty }

    private var saveDisabled: Bool {
        saving || (!isReview && !validLOINC(loincCode)) || printedName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || payload["raw_result"].stringValue.isEmpty
            || (isNewMetric && (!validLOINC(newMetricLOINC) || newMetricName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty))
    }

    private func validLOINC(_ value: String) -> Bool {
        let code = value.trimmingCharacters(in: .whitespacesAndNewlines)
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
            var reviewedPayload = payload
            reviewedPayload["metric_id"] = .optional(metricSelection)
            if isNewMetric {
                reviewedPayload["unmapped_choice"] = .object([
                    "kind": .string("new"),
                    "definition": .object([
                        "name": .string(newMetricName),
                        "standard_unit": .string(newMetricUnit),
                        "loinc_code": .string(newMetricLOINC.trimmingCharacters(in: .whitespacesAndNewlines)),
                    ]),
                ])
            } else {
                reviewedPayload["unmapped_choice"] = .null
            }
            if !isReview {
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
            let choices = mappingChoices
            let candidate = observation["payload"]["ocr_metric_candidate"].stringValue
            let recommendation = choices.first { $0.id == candidate }?.id
            HStack(spacing: 8) {
                SelectionPicker(title: uiText("Maps to"), selection: $metricSelection, placeholder: uiText("New"),
                    choices: choices)
                    .labelsHidden()
                    .lineLimit(1)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .accessibilityIdentifier("metric-mapping-picker")
                Button(uiText("Suggested")) {
                    if let recommendation { metricSelection = recommendation }
                }
                .buttonStyle(.bordered)
                .fixedSize(horizontal: true, vertical: false)
                .accessibilityIdentifier("use-suggested-metric")
                .disabled(recommendation == nil || saving)
                Button(uiText("New")) { metricSelection = "" }
                    .buttonStyle(.bordered)
                    .fixedSize(horizontal: true, vertical: false)
                    .accessibilityIdentifier("mark-as-new-metric")
                    .disabled(isNewMetric || saving)
            }
            if isReview {
                mappingComparison
            } else {
                manualDatabaseRows
            }
            if isNewMetric && !validLOINC(newMetricLOINC) {
                Text(uiText("Enter a LOINC code such as 50560-2.")).font(.caption).foregroundStyle(.red)
            }
            candidateOptions
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
        Section(isReview ? uiText("OCR Results") : uiText("Result details")) {
            printedRows
            if !isReview && !validLOINC(loincCode) {
                Text(uiText("Enter a LOINC code such as 50560-2.")).font(.caption).foregroundStyle(.red)
            }
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
                        metricSelection = suggestion["metric_id"].stringValue
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
        if !isReview {
            resultRow(uiText("Metric name"), value: $printedName)
        }
        resultRow(uiText("Lab code"), value: text("raw_lab_code"))
        if !isReview {
            resultRow(uiText("LOINC code"), value: $loincCode)
            resultRow(uiText("Result"), value: text("raw_result"))
            resultRow(uiText("Unit"), value: text("raw_unit"))
        }
        resultRow(uiText("Reference range"), value: text("reference_range"))
        resultRow(uiText("Report flag"), value: text("report_flag"))
        resultRow(uiText("Collection date"), value: text("sampled_at"))
    }

    private func resultRow(_ label: String, value: Binding<String>) -> some View {
        printedRow(label, value: value, readOnly: isReview ? value.wrappedValue : nil)
    }

    private var databaseMetric: JSONValue {
        metrics.first { $0["metric_id"] == .string(metricSelection) }
            ?? observation["metric_suggestions"].arrayValue.first { $0["metric_id"] == .string(metricSelection) }
            ?? .null
    }

    @ViewBuilder private var mappingComparison: some View {
        let metric = databaseMetric
        let entries = metric["loinc"].arrayValue
        let codes = entries.map { $0["code"].stringValue }.joined(separator: ", ")
        let names = isNewMetric ? newDatabaseLOINCName : entries
            .filter { !$0["code"].stringValue.isEmpty }
            .map { $0["official_name"].stringValue }
            .filter { !$0.isEmpty }.joined(separator: "\n")
        comparisonRow("", ocr: Text("OCR").font(.subheadline.weight(.semibold)),
                      database: Text(uiText("Database")).font(.subheadline.weight(.semibold)))
        comparisonRow(uiText("Name"),
                      ocr: readOnlyValue(printedName).accessibilityIdentifier("ocr-metric-name"),
                      database: editorField(uiText("Name"), value: $newMetricName,
                          readOnly: isNewMetric ? nil : metric["name"].stringValue, identifier: "database-metric-name"))
        comparisonRow(uiText("Unit"),
                      ocr: readOnlyValue(payload["raw_unit"].stringValue).accessibilityIdentifier("ocr-metric-unit"),
                      database: editorField(uiText("Unit"), value: $newMetricUnit,
                          readOnly: isNewMetric ? nil : metric["standard_unit"].stringValue, identifier: "database-metric-unit"))
        comparisonRow(uiText("LOINC code"),
                      ocr: readOnlyValue(loincCode).accessibilityIdentifier("ocr-loinc-code"),
                      database: editorField(uiText("LOINC code"), value: $newMetricLOINC,
                          readOnly: isNewMetric ? nil : codes, identifier: "database-metric-loinc"))
        comparisonRow(uiText("LOINC name"),
                      ocr: readOnlyValue(printedLOINCName, emptyValue: "").accessibilityIdentifier("ocr-loinc-name"),
                      database: readOnlyValue(names, emptyValue: "").accessibilityIdentifier("database-loinc-name"))
        comparisonRow(uiText("Result"),
                      ocr: readOnlyValue(payload["raw_result"].stringValue).accessibilityIdentifier("ocr-result"),
                      database: Spacer(minLength: 0))
    }

    private func comparisonRow<OCR: View, Database: View>(_ label: String, ocr: OCR, database: Database) -> some View {
        HStack(alignment: .top, spacing: 10) {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(width: 72, alignment: .leading)
            ocr.frame(minWidth: 0, maxWidth: .infinity, alignment: .leading)
            database.frame(minWidth: 0, maxWidth: .infinity, alignment: .leading)
        }
        .padding(.vertical, 3)
    }

    @ViewBuilder private var manualDatabaseRows: some View {
        let metric = databaseMetric
        let codes = metric["loinc"].arrayValue.map { $0["code"].stringValue }.joined(separator: ", ")
        printedRow(uiText("Database metric name"), value: $newMetricName,
                   readOnly: isNewMetric ? nil : metric["name"].stringValue, identifier: "database-metric-name")
        printedRow(uiText("Database metric unit"), value: $newMetricUnit,
                   readOnly: isNewMetric ? nil : metric["standard_unit"].stringValue, identifier: "database-metric-unit")
        printedRow(uiText("Database LOINC code"), value: $newMetricLOINC,
                   readOnly: isNewMetric ? nil : codes, identifier: "database-metric-loinc")
    }

    private func readOnlyValue(_ value: String, emptyValue: String = "—") -> some View {
        Text(value.isEmpty ? emptyValue : value)
            .foregroundStyle(.secondary)
    }

    private func editorField(_ label: String, value: Binding<String>, readOnly: String?, identifier: String?) -> some View {
        Group {
            if let readOnly {
                readOnlyValue(readOnly)
            } else {
                TextField("—", text: value, axis: .vertical)
                    .autocorrectionDisabled()
                    .accessibilityLabel(Text(label))
            }
        }
        .accessibilityIdentifier(identifier ?? label)
    }

    @ViewBuilder private func printedRow(_ label: String, value: Binding<String>, readOnly: String? = nil,
                                        identifier: String? = nil) -> some View {
        let field = editorField(label, value: value, readOnly: readOnly, identifier: identifier)
            .multilineTextAlignment(.trailing)
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

    private var printedLOINCName: String {
        loincCode.isEmpty ? "" : observation["lab_test_name"].stringValue
    }

    private var newDatabaseLOINCCode: String? {
        let code = newMetricLOINC.trimmingCharacters(in: .whitespacesAndNewlines)
        return isReview && isNewMetric && !code.isEmpty && validLOINC(code) ? code : nil
    }

    private var newDatabaseLOINCName: String {
        guard let code = newDatabaseLOINCCode else { return "" }
        if unavailableLOINCCode == code { return uiText("Official name unavailable") }
        return knownLOINCName(code) ?? ""
    }

    private func knownLOINCName(_ code: String) -> String? {
        if code == loincCode && !printedLOINCName.isEmpty { return printedLOINCName }
        if let name = lookedUpLOINCNames[code] { return name }
        let entry = metrics.lazy.flatMap { $0["loinc"].arrayValue }
            .first { $0["code"].stringValue == code && !$0["official_name"].stringValue.isEmpty }
        return entry?["official_name"].stringValue
    }

    private func loadNewLOINCName() async {
        unavailableLOINCCode = nil
        guard let code = newDatabaseLOINCCode, knownLOINCName(code) == nil else { return }
        do {
            try await Task.sleep(for: .milliseconds(300))
            let name = try await lookupLOINCName(code)
            guard !Task.isCancelled, newDatabaseLOINCCode == code else { return }
            lookedUpLOINCNames[code] = name
        } catch {
            if !Task.isCancelled && newDatabaseLOINCCode == code { unavailableLOINCCode = code }
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
