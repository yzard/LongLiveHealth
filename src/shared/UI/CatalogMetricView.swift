import SwiftUI

/// Alternate entry points resolve the same server definition as Browse.
struct CatalogMetricView: View {
    let model: AppModel
    let kind: String
    @State var definition: JSONValue = .null
    @State var loading = true
    @State var failure: String?
    var body: some View {
        Group {
            if definition != .null { MetricMeasurementsView(model: model, kind: kind, definition: definition) }
            else if loading { ProgressView("Loading metric") }
            else { Text(failure ?? "No recorded measurements.").foregroundStyle(.secondary) }
        }.initialTask {
            defer { loading = false }
            do {
                let catalog = try await model.request("browse/catalog", body: .object([:]))
                definition = catalog["entries"].arrayValue.first { $0["kind"].stringValue == kind }?["definition"] ?? .null
            } catch { if !Task.isCancelled { failure = error.localizedDescription } }
        }
    }
}
