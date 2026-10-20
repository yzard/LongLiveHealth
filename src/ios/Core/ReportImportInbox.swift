import Foundation

public nonisolated struct SharedReport: Codable, Identifiable, Sendable {
    public let id: UUID
    public let filename: String
    public let createdAt: Date
}

/// Files received from other apps remain unassigned until the user confirms an account.
public nonisolated struct ReportImportInbox: Sendable {
    private let archive: LocalArchive
    public init(directory: URL) throws { archive = try LocalArchive(directory: directory) }

    public func reports() throws -> [SharedReport] {
        try FileManager.default.contentsOfDirectory(at: archive.directory, includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "json" }
            .map { try JSONDecoder().decode(SharedReport.self, from: Data(contentsOf: $0)) }
            .sorted { $0.createdAt < $1.createdAt }
    }

    public func receive(_ bytes: Data, filename: String) throws -> SharedReport {
        guard bytes.count <= 20 * 1024 * 1024 else { throw APIError.status(413, "The document exceeds 20 MiB") }
        guard bytes.starts(with: Data("%PDF-".utf8)) else { throw APIError.status(400, "Choose a PDF document") }
        let report = SharedReport(id: UUID(), filename: filename, createdAt: Date())
        try archive.saveBytes(bytes, name: "\(report.id).pdf")
        try archive.save(report, name: "\(report.id).json")
        return report
    }

    public func bytes(for report: SharedReport) throws -> Data { try archive.bytes(name: "\(report.id).pdf") }
    public func remove(_ report: SharedReport) throws {
        try archive.remove(name: "\(report.id).json")
        try archive.remove(name: "\(report.id).pdf")
    }
}
