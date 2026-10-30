import SwiftUI

/// Contextual explanations stay available without taking space from the user's data.
struct InfoHelp: View {
    let title: String
    let message: String
    var body: some View {
        HStack(spacing: 4) {
            Text(title)
            InfoHelpButton(title: title, message: message)
        }
    }
}

struct InfoHelpButton: View {
    let title: String
    let message: String
    @State var presented = false

    var body: some View {
        Button { presented = true } label: {
            Image(systemName: MobileSymbol.name("info.circle"))
                .font(.body)
                .frame(minWidth: 44, minHeight: 44)
                #if os(iOS)
                .contentShape(Rectangle())
                #endif
        }
        .buttonStyle(.borderless)
        .accessibilityLabel(Text("About \(title)"))
        #if !os(Android)
        .accessibilityHint(Text("Shows an explanation"))
        #endif
        #if os(Android)
        .sheet(isPresented: $presented) { helpContents }
        #else
        .popover(isPresented: $presented) {
            helpContents.presentationCompactAdaptation(.popover)
        }
        #endif
    }

    var helpContents: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    Text(title).font(.headline)
                    Spacer()
                    Button("Done") { presented = false }
                }
                Text(message).font(.body).foregroundStyle(.primary)
                    .fixedSize(horizontal: false, vertical: true)
            }.padding()
        }
        .frame(idealWidth: 320, idealHeight: 240)
    }
}
