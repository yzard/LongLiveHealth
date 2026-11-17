import SwiftUI

struct ReportImportControl: View {
    let model: AppModel
    @State var choosing = false
    var body: some View {
        Button { choosing = true } label: {
            Path { path in
                path.move(to: CGPoint(x: 12, y: 16)); path.addLine(to: CGPoint(x: 12, y: 3))
                path.move(to: CGPoint(x: 7, y: 8)); path.addLine(to: CGPoint(x: 12, y: 3)); path.addLine(to: CGPoint(x: 17, y: 8))
                path.move(to: CGPoint(x: 3, y: 14)); path.addLine(to: CGPoint(x: 3, y: 21)); path.addLine(to: CGPoint(x: 21, y: 21)); path.addLine(to: CGPoint(x: 21, y: 14))
            }.stroke(Color.accentColor, style: StrokeStyle(lineWidth: 1.8, lineCap: .round, lineJoin: .round))
                .frame(width: 24, height: 24).frame(width: 44, height: 44)
        }.accessibilityLabel(Text(uiText("Import Blood Work")))
            .disabled(model.isBusy)
            .confirmationDialog(uiText("Import Blood Work"), isPresented: $choosing) {
                Button(uiText("Import PDF")) { importDocument("pdf") }
                Button(uiText("Import Images")) { importDocument("images") }
                Button(uiText("Camera")) { importDocument("camera") }
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

extension View {
    func originalReportPreview(_ location: Binding<DocumentLocation?>) -> some View {
        modifier(OriginalReportPreview(location: location))
    }
}

struct OriginalReportPreview: ViewModifier {
    @Binding var location: DocumentLocation?
    @State var failure: String?

    func body(content: Content) -> some View {
        content
            .task(id: location?.id) {
                guard let requested = location else { return }
                defer {
                    if location?.id == requested.id { location = nil }
                }
                do {
                    _ = try await AndroidSystem.perform("document.open", input: .object([
                        "path": .string(requested.url.path), "page": .number(Double(requested.page))
                    ]))
                } catch is CancellationError {
                } catch {
                    failure = error.localizedDescription
                }
            }
            .alert(uiText("Could not open original report"), isPresented: Binding(
                get: { failure != nil }, set: { if !$0 { failure = nil } }
            )) {
                Button(uiText("OK")) { failure = nil }
            } message: {
                Text(failure ?? "")
            }
    }
}
