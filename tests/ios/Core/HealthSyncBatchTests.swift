import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func batchesFitTransportAndRetainEveryRawRecord() throws {
    let records = (0..<3).map { JSONValue.object(["record_id": .string("sample-\($0)"), "payload": .string(String(repeating: "x", count: 200))]) }
    let requests = try HealthSyncBatch.requests(connectionID: "connection", recordType: "raw_type", coverageStatus: "observed", records: records, maximumRecords: HealthSyncBatch.maximumRecords, maximumBytes: 600)
    #expect(requests.count > 1)
    #expect(requests.flatMap { $0["records"].arrayValue } == records)
    #expect(Set(requests.map { $0["batch_id"].stringValue }).count == requests.count)
    for request in requests { #expect(try JSONEncoder().encode(request).count <= 600) }
    #expect(throws: APIError.self) {
        try HealthSyncBatch.requests(connectionID: "connection", recordType: "raw_type", coverageStatus: "observed", records: records, maximumRecords: HealthSyncBatch.maximumRecords, maximumBytes: 100)
    }
}

@Test func batchesRespectCountLimitAndSendEmptyCoverage() throws {
    let records = (0..<2001).map { JSONValue.object(["record_id": .string("sample-\($0)"), "payload": .object([:])]) }
    let requests = try HealthSyncBatch.requests(connectionID: "connection", recordType: "raw_type", coverageStatus: "observed", records: records, maximumRecords: HealthSyncBatch.maximumRecords, maximumBytes: 40 * 1024 * 1024)
    #expect(requests.map { $0["records"].arrayValue.count } == [1000, 1000, 1])
    #expect(requests.flatMap { $0["records"].arrayValue } == records)
    let empty = try HealthSyncBatch.requests(connectionID: "connection", recordType: "raw_type", coverageStatus: "no_visible_samples", records: [], maximumRecords: HealthSyncBatch.maximumRecords, maximumBytes: 40 * 1024 * 1024)
    #expect(empty.count == 1)
    #expect(empty[0]["coverage_status"] == .string("no_visible_samples"))
    #expect(empty[0]["records"].arrayValue.isEmpty)
}

@Test func byteLimitIncludesEscapedUnicodeRecordsAndEnvelope() throws {
    let records = (0..<3).map { JSONValue.object(["record_id": .string("记录/\($0)"), "payload": .string("line\n\"quoted\"/健康")]) }
    let combined = try HealthSyncBatch.requests(connectionID: "设备/one", recordType: "raw_type", coverageStatus: "observed", records: records, maximumRecords: HealthSyncBatch.maximumRecords, maximumBytes: 4096)
    let limit = try JSONEncoder().encode(combined[0]).count - 1
    let split = try HealthSyncBatch.requests(connectionID: "设备/one", recordType: "raw_type", coverageStatus: "observed", records: records, maximumRecords: HealthSyncBatch.maximumRecords, maximumBytes: limit)
    #expect(split.count == 2)
    #expect(split.flatMap { $0["records"].arrayValue } == records)
    for request in split { #expect(try JSONEncoder().encode(request).count <= limit) }
    #expect(throws: APIError.self) {
        try HealthSyncBatch.requests(connectionID: "connection", recordType: "raw_type", coverageStatus: "no_visible_samples", records: [], maximumRecords: HealthSyncBatch.maximumRecords, maximumBytes: 1)
    }
}
