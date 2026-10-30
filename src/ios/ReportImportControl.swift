import SwiftUI
import PhotosUI
import UniformTypeIdentifiers
import VisionKit
import ImageIO

struct ReportImportControl: View {
    var model: AppModel
    @State private var choosingImport = false
    @State private var importAction: String?
    @State private var importing = false
    @State private var scanning = false
    @State private var choosingPhoto = false
    @State private var photo: PhotosPickerItem?
    var body: some View {
        Button("Upload", systemImage: "square.and.arrow.up") { choosingImport = true }.disabled(model.isBusy)
        .sheet(isPresented: $choosingImport, onDismiss: {
            switch importAction {
            case "file": importing = true
            case "photo": choosingPhoto = true
            case "scan": scanning = true
            default: break
            }
            importAction = nil
        }) {
            NavigationStack {
                Form {
                    Section {
                        Button("Import PDF", systemImage: "doc") { importAction = "file"; choosingImport = false }
                        Button("Import Images", systemImage: "photo.on.rectangle") { importAction = "photo"; choosingImport = false }
                        Button("Camera", systemImage: "camera") { importAction = "scan"; choosingImport = false }
                            .disabled(!VNDocumentCameraViewController.isSupported)
                    }
                }.navigationTitle("Import Health Report").toolbar { Button("Cancel") { choosingImport = false } }
            }
        }
        .fileImporter(isPresented: $importing, allowedContentTypes: [.pdf], allowsMultipleSelection: false) { result in
            Task {
                do {
                    guard let url = try result.get().first else { return }
                    let access = url.startAccessingSecurityScopedResource(); defer { if access { url.stopAccessingSecurityScopedResource() } }
                    let bytes = try Data(contentsOf: url)
                    await model.addDocument(bytes, filename: url.lastPathComponent, contentType: "application/pdf", sourceName: nil, originalBytes: nil, originalFilename: nil, originalContentType: nil)
                } catch { model.errorMessage = error.localizedDescription }
            }
        }
        .photosPicker(isPresented: $choosingPhoto, selection: $photo, matching: .images)
        .onChange(of: photo) { _, item in Task {
            do { if let bytes = try await item?.loadTransferable(type: Data.self) { await addPhoto(bytes, name: "Report photo") } }
            catch { model.errorMessage = error.localizedDescription }
            photo = nil
        } }
        .sheet(isPresented: $scanning) { CameraReportCapture(cancel: { scanning = false }) { result in
            scanning = false
            switch result {
            case .success(let pdf): Task {
                await model.addDocument(pdf, filename: "Camera report.pdf", contentType: "application/pdf", sourceName: nil, originalBytes: nil, originalFilename: nil, originalContentType: nil)
            }
            case .failure(let error): model.errorMessage = error.localizedDescription
            }
        } }
    }
    private func addPhoto(_ bytes: Data, name: String) async {
        guard let source = CGImageSourceCreateWithData(bytes as CFData, nil), let identifier = CGImageSourceGetType(source),
              let type = UTType(identifier as String), let contentType = type.preferredMIMEType else { model.errorMessage = "Could not decode this image"; return }
        let filename = name + "." + (type.preferredFilenameExtension ?? "image")
        if type == .jpeg || type == .png {
            await model.addDocument(bytes, filename: filename, contentType: contentType, sourceName: nil, originalBytes: nil, originalFilename: nil, originalContentType: nil)
            return
        }
        guard ["image/heic", "image/heif"].contains(contentType), let image = UIImage(data: bytes), let jpeg = image.jpegData(compressionQuality: 0.92) else {
            model.errorMessage = "Choose a JPEG, PNG, HEIC or HEIF photo"; return
        }
        await model.addDocument(jpeg, filename: name + ".jpg", contentType: "image/jpeg", sourceName: nil, originalBytes: bytes, originalFilename: filename, originalContentType: contentType)
    }
}
