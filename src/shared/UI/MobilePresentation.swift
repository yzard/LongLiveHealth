import SwiftUI

extension View {
    /// One title policy for shared screens; native iOS title behavior is preserved.
    @ViewBuilder func mobileNavigationTitle(_ title: String) -> some View {
        #if os(Android)
        self.navigationTitle(title).navigationBarTitleDisplayMode(.inline)
        #else
        self.navigationTitle(title)
        #endif
    }

    @ViewBuilder func metricListStyle() -> some View {
        #if os(Android)
        self.listStyle(.automatic)
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
        self
        #else
        self.monospacedDigit()
        #endif
    }
    @ViewBuilder func metricsSearch(_ text: Binding<String>) -> some View {
        #if os(Android)
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass").foregroundStyle(.secondary)
                TextField("Search metrics", text: text).textFieldStyle(.plain)
                    .autocorrectionDisabled().accessibilityLabel(Text("Search metrics"))
                if !text.wrappedValue.isEmpty {
                    Button { text.wrappedValue = "" } label: {
                        Text("×").font(.system(size: 22, weight: .medium)).foregroundStyle(.secondary)
                            .frame(width: 28, height: 28)
                            .background(Color.secondary.opacity(0.18), in: Circle())
                            .frame(width: 44, height: 44)
                    }.buttonStyle(.plain).accessibilityLabel(Text("Clear search"))
                }
            }.padding(12).background(Color.secondary.opacity(0.12), in: RoundedRectangle(cornerRadius: 12))
                .padding(.horizontal, 16).padding(.bottom, 8)
            self
        }
        #else
        self.searchable(text: text, placement: .navigationBarDrawer(displayMode: .always), prompt: "Search metrics")
            .autocorrectionDisabled().textInputAutocapitalization(.never)
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
        case "arrow.clockwise": "arrow.clockwise.circle"
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

/// Keep the placeholder and dynamic choices in one collection. The native Android
/// picker bridge cannot safely index a standalone placeholder followed by ForEach.
struct MenuChoice: Identifiable {
    let id: String
    let title: String
}

struct SelectionPicker: View {
    let title: String
    @Binding var selection: String
    let placeholder: String
    let choices: [MenuChoice]
    var body: some View {
        Picker(title, selection: $selection) {
            ForEach([MenuChoice(id: "", title: placeholder)] + choices.filter { !$0.id.isEmpty }) { choice in
                Text(choice.title).tag(choice.id)
            }
        }
    }
}
