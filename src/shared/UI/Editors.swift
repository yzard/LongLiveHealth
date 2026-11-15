import SwiftUI

struct ObservationEditor: View {
    let observation: JSONValue
    let metrics: [JSONValue]
    let pageCount: Int
    let save: (JSONValue, JSONValue) async throws -> Void
    @State var payload: JSONValue
    @State var printedName: String
    @State var loincCode: String
    @State var choosingMetric: Bool
    @State var creatingMetric = false
    @State var usesDefaultMapping = true
    @State var newMetricName: String
    @State var newMetricUnit: String
    @State var status: String
    @State var saving = false
    @State var error: String?
    @Environment(\.dismiss) var dismiss
    init(observation: JSONValue, metrics: [JSONValue], pageCount: Int, choosingMetric: Bool, save: @escaping (JSONValue, JSONValue) async throws -> Void) {
        self.observation = observation; self.metrics = metrics; self.pageCount = pageCount; self.save = save
        _payload = State(initialValue: observation == .null ? .object(["source": .object(["page": .number(1), "quote": .string("")])]) : observation["payload"])
        let code = observation["lab_test_code"].stringValue
        _printedName = State(initialValue: LaboratoryPresentation.printedName(observation["payload"]["raw_name"].stringValue, code: code))
        _loincCode = State(initialValue: code)
        _choosingMetric = State(initialValue: choosingMetric)
        _newMetricName = State(initialValue: observation["payload"]["ocr_new_metric"]["name"].stringValue.isEmpty ? laboratoryResultName(observation) : observation["payload"]["ocr_new_metric"]["name"].stringValue)
        _newMetricUnit = State(initialValue: observation["payload"]["ocr_new_metric"] == .null ? observation["payload"]["raw_unit"].stringValue : observation["payload"]["ocr_new_metric"]["standard_unit"].stringValue)
        _status = State(initialValue: observation["status"].stringValue.isEmpty ? "pending" : observation["status"].stringValue)
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
                            Button("Save") { Task { await submit() } }.disabled(saveDisabled)
                        }
                    }
                }.interactiveDismissDisabled()
        }
    }
    private var saveDisabled: Bool {
        saving || printedName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || payload["raw_result"].stringValue.isEmpty
            || (creatingMetric && (newMetricName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || status == "rejected"))
    }

    private func submit() async {
        saving = true
        defer { saving = false }
        do {
            let metric: JSONValue = creatingMetric
                ? .object(["name": .string(newMetricName), "standard_unit": .string(newMetricUnit)])
                : .null
            let originalCode = observation["lab_test_code"].stringValue
            let originalName = LaboratoryPresentation.printedName(payload["raw_name"].stringValue, code: originalCode)
            var reviewedPayload = payload
            if let suggestedMetricID, usesDefaultMapping, !creatingMetric {
                reviewedPayload["metric_id"] = .string(suggestedMetricID)
            }
            if printedName != originalName || loincCode != originalCode {
                let code = loincCode.trimmingCharacters(in: .whitespacesAndNewlines)
                reviewedPayload["raw_name"] = .string(code.isEmpty ? printedName : "\(printedName) (\(code))")
            }
            let change: JSONValue = .object([
                "observation_id": observation["observation_id"],
                "expected_revision": observation["revision"],
                "status": .string(status),
                "payload": reviewedPayload,
            ])
            try await save(change, metric)
            dismiss()
        } catch { self.error = error.localizedDescription }
    }

    @ViewBuilder private var reviewSections: some View {
        Section("Mapping") {
            recommendedName
            if creatingMetric {
                TextField("Metric name", text: $newMetricName)
                TextField("Standard unit", text: $newMetricUnit)
                Button("Choose an existing metric") {
                    creatingMetric = false
                    choosingMetric = true
                }
            } else {
                SelectionPicker(title: "Metric mapping", selection: mapping, placeholder: "Unmapped",
                    choices: mappingChoices)
                if suggestedMetricID != nil {
                    Button(usesDefaultMapping ? "Suggested match" : "Review suggested matches") {
                        choosingMetric = true
                    }.font(.caption)
                }
            }
            mappingIssues
        }
        Section("As printed on the report") {
            TextField("Name", text: $printedName, axis: .vertical).autocorrectionDisabled()
            if !observation["lab_test_code"].stringValue.isEmpty {
                TextField("LOINC code", text: $loincCode).autocorrectionDisabled()
            }
            field("Result, including < or >", "raw_result"); field("Unit", "raw_unit")
            field("Reference range", "reference_range"); field("Report flag", "report_flag")
            field("Collection date: YYYY-MM-DD or ISO timestamp", "sampled_at")
        }
        Section("Source") {
            Stepper("Page \(Int(payload["source"]["page"].numberValue ?? 1))", value: Binding(get: { Int(payload["source"]["page"].numberValue ?? 1) }, set: { payload["source"]["page"] = .number(Double($0)) }), in: 1...max(1, pageCount))
            TextField("Exact source text", text: Binding(get: { payload["source"]["quote"].stringValue }, set: { payload["source"]["quote"] = .string($0) }), axis: .vertical)
            field("Your notes", "notes")
        }
        Section {
            Picker("Review decision status", selection: $status) {
                Text("Pending").tag("pending")
                Text("Confirmed").tag("confirmed")
                Text("Rejected").tag("rejected")
            }
        }
        if let error { Section { Text(error).foregroundStyle(.red) } }
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
            Button("Create new metric") {
                payload["metric_id"] = .null
                usesDefaultMapping = false
                creatingMetric = true
                choosingMetric = false
            }.accessibilityIdentifier("create-browse-metric")
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
            VStack(alignment: .leading, spacing: 4) {
                Text("Recommended name (LOINC)").font(.caption).foregroundStyle(.secondary)
                Text(name).accessibilityIdentifier("review-result-loinc-name")
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
                return issue["scope"] == .string("browse")
                    || ["negative_with_unit", "unsupported_unit", "missing_unit"].contains(code)
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
        }.sorted { $0.title.localizedStandardCompare($1.title) == .orderedAscending }
    }

    private var mapping: Binding<String> {
        Binding(get: {
            let selected = payload["metric_id"].stringValue
            if !selected.isEmpty || !usesDefaultMapping { return selected }
            return suggestedMetricID ?? observation["resolved_metric_id"].stringValue
        }, set: {
            usesDefaultMapping = false
            payload["metric_id"] = .optional($0)
        })
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
