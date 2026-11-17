import SwiftUI
import PDFKit
import VisionKit

struct DocumentLocation: Identifiable {
    let id = UUID()
    let url: URL
    let page: Int
}

extension View {
    func originalReportPreview(_ location: Binding<DocumentLocation?>) -> some View {
        sheet(item: location) { DocumentPreview(location: $0) }
    }
}

private struct DocumentPreview: View {
    let location: DocumentLocation
    @Environment(\.dismiss) private var dismiss
    var body: some View {
        NavigationStack {
            Group {
                if let document = PDFDocument(url: location.url), document.pageCount > 0 {
                    PDFPreview(document: document, page: location.page)
                }
                else if let image = UIImage(contentsOfFile: location.url.path) {
                    ScrollView([.horizontal, .vertical]) { Image(uiImage: image).resizable().scaledToFit().frame(maxWidth: 900).padding() }
                } else { ContentUnavailableView(uiText("Preview unavailable"), systemImage: "doc.questionmark") }
            }
            .navigationTitle(uiText("Original · page \(location.page)"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button(uiText("Done")) { dismiss() } } }
        }
    }
}

private struct PDFPreview: UIViewRepresentable {
    let document: PDFDocument
    let page: Int

    func makeUIView(context: Context) -> PDFView {
        let view = PDFView()
        view.autoScales = true
        view.displayMode = .singlePageContinuous
        view.displayDirection = .vertical
        view.document = document
        if let target = document.page(at: min(max(0, page - 1), document.pageCount - 1)) {
            view.go(to: target)
        }
        return view
    }

    func updateUIView(_ view: PDFView, context: Context) {}
}

struct DocumentScanner: UIViewControllerRepresentable {
    let cancel: () -> Void
    let completion: (Result<[UIImage], any Error>) -> Void
    func makeCoordinator() -> Coordinator { Coordinator(cancel: cancel, completion: completion) }
    func makeUIViewController(context: Context) -> VNDocumentCameraViewController {
        let controller = VNDocumentCameraViewController(); controller.delegate = context.coordinator; return controller
    }
    func updateUIViewController(_ controller: VNDocumentCameraViewController, context: Context) {}
    @MainActor final class Coordinator: NSObject, VNDocumentCameraViewControllerDelegate {
        let cancel: () -> Void
        let completion: (Result<[UIImage], any Error>) -> Void
        init(cancel: @escaping () -> Void, completion: @escaping (Result<[UIImage], any Error>) -> Void) {
            self.cancel = cancel
            self.completion = completion
        }
        func documentCameraViewController(_ controller: VNDocumentCameraViewController, didFinishWith scan: VNDocumentCameraScan) {
            completion(.success((0..<scan.pageCount).map { scan.imageOfPage(at: $0) }))
        }

        func documentCameraViewControllerDidCancel(_ controller: VNDocumentCameraViewController) { cancel() }
        func documentCameraViewController(_ controller: VNDocumentCameraViewController, didFailWithError error: any Error) {
            completion(.failure(error))
        }
    }
}

/// Keep camera pages in capture order and upload them as one report.
enum CameraReportPDF {
    static func make(pageCount: Int, imageAt: (Int) -> UIImage) throws -> Data {
        guard pageCount > 0 else { throw APIError.invalidResponse }
        let document = PDFDocument()
        for index in 0..<pageCount {
            guard let page = PDFPage(image: imageAt(index)) else { throw APIError.invalidResponse }
            document.insert(page, at: index)
        }
        guard let bytes = document.dataRepresentation() else { throw APIError.invalidResponse }
        return bytes
    }
}

struct CameraReportPages {
    var images: [UIImage]
    var selected = 0

    mutating func deleteSelected() {
        guard images.indices.contains(selected) else { return }
        images.remove(at: selected)
        selected = min(selected, max(0, images.count - 1))
    }

    func pdf() throws -> Data {
        try CameraReportPDF.make(pageCount: images.count) { images[$0] }
    }
}

struct CameraReportCapture: View {
    let cancel: () -> Void
    let completion: (Result<Data, any Error>) -> Void
    @State private var pages: CameraReportPages?

    var body: some View {
        if let pages {
            CameraReportReview(pages: pages, cancel: cancel, completion: completion)
        } else {
            DocumentScanner(cancel: cancel) { result in
                switch result {
                case .success(let images): pages = CameraReportPages(images: images)
                case .failure(let error): completion(.failure(error))
                }
            }
        }
    }
}

struct CameraReportReview: View {
    @State var pages: CameraReportPages
    let cancel: () -> Void
    let completion: (Result<Data, any Error>) -> Void

    var body: some View {
        NavigationStack {
            VStack(spacing: 16) {
                if pages.images.isEmpty {
                    ContentUnavailableView(uiText("No pages"), systemImage: "doc", description: Text(uiText("All photos have been removed.")))
                } else {
                    TabView(selection: $pages.selected) {
                        ForEach(pages.images.indices, id: \.self) { index in
                            Image(uiImage: pages.images[index])
                                .resizable().scaledToFit().padding(.horizontal)
                                .accessibilityLabel(uiText("Report page \(index + 1)"))
                                .tag(index)
                        }
                    }.tabViewStyle(.page(indexDisplayMode: .never))
                    Text(uiText("Page \(pages.selected + 1) of \(pages.images.count)"))
                        .accessibilityIdentifier("camera-page-count")
                    if pages.images.count > 1 {
                        Slider(value: Binding(get: { Double(pages.selected) }, set: { pages.selected = Int($0) }),
                               in: 0...Double(pages.images.count - 1), step: 1)
                            .accessibilityLabel(uiText("Browse report pages"))
                            .accessibilityValue(uiText("Page \(pages.selected + 1) of \(pages.images.count)"))
                            .padding(.horizontal)
                    }
                    ScrollViewReader { proxy in
                        ScrollView(.horizontal) {
                            HStack(spacing: 12) {
                                ForEach(pages.images.indices, id: \.self) { index in
                                    Button { pages.selected = index } label: {
                                        Image(uiImage: pages.images[index]).resizable().scaledToFit()
                                            .frame(width: 52, height: 72).padding(4)
                                            .background(pages.selected == index ? Color.accentColor.opacity(0.2) : Color.clear)
                                            .clipShape(RoundedRectangle(cornerRadius: 6))
                                            .overlay(RoundedRectangle(cornerRadius: 6).stroke(pages.selected == index ? Color.accentColor : Color.clear, lineWidth: 2))
                                    }.accessibilityLabel(uiText("Page \(index + 1)"))
                                        .accessibilityAddTraits(pages.selected == index ? .isSelected : [])
                                        .id(index)
                                }
                            }.padding(.horizontal)
                        }.onChange(of: pages.selected) { _, index in proxy.scrollTo(index, anchor: .center) }
                    }
                    Button(uiText("Delete Photo"), systemImage: "trash", role: .destructive) { pages.deleteSelected() }
                        .padding(.bottom)
                }
            }
            .navigationTitle(uiText("Review Photos"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button(uiText("Cancel"), action: cancel) }
                ToolbarItem(placement: .confirmationAction) {
                    Button(uiText("Import")) {
                        do { completion(.success(try pages.pdf())) }
                        catch { completion(.failure(error)) }
                    }.disabled(pages.images.isEmpty)
                }
            }
        }
    }
}
