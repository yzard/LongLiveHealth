import Foundation

/// Display metadata comes exclusively from the server catalog.
public nonisolated struct BrowseCategory: Identifiable, Hashable, Sendable {
    public let id: String
    public let title: String
    public let symbol: String
    public let tint: String
    public let section: String
    public init(_ value: JSONValue) {
        id = value["id"].stringValue
        title = value["title"].stringValue
        symbol = value["symbol"].stringValue
        tint = value["color"].stringValue
        section = value["section"].stringValue
    }
}
