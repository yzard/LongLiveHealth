import SwiftUI

/// One presentation contract for every Browse metric, regardless of its data source.
struct MetricDetailScreen<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        List { content }
            .metricListStyle()
            .mobileNavigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            #if !os(Android)
            .toolbar {
                ToolbarItem(placement: .principal) {
                    Text(title)
                        .font(.largeTitle.bold())
                        .lineLimit(1)
                        .minimumScaleFactor(0.55)
                        .accessibilityAddTraits(.isHeader)
                        .accessibilityIdentifier("metric-detail-title")
                }
            }
            #endif
    }
}

/// Chart controls and plots share the same background and horizontal insets.
struct MetricChartSection<Content: View>: View {
    @ViewBuilder let content: Content

    var body: some View {
        Section { content }
            .listRowBackground(Color.clear)
            .metricChartInsets()
            .listRowSeparator(.hidden)
    }
}
