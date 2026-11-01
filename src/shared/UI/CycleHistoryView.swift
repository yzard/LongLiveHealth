import SwiftUI

struct CycleHistoryView: View {
    var model: AppModel
    @State var date = Date()
    @State var observations: [JSONValue] = []
    @State var predictions: [JSONValue] = []
    @State var failure: String?
    @State var editing: Selection?
    struct Selection: Identifiable { let id = UUID(); let existing: JSONValue?; let kind: String }
    var body: some View {
        List {
            Section("Cycle history") {
                DatePicker("Ending date", selection: $date, displayedComponents: .date)
                InfoHelp(title: "Cycle history", message: "Recorded observations from the last 90 calendar days. Pregnancy, postpartum and perimenopause remain explicit contexts.")
                Button("Record observation") { editing = Selection(existing: nil, kind: "cycle") }
                Button("Record an external prediction") { editing = Selection(existing: nil, kind: "cycle_prediction") }
                InfoHelp(title: "Predictions", message: "External predictions remain separate from confirmed observations. This view does not infer ovulation or provide contraception guidance.")
                if let failure { Text(failure).foregroundStyle(.secondary) }
            }
            Section("Observed flow, symptoms and context") {
                if observations.isEmpty { Text("No recorded observations") }
                ForEach(Array(observations.enumerated()), id: \.offset) { _, entry in
                    VStack(alignment: .leading) {
                        Text(Date(timeIntervalSince1970: entry["at"].numberValue ?? 0).formatted())
                        Text("\(entry["entry"]["content"]["flow"].stringValue.capitalized) · \(entry["entry"]["content"]["context"].stringValue.capitalized)")
                        Text(entry["entry"]["content"]["symptoms"].arrayValue.map(\.stringValue).joined(separator: ", ")).font(.caption)
                        Button("Edit observation") { editing = Selection(existing: entry, kind: "cycle") }
                    }
                }
            }
            Section("Predictions from another source") {
                if predictions.isEmpty { Text("No recorded source predictions") }
                ForEach(Array(predictions.enumerated()), id: \.offset) { _, entry in
                    let prediction = entry["entry"]["content"]
                    VStack(alignment: .leading) {
                        Text(prediction["start_date"].stringValue + " to " + prediction["end_date"].stringValue)
                        Text(model.sourceLabel(prediction["source"].stringValue)).font(.headline)
                        if let generated = prediction["generated_at"].numberValue { Text("Generated " + Date(timeIntervalSince1970: generated).formatted()).font(.caption) }
                        Text(prediction["uncertainty"].stringValue).font(.caption)
                        Button("Correct source prediction") { editing = Selection(existing: entry, kind: "cycle_prediction") }
                    }
                }
            }
            NavigationLink("Manage recorded entries", destination: DailyLogsView(model: model))
        }.mobileNavigationTitle("Cycle and life stages").initialTask(id: date) { await load() }
            .sheet(item: $editing) { selection in NavigationStack { LogEditor(model: model, existing: selection.existing, initialKind: selection.kind, initialValues: [:], initialDate: date) } }
    }
    private func load() async {
        guard let token = model.session?.token else { return }
        let day = Calendar.current.startOfDay(for: date)
        guard let start = Calendar.current.date(byAdding: .day, value: -89, to: day), let end = Calendar.current.date(byAdding: .day, value: 1, to: day) else { return }
        do {
            let observed = try await model.request("wellness/entries/list", body: .object(["start_at": .number(start.timeIntervalSince1970), "end_at": .number(end.timeIntervalSince1970), "kind": .string("cycle")]))
            let predicted = try await model.request("wellness/entries/list", body: .object(["start_at": .number(start.timeIntervalSince1970), "end_at": .number(end.timeIntervalSince1970), "kind": .string("cycle_prediction")]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            observations = observed["entries"].arrayValue; predictions = predicted["entries"].arrayValue; failure = nil
        } catch { if !Task.isCancelled, model.session?.token == token { failure = error.localizedDescription } }
    }
}
