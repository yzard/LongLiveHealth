import SwiftUI

struct ObservationEditor: View {
    let observation: JSONValue
    let metrics: [JSONValue]
    let pageCount: Int
    let save: (JSONValue) async throws -> Void
    @State var payload: JSONValue
    @State var status: String
    @State var saving = false
    @State var error: String?
    @Environment(\.dismiss) var dismiss
    init(observation: JSONValue, metrics: [JSONValue], pageCount: Int, save: @escaping (JSONValue) async throws -> Void) {
        self.observation = observation; self.metrics = metrics; self.pageCount = pageCount; self.save = save
        _payload = State(initialValue: observation == .null ? .object(["source": .object(["page": .number(1), "quote": .string("")])]) : observation["payload"])
        _status = State(initialValue: observation["status"].stringValue.isEmpty ? "pending" : observation["status"].stringValue)
    }
    var body: some View {
        NavigationStack {
            Form {
                Section("As printed on the report") {
                    field("Name", "raw_name"); field("Result, including < or >", "raw_result"); field("Unit", "raw_unit")
                    field("Reference range", "reference_range"); field("Report flag", "report_flag")
                    field("Collection date: YYYY-MM-DD or ISO timestamp", "sampled_at")
                }
                Section("Comparison") {
                    Picker("Metric", selection: text("metric_id")) {
                        Text("Automatic from report name").tag("")
                        ForEach(metrics, id: \.identifier) { Text($0["name"].stringValue).tag($0["metric_id"].stringValue) }
                    }
                    if payload["metric_id"] == .null,
                       let match = metrics.first(where: { $0["metric_id"] == observation["resolved_metric_id"] }) {
                        Text("Matched metric: \(match["name"].stringValue)").font(.caption)
                    }
                    InfoHelp(title: "Metric mapping", message: "Check the printed test, value and unit before mapping. The server handles supported unit conversions; unknown units and bounded or text results remain in the archive without an exact numeric trend.")
                }
                Section("Source") {
                    Stepper("Page \(Int(payload["source"]["page"].numberValue ?? 1))", value: Binding(get: { Int(payload["source"]["page"].numberValue ?? 1) }, set: { payload["source"]["page"] = .number(Double($0)) }), in: 1...max(1, pageCount))
                    TextField("Exact source text", text: Binding(get: { payload["source"]["quote"].stringValue }, set: { payload["source"]["quote"] = .string($0) }), axis: .vertical)
                    field("Your notes", "notes")
                }
                Section("Review decision") {
                    Picker("Status", selection: $status) { Text("Pending").tag("pending"); Text("Confirmed").tag("confirmed"); Text("Rejected").tag("rejected") }
                    InfoHelp(title: "Review decision", message: "Confirm after checking the original report. Each save retains the previous revision.")
                }
                if let error { Section { Text(error).foregroundStyle(.red) } }
            }.navigationTitle(observation == .null ? "Add result" : "Review result")
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() }.disabled(saving) }
                    ToolbarItem(placement: .confirmationAction) { Button("Save") { Task {
                        saving = true; defer { saving = false }
                        do {
                            try await save(.object(["observation_id": observation["observation_id"], "expected_revision": observation["revision"], "status": .string(status), "payload": payload]))
                            dismiss()
                        } catch { self.error = error.localizedDescription }
                    } }.disabled(saving || payload["raw_name"].stringValue.isEmpty || payload["raw_result"].stringValue.isEmpty) }
                }.interactiveDismissDisabled()
        }
    }
    private func text(_ key: String) -> Binding<String> { Binding(get: { payload[key].stringValue }, set: { payload[key] = ["raw_name", "raw_result"].contains(key) ? .string($0) : .optional($0) }) }
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
        }.navigationTitle("Report Info").toolbar {
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
