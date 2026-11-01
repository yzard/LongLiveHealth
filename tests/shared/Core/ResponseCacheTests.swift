import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func unifiedCacheIsEmptyUntilFetchedAndIsolatesParametersAndAccounts() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: root) }
    let account = try LocalArchive(directory: root.appendingPathComponent("account-one"))
    try account.save(.object(["old": .bool(true)]) as JSONValue, name: "daily-overview.json")
    try account.save("keep", name: "anchor-sleep.json")
    let cache = try ResponseCache(archive: account)
    #expect(try account.load(JSONValue.self, name: "daily-overview.json") == nil)
    #expect(try account.load(String.self, name: "anchor-sleep.json") == "keep")
    let empty = JSONValue.object([:])
    #expect(try cache.load("browse/catalog", body: empty) == nil)
    for path in ["browse/catalog", "wellness/day", "trends/get", "analysis/list", "reports/list"] {
        try await cache.save(.object(["server_value": .string(path)]), path: path, body: empty)
        #expect(try cache.load(path, body: empty)?["result"]["server_value"] == .string(path))
        #expect(try cache.load(path, body: .object(["date": .string("2026-10-30")])) == nil)
    }
    let reopened = try ResponseCache(archive: account)
    #expect(try reopened.load("browse/catalog", body: empty) != nil)
    let other = try ResponseCache(archive: LocalArchive(directory: root.appendingPathComponent("account-two")))
    #expect(try other.load("browse/catalog", body: empty) == nil)
    try reopened.clear()
    #expect(try cache.load("browse/catalog", body: empty) == nil)
}
