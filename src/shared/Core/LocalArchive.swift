import Foundation

public nonisolated struct LocalArchive: Sendable {
    public let directory: URL
    public init(directory: URL) throws {
        self.directory = directory
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        #if os(iOS)
        try FileManager.default.setAttributes([.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication], ofItemAtPath: directory.path)
        var folder = directory
        var values = URLResourceValues(); values.isExcludedFromBackup = true
        try folder.setResourceValues(values)
        #endif
    }

    // Health checkpoints can contain large series. Keep their coding and disk I/O off the UI actor.
    @concurrent public func saveAsync<T: Encodable & Sendable>(_ content: T, name: String) async throws {
        try Task.checkCancellation()
        try save(content, name: name)
    }
    @concurrent public func loadAsync<T: Decodable & Sendable>(_ type: T.Type, name: String) async throws -> T? {
        try Task.checkCancellation()
        return try load(type, name: name)
    }

    public func save<T: Encodable>(_ content: T, name: String) throws {
        try saveBytes(JSONEncoder().encode(content), name: name)
    }
    public func load<T: Decodable>(_ type: T.Type, name: String) throws -> T? {
        let location = try location(name)
        guard FileManager.default.fileExists(atPath: location.path) else { return nil }
        return try JSONDecoder().decode(type, from: Data(contentsOf: location))
    }
    public func saveBytes(_ content: Data, name: String) throws {
        let location = try location(name)
        try content.write(to: location, options: .atomic)
        #if os(iOS)
        try FileManager.default.setAttributes([.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication], ofItemAtPath: location.path)
        #endif
    }
    public func bytes(name: String) throws -> Data { try Data(contentsOf: location(name)) }
    public func remove(name: String) throws {
        let location = try location(name)
        if FileManager.default.fileExists(atPath: location.path) { try FileManager.default.removeItem(at: location) }
    }
    public func clear() throws {
        if FileManager.default.fileExists(atPath: directory.path) { try FileManager.default.removeItem(at: directory) }
    }
    public func location(_ name: String) throws -> URL {
        guard !name.isEmpty, !name.contains("/"), !name.contains("\\"), name != ".", name != ".." else { throw APIError.invalidResponse }
        return directory.appendingPathComponent(name)
    }
}
