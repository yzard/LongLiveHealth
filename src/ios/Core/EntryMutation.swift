import Foundation

public nonisolated struct EntryMutation: Equatable, Sendable {
    public let id: UUID
    public let recordID: String
    public let record: JSONValue?
    public init(recordID: String, record: JSONValue?) {
        id = UUID(); self.recordID = recordID; self.record = record
    }
    public func applying(to entries: [JSONValue], includes: (JSONValue) -> Bool) -> [JSONValue] {
        var updated = entries.filter { $0["record_id"].stringValue != recordID }
        if let record, includes(record) { updated.append(record) }
        return updated.sorted { ($0["at"].numberValue ?? 0) > ($1["at"].numberValue ?? 0) }
    }
}
