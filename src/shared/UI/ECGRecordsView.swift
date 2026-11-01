import SwiftUI

/// ECG uses its recorded voltage waveform, rather than a calendar aggregate.
struct ECGRecordsView: View {
    let model: AppModel
    @State var records: [JSONValue] = []
    @State var selected = ""
    @State var cursor: JSONValue = .null
    @State var loading = false
    @State var failure: String?
    private func identity(_ record: JSONValue) -> String {
        "\(record["platform"].stringValue):\(record["source_id"].stringValue):\(record["record_id"].stringValue)"
    }
    var body: some View {
        VStack(spacing: 0) {
            if loading { ProgressView("Loading recordings") }
            if let failure { Text(failure).foregroundStyle(.secondary) }
            if let record = records.first(where: { identity($0) == selected }) {
                SpecialtyRecordView(model: model, reference: record, title: "Electrocardiogram").id(selected)
            } else if !loading && failure == nil { Text("No ECG recordings.") }
            if !records.isEmpty {
                Picker("ECG recording", selection: $selected) {
                    ForEach(Array(records.enumerated()), id: \.offset) { _, record in
                        Text("\(Date(timeIntervalSince1970: record["at"].numberValue ?? 0).formatted())").tag(identity(record))
                    }
                }.padding(.horizontal)
            }
            if cursor != .null { Button("More recordings") { Task { await load() } }.disabled(loading) }
        }.mobileNavigationTitle("Electrocardiogram").initialTask { await load() }
    }
    private func load() async {
        guard !loading, let token = model.session?.token else { return }
        loading = true
        defer { loading = false }
        do {
            let response = try await model.request("wellness/records/list", body: .object(["kind":.string("ecg"),"after":cursor]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            records += response["records"].arrayValue
            records.sort { ($0["at"].numberValue ?? 0) > ($1["at"].numberValue ?? 0) }
            cursor = response["next_cursor"]; failure = nil
            if selected.isEmpty, let first = records.first { selected = identity(first) }
        } catch { if !Task.isCancelled { failure = error.localizedDescription } }
    }
}
