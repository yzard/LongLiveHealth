import Foundation
#if canImport(CryptoKit)
import CryptoKit
#else
import Crypto
#endif

/// Account/server isolation is provided by the owning LocalArchive directory.
/// Every view uses this same request-keyed store; it contains no seed data.
public nonisolated struct ResponseCache: Sendable {
    private let archive: LocalArchive
    public init(archive: LocalArchive) throws {
        self.archive = try LocalArchive(directory: archive.directory.appendingPathComponent("responses"))
        // Remove disposable snapshots outside the single response store. Upload drafts,
        // originals, credentials and committed Health anchors are not cache entries.
        for file in try FileManager.default.contentsOfDirectory(at: archive.directory, includingPropertiesForKeys: nil) {
            let name = file.lastPathComponent
            if ["reports.json", "metrics.json", "wellness-preferences.json", "source-catalog.json", "refresh.json", "daily-overview.json"].contains(name)
                || (name.hasPrefix("read-") || name.hasPrefix("report-")) && name.hasSuffix(".json") {
                try FileManager.default.removeItem(at: file)
            }
        }
    }
    public static func key(_ path: String, body: JSONValue) throws -> String {
        let encoder = JSONEncoder(); encoder.outputFormatting = [.sortedKeys]
        var bytes = Data(path.utf8); bytes.append(0); bytes.append(try encoder.encode(body))
        return SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined() + ".json"
    }
    public func load(_ path: String, body: JSONValue) throws -> JSONValue? {
        try archive.load(JSONValue.self, name: Self.key(path, body: body))
    }
    @concurrent public func save(_ result: JSONValue, path: String, body: JSONValue) async throws {
        try archive.save(.object(["saved_at": .number(Date().timeIntervalSince1970), "result": result]) as JSONValue, name: Self.key(path, body: body))
    }
    public func remove(_ path: String, body: JSONValue) throws { try archive.remove(name: Self.key(path, body: body)) }
    public func clear() throws {
        for file in try FileManager.default.contentsOfDirectory(at: archive.directory, includingPropertiesForKeys: nil) where file.pathExtension == "json" {
            try FileManager.default.removeItem(at: file)
        }
    }
}
