/// A card has its own identity across metric, source, unit and assay boundaries.
public nonisolated struct LongTermTrend: Identifiable, Sendable {
    public let payload: JSONValue
    public var id: String { payload["id"].stringValue }

    public init(_ payload: JSONValue) {
        self.payload = payload
    }
}
