import SwiftUI

struct ReportImportControl: View {
    let model: AppModel
    @State var choosing = false
    var body: some View {
        Button { choosing = true } label: { Image(systemName: MobileSymbol.name("square.and.arrow.up")) }
            .disabled(model.isBusy)
            .confirmationDialog("Import Health Report", isPresented: $choosing) {
                Button("Import PDF") { importDocument("pdf") }
                Button("Import Images") { importDocument("images") }
                Button("Camera") { importDocument("camera") }
            }
    }
    private func importDocument(_ kind: String) {
        Task {
            do {
                let result = try await AndroidSystem.perform("document.import", input: .object(["kind": .string(kind)]))
                for file in result["files"].arrayValue {
                    let url = URL(fileURLWithPath: file["path"].stringValue)
                    let bytes = try await Task.detached { try Data(contentsOf: url) }.value
                    let retained = await model.addDocument(bytes, filename: file["name"].stringValue, contentType: file["type"].stringValue,
                        sourceName: nil, originalBytes: nil, originalFilename: nil, originalContentType: nil)
                    if retained { try FileManager.default.removeItem(at: url) }
                }
            } catch { model.errorMessage = error.localizedDescription }
        }
    }
}

struct DocumentLocation: Identifiable {
    let id = UUID()
    let url: URL
    let page: Int
}

struct DocumentPreview: View {
    let location: DocumentLocation
    @Environment(\.dismiss) var dismiss
    @State var failure: String?
    var body: some View {
        NavigationStack {
            VStack(spacing: 16) {
                if let failure { Text(failure) }
                Button("Open original report") { Task { await open() } }
            }.navigationTitle("Original · page \(location.page)")
                .toolbar { Button("Done") { dismiss() } }
                .task { await open() }
        }
    }
    private func open() async {
        do { _ = try await AndroidSystem.perform("document.open", input: .object(["path": .string(location.url.path), "page": .number(Double(location.page))])) }
        catch { failure = error.localizedDescription }
    }
}
