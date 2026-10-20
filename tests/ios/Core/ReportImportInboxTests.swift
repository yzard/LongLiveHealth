import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func sharedPDFSurvivesRelaunchUntilExplicitImportOrDiscard() throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: directory) }
    let inbox = try ReportImportInbox(directory: directory)
    let bytes = Data("%PDF-1.4\nsynthetic report".utf8)
    let report = try inbox.receive(bytes, filename: "Health report.pdf")
    let restored = try ReportImportInbox(directory: directory)
    #expect(try restored.reports().map(\.id) == [report.id])
    #expect(try restored.bytes(for: report) == bytes)
    try restored.remove(report)
    #expect(try restored.reports().isEmpty)
}

@Test func sharedPDFRejectsInvalidAndOversizedFilesWithoutQueueingThem() throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: directory) }
    let inbox = try ReportImportInbox(directory: directory)
    #expect(throws: APIError.self) { try inbox.receive(Data("not a pdf".utf8), filename: "fake.pdf") }
    #expect(throws: APIError.self) { try inbox.receive(Data(repeating: 0, count: 20 * 1024 * 1024 + 1), filename: "large.pdf") }
    #expect(try inbox.reports().isEmpty)
}
