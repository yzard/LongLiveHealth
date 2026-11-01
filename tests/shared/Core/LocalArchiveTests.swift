import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func checkpointSurvivesRelaunchAndArchiveRejectsTraversal() throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let archive = try LocalArchive(directory: directory)
    defer { try? archive.clear() }
    let checkpoint = JSONValue.object(["batch_id": .string("stable")])
    try archive.save(checkpoint, name: "pending.json")
    let reopened = try LocalArchive(directory: directory)
    let loaded = try #require(try reopened.load(JSONValue.self, name: "pending.json"))
    #expect(loaded == checkpoint)
    #expect(throws: (any Error).self) { try archive.location("../private") }
    #expect(throws: (any Error).self) { try archive.location("..") }
    try archive.remove(name: "pending.json")
    #expect(try archive.load(JSONValue.self, name: "pending.json") == nil)
}

private struct ThreadCheckedValue: Codable, Sendable {
    let value: Int
    enum CodingKeys: String, CodingKey { case value }
    init(value: Int) { self.value = value }
    init(from decoder: any Decoder) throws {
        #expect(!Thread.isMainThread)
        value = try decoder.container(keyedBy: CodingKeys.self).decode(Int.self, forKey: .value)
    }
    func encode(to encoder: any Encoder) throws {
        #expect(!Thread.isMainThread)
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(value, forKey: .value)
    }
}

@Test @MainActor func checkpointCodingLeavesTheUIThreadAndPreservesData() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let archive = try LocalArchive(directory: directory)
    defer { try? archive.clear() }
    try await archive.saveAsync(ThreadCheckedValue(value: 44), name: "pending.json")
    let restored = try await archive.loadAsync(ThreadCheckedValue.self, name: "pending.json")
    #expect(restored?.value == 44)
    await #expect(throws: (any Error).self) { try await archive.saveAsync(ThreadCheckedValue(value: 0), name: "../invalid") }
}
