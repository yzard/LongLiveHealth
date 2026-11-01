import SwiftUI

/// One compact row shared by numeric, report, daily and source-record histories.
struct HistoricalValueRow: View {
    let timestamp: Double?
    let dateText: String
    let source: String
    let value: String
    var body: some View {
        HStack(spacing: 8) {
            Text(HistoricalValue.date(timestamp: timestamp, text: dateText))
#if os(Android)
                .font(.system(.caption, design: .monospaced))
#else
                .monospacedDigit()
#endif
                .frame(width: 145, alignment: .leading)
            Text(source.isEmpty ? "—" : source).foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
            Text(value.isEmpty ? "—" : value)
#if os(Android)
                .font(.system(.caption, design: .monospaced))
#else
                .monospacedDigit()
#endif
                .frame(minWidth: 50, alignment: .trailing)
#if !os(Android)
                .layoutPriority(1)
#endif
        }.font(.caption).lineLimit(1).minimumScaleFactor(0.7)
            .frame(maxWidth: .infinity, minHeight: 40, alignment: .leading)
#if !os(Android)
            .accessibilityElement(children: .combine)
#endif
    }
}
