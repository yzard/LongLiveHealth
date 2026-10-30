import SwiftUI

extension View {
    @ViewBuilder func metricListStyle() -> some View {
        #if os(Android)
        self.listStyle(.plain)
        #else
        self.listStyle(.insetGrouped)
        #endif
    }
    @ViewBuilder func metricChartInsets() -> some View {
        #if os(Android)
        self.padding(.vertical, 12)
        #else
        self.listRowInsets(EdgeInsets(top: 12, leading: 16, bottom: 12, trailing: 16))
        #endif
    }
    @ViewBuilder func stableDigits() -> some View {
        #if os(Android)
        self.fontDesign(.monospaced)
        #else
        self.monospacedDigit()
        #endif
    }
    @ViewBuilder func metricsSearch(_ text: Binding<String>) -> some View {
        #if os(Android)
        self.searchable(text: text, prompt: "Search metrics")
        #else
        self.searchable(text: text, placement: .navigationBarDrawer(displayMode: .always), prompt: "Search metrics")
        #endif
    }
}

struct EmptyContent: View {
    let title: String
    let symbol: String
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: MobileSymbol.name(symbol)).font(.largeTitle).foregroundStyle(.secondary)
            Text(title).font(.headline).multilineTextAlignment(.center)
        }.frame(maxWidth: .infinity).padding()
    }
}

struct CopyableText: View {
    let value: String
    init(_ value: String) { self.value = value }
    var body: some View {
        #if os(Android)
        Text(value).contextMenu { Button("Copy") { MobilePlatform.copy(value) } }
        #else
        Text(value).textSelection(.enabled)
        #endif
    }
}

/// Keeps iOS SF Symbols unchanged; Android uses available native equivalents.
enum MobileSymbol {
    static func name(_ name: String) -> String {
        #if os(Android)
        switch name {
        case "heart.text.clipboard": "heart"
        case "chart.xyaxis.line": "chart.bar.xaxis"
        case "square.grid.2x2": "house"
        case "externaldrive": "person.crop.square"
        case "sparkles": "star.fill"
        case "doc.badge.plus": "plus.circle.fill"
        case "doc.text.magnifyingglass": "magnifyingglass"
        case "icloud.and.arrow.up": "square.and.arrow.up"
        case "wifi.slash": "exclamationmark.triangle"
        case "circle": "checkmark.circle"
        default: name
        }
        #else
        name
        #endif
    }
}
