import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func savedRowsPatchOnlyMatchingRecordsAndFilters() {
    let old: JSONValue = .object(["record_id": .string("one"), "at": .number(1), "entry": .object(["kind": .string("food")])])
    let other: JSONValue = .object(["record_id": .string("two"), "at": .number(2)])
    var saved = old; saved["version"] = .number(2)
    let change = EntryMutation(recordID: "one", record: saved)
    #expect(change.applying(to: [old, other], includes: { _ in true }) == [other, saved])
    #expect(change.applying(to: [old, other], includes: { $0["entry"]["kind"] == .string("recipe") }) == [other])
    #expect(EntryMutation(recordID: "one", record: nil).applying(to: [old, other], includes: { _ in true }) == [other])
}

@Test func legacyStoredSessionRemainsReadable() throws {
    let session = try JSONDecoder().decode(StoredSession.self, from: Data(#"{"server":"http://192.168.1.20","userID":"one","username":"alice","token":"token"}"#.utf8))
    #expect(session.mustChangePassword == nil)
    var restricted = session; restricted.mustChangePassword = true
    #expect(try JSONDecoder().decode(StoredSession.self, from: JSONEncoder().encode(restricted)).mustChangePassword == true)
}
