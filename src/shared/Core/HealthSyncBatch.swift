import Foundation

/// One durable mixed-type request and the local cursors it may commit after acknowledgement.
public nonisolated struct HealthSyncBlock: Codable, Sendable {
    public let request: JSONValue
    public let writes: [String: JSONValue]

    public func commit(to archive: LocalArchive) async throws {
        // The pending block remains until every write succeeds; replay is idempotent.
        for name in writes.keys.sorted() {
            try await archive.saveAsync(writes[name]!, name: name)
        }
    }
}

public nonisolated struct HealthSyncBatch: Sendable {
    private let connectionID: String
    private var records: [JSONValue] = []
    private var coverage: [String: String] = [:]
    private var writes: [String: JSONValue] = [:]
    private var encodedBytes = 0
    public static let targetBytes = 8 * 1024 * 1024

    public init(connectionID: String) { self.connectionID = connectionID }

    /// A page's cursor is attached only to the block containing its final record.
    /// Earlier blocks can safely replay from the previously committed cursor.
    @concurrent public mutating func append(recordType: String, status: String, records incoming: [JSONValue], writes updates: [String: JSONValue]) async throws -> [HealthSyncBlock] {
        var ready: [HealthSyncBlock] = []
        if incoming.isEmpty {
            coverage[recordType] = status
            writes.merge(updates) { _, new in new }
        }
        for (index, record) in incoming.enumerated() {
            let bytes = try JSONEncoder().encode(record).count + 1
            if !records.isEmpty, encodedBytes + bytes > Self.targetBytes,
               let block = finish() { ready.append(block) }
            records.append(record)
            encodedBytes += bytes
            coverage[recordType] = status
            if index == incoming.count - 1 { writes.merge(updates) { _, new in new } }
            // A target controls buffering, never acceptance: even one oversized record is sent intact.
            if encodedBytes >= Self.targetBytes,
               let block = finish() { ready.append(block) }
        }
        return ready
    }

    public mutating func finish() -> HealthSyncBlock? {
        guard !coverage.isEmpty else { return nil }
        let entries = coverage.keys.sorted().map { kind in
            JSONValue.object(["record_type": .string(kind), "status": .string(coverage[kind]!)])
        }
        let block = HealthSyncBlock(request: .object([
            "connection_id": .string(connectionID), "batch_id": .string(UUID().uuidString),
            "coverage": .array(entries), "records": .array(records)
        ]), writes: writes)
        records = []; coverage = [:]; writes = [:]; encodedBytes = 0
        return block
    }
}
