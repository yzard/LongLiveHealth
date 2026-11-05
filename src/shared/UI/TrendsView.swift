import SwiftUI

struct TrendsView: View {
    var model: AppModel
    @State var response: JSONValue = .null
    @State var loading = false
    @State var failure: String?

    private var cards: [LongTermTrend] { response["cards"].arrayValue.map { LongTermTrend($0) } }

    var body: some View {
        List {
            if loading { ProgressView("Loading trends") }
            if let failure { Text(failure).foregroundStyle(.secondary) }
            if !loading && failure == nil && cards.isEmpty {
                Text("No clear directional changes with enough comparable data.").foregroundStyle(.secondary)
            }
            ForEach(["worsening", "improving"], id: \.self) { direction in
                let rows = cards.filter { $0.payload["change"]["direction"].stringValue == direction }
                if !rows.isEmpty {
                    Section {
                        ForEach(rows) { card in
                            LongTermTrendTile(model: model, card: card.payload)
                        }
                    } header: {
                        Text(direction == "worsening" ? "Worsening" : "Improving")
                    }
                }
            }
        }
        .mobileNavigationTitle("Trends")
        .initialTask { await reload() }
        .refreshable { await reload() }
    }

    private func reload() async {
        guard !loading, let token = model.session?.token else { return }
        loading = true
        defer { loading = false }
        do {
            let next = try await model.request("wellness/trends", body: .object(["timezone": .string(TimeZone.current.identifier)]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            response = next
            failure = nil
        } catch {
            guard !Task.isCancelled, model.session?.token == token else { return }
            failure = error.localizedDescription
        }
    }
}
