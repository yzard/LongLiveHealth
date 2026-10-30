import SwiftUI

/// The explanation stays visible; it is the content of this section, not contextual help.
struct MetricAboutView: View {
    let id: String
    let name: String

    var body: some View {
        Text(MetricEducation.explanation(id: id, name: name)
             ?? "A description for this specific test is not available yet. Its full name and original report identify what was measured.")
            .accessibilityIdentifier("metric-explanation")
    }
}
