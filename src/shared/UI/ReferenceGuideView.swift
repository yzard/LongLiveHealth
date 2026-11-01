import SwiftUI

/// Both mobile clients render the server's assessment without duplicating medical rules.
struct ReferenceGuideView: View {
    let guidance: JSONValue
    let assessment: JSONValue
    let sourceName: String
    private var applicableRules: [JSONValue] {
        let rule = assessment["applicable_rule"]
        return rule == .null ? [] : [rule]
    }
    private func interval(_ rule: JSONValue) -> String {
        let unit = assessment["unit"].stringValue.isEmpty ? guidance["unit"].stringValue : assessment["unit"].stringValue
        return ReferenceIntervalText.interval(rule,unit:unit,sleepDuration:guidance["metric_id"] == .string("sleep") && ["s", "seconds"].contains(unit))
    }
    private func population(_ rule: JSONValue) -> String {
        if rule["kind"] == .string("report_reference") { return "Original result reference interval" }
        let minimum = Int(rule["min_age"].numberValue ?? 0)
        let age = rule["max_age_exclusive"].numberValue.map { "Ages \(minimum)–\(Int($0) - 1)" } ?? "Ages \(minimum)+"
        let sex = rule["sex"].stringValue
        return sex.isEmpty ? age : "\(age) · \(sex.capitalized)"
    }
    var body: some View {
        if guidance != .null {
            VStack(alignment: .leading, spacing: 6) {
                if assessment != .null {
                    Text("Latest measurement in this interval · \(assessment["label"].stringValue)")
                        .font(.subheadline).foregroundStyle(assessment["status"] == .string("in_range") ? Color.green : Color.primary)
                    if !assessment["reason"].stringValue.isEmpty { Text(assessment["reason"].stringValue).font(.caption).foregroundStyle(.secondary) }
                    if assessment["kind"] == .string("report_reference") { Text("Source: \(sourceName.isEmpty ? "Original report reference interval" : sourceName)").font(.caption).foregroundStyle(.secondary) }
                } else if !guidance["reason"].stringValue.isEmpty {
                    Text(guidance["reason"].stringValue).font(.caption).foregroundStyle(.secondary)
                }
                if !applicableRules.isEmpty {
                    Group {
                        VStack(alignment: .leading, spacing: 10) {
                            ForEach(applicableRules, id: \.identifier) { rule in
                                VStack(alignment: .leading, spacing: 3) {
                                    Text(population(rule)).font(.caption.bold())
                                    if rule["bands"].arrayValue.isEmpty { Text(interval(rule)).font(.caption) }
                                    ForEach(Array(rule["bands"].arrayValue.enumerated()), id: \.offset) { _, band in
                                        Text("\(interval(band)) · \(band["label"].stringValue)").font(.caption)
                                    }
                                    if case .object(let context) = rule["context"], !context.isEmpty {
                                        Text(context.keys.sorted().map { "\($0.replacingOccurrences(of: "_", with: " ")): \(context[$0]?.stringValue ?? "")" }.joined(separator: " · ")).font(.caption).foregroundStyle(.secondary)
                                    }
                                }
                            }
                            if assessment["kind"] != .string("report_reference") { Text(guidance["note"].stringValue).font(.caption).foregroundStyle(.secondary) }
                        }.padding(.vertical, 6)
                    }.font(.caption)
                }
                ForEach(assessment["kind"] == .string("report_reference") ? [] : guidance["citations"].arrayValue, id: \.identifier) { source in
                    if let url = URL(string: source["url"].stringValue) {
                        Link(source["title"].stringValue, destination: url).font(.caption)
                    }
                }
            }.accessibilityIdentifier("reference-guide")
        }
    }
}

/// One immutable destination value prevents row taps from activating adjacent links.
struct ReferenceGuideSelection: Hashable {
    let id = UUID()
    let guidance: JSONValue
    let assessment: JSONValue
    static func == (left: Self, right: Self) -> Bool { left.id == right.id }
    func hash(into hasher: inout Hasher) { hasher.combine(id) }
}

struct ReferenceGuideButton: View {
    let show: () -> Void
    var body: some View {
        Button(action: show) {
            Image(systemName: MobileSymbol.name("info.circle"))
                .font(.body).frame(minWidth: 44, minHeight: 44)
        }
        .buttonStyle(.borderless)
        .accessibilityLabel(Text("About Reference Levels"))
    }
}

struct ReferenceGuidePage: View {
    let model: AppModel
    let guidance: JSONValue
    let assessment: JSONValue
    @State var preview: DocumentLocation?
    @State var failure: String?
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                ReferenceGuideView(guidance: guidance, assessment: assessment,
                    sourceName: model.sourceLabel(assessment["reference_origin"]["source_key"].stringValue,
                                                  name: assessment["reference_origin"]["name"].stringValue))
                if let failure { Text(failure).foregroundStyle(.secondary) }
                if !assessment["reference_origin"]["report_id"].stringValue.isEmpty {
                    Button("View original report") { Task { await openOriginal() } }
                }
            }.padding()
        }.mobileNavigationTitle("Reference Levels").navigationBarTitleDisplayMode(.inline)
            .sheet(item: $preview) { DocumentPreview(location: $0) }
    }
    private func openOriginal() async {
        guard let token = model.session?.token else { return }
        do {
            let url = try await model.source(assessment["reference_origin"]["report_id"].stringValue)
            guard !Task.isCancelled, model.session?.token == token else { return }
            preview = DocumentLocation(url: url, page: 1)
        } catch { if !Task.isCancelled { failure = error.localizedDescription } }
    }
}
