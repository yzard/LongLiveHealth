import Foundation
import Testing
@testable import LongLiveHealthCore

private func sample(_ kind: String, _ index: Int) -> JSONValue {
    .object(["record_type": .string(kind), "record_id": .string("\(kind)-\(index)")])
}

@Test func smallRecordsDoNotFlushAtTenThousand() async throws {
    var builder = HealthSyncBatch(connectionID: "connection")
    #expect(try await builder.append(recordType: "weight", status: "observed", records: [sample("weight", 0)], writes: ["weight": .string("a")]).isEmpty)
    #expect(try await builder.append(recordType: "sleep", status: "observed", records: (0..<3).map { sample("sleep", $0) }, writes: ["sleep": .string("b")]).isEmpty)
    #expect(try await builder.append(recordType: "empty", status: "no_visible_samples", records: [], writes: ["empty": .string("c")]).isEmpty)
    let blocks = try await builder.append(recordType: "heart", status: "observed", records: (0..<9997).map { sample("heart", $0) }, writes: ["heart": .string("d")])
    #expect(blocks.isEmpty)
    let finished = builder.finish()
    let block = try #require(finished)
    #expect(block.request["records"].arrayValue.count == 10_001)
    #expect(block.request["coverage"].arrayValue.count == 4)
    #expect(block.writes.count == 4)
    let empty = builder.finish()
    #expect(empty == nil)
}

@Test func splitPageCommitsCursorOnlyWithItsFinalRecordAndKeepsTail() async throws {
    var builder = HealthSyncBatch(connectionID: "connection")
    let large = JSONValue.object(["payload": .string(String(repeating: "x", count: HealthSyncBatch.targetBytes / 2))])
    let blocks = try await builder.append(recordType: "heart", status: "observed", records: [large, large], writes: ["heart": .string("b")])
    let first = try #require(blocks.first)
    #expect(first.request["records"].arrayValue.count == 1)
    #expect(first.writes["heart"] == nil)
    let finished = builder.finish()
    let last = try #require(finished)
    #expect(last.request["records"].arrayValue.count == 1)
    #expect(last.writes["heart"] == .string("b"))
    #expect(first.request["batch_id"] != last.request["batch_id"])
}

@Test func mixedCheckpointSurvivesRestartWithoutAdvancingBeforeAcknowledgement() async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: directory) }
    let archive = try LocalArchive(directory: directory)
    var builder = HealthSyncBatch(connectionID: "connection")
    let anchor = Data([1,2,3])
    _ = try await builder.append(recordType: "weight", status: "observed", records: [sample("weight", 1)], writes: ["anchor-weight.json": .string(anchor.base64EncodedString())])
    _ = try await builder.append(recordType: "snapshot", status: "observed", records: [sample("snapshot", 1)], writes: ["snapshot.json": .array([sample("snapshot", 1)])])
    let tail = builder.finish()
    let block = try #require(tail)
    try await archive.saveAsync(block, name: "pending.json")
    #expect(try archive.load(Data.self, name: "anchor-weight.json") == nil)
    let reopened = try LocalArchive(directory: directory)
    let restored = try #require(try reopened.load(HealthSyncBlock.self, name: "pending.json"))
    #expect(restored.request == block.request)
    try await restored.commit(to: reopened)
    try await restored.commit(to: reopened)
    #expect(try reopened.load(Data.self, name: "anchor-weight.json") == anchor)
    #expect(try reopened.load([JSONValue].self, name: "snapshot.json")?.count == 1)
}

@Test func oversizedRecordFlushesIntactWithoutLosingCursor() async throws {
    var builder = HealthSyncBatch(connectionID: "connection")
    let record = JSONValue.object(["payload": .string(String(repeating: "x", count: HealthSyncBatch.targetBytes + 1))])
    let blocks = try await builder.append(recordType: "workout_route", status: "observed", records: [record], writes: ["anchor": .string("next")])
    #expect(blocks.count == 1)
    #expect(blocks[0].request["records"].arrayValue == [record])
    #expect(blocks[0].writes["anchor"] == .string("next"))
    #expect(builder.finish() == nil)
}

@Test func memoryFlushDefersPageCursorUntilFinalRecord() async throws {
    var builder = HealthSyncBatch(connectionID: "connection")
    let large = JSONValue.object(["payload": .string(String(repeating: "x", count: HealthSyncBatch.targetBytes))])
    let small = JSONValue.object(["record_id": .string("tail")])
    let blocks = try await builder.append(recordType: "workout_route", status: "observed", records: [large, small], writes: ["anchor": .string("next")])
    #expect(blocks.count == 1)
    #expect(blocks[0].writes.isEmpty)
    let tail = builder.finish()
    #expect(tail?.request["records"].arrayValue == [small])
    #expect(tail?.writes["anchor"] == .string("next"))
}

@Test func chunkingCountsEncodedUnicodeBytesRatherThanCharacters() async throws {
    var builder = HealthSyncBatch(connectionID: "connection")
    let record = JSONValue.object(["text": .string(String(repeating: "测", count: 2 * 1024 * 1024))])
    let blocks = try await builder.append(recordType: "mixed", status: "observed", records: [record, record], writes: ["anchor": .string("end")])
    #expect(blocks.count == 1)
    #expect(blocks[0].request["records"].arrayValue.count == 1)
    #expect(blocks[0].writes.isEmpty)
    let tail = builder.finish()
    #expect(tail?.request["records"].arrayValue.count == 1)
    #expect(tail?.writes["anchor"] == .string("end"))
}
