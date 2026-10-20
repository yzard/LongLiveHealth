import Testing
@testable import LongLiveHealthCore

@Test func aliasesChangePresentationWithoutChangingSourceIdentity() {
    let keys = ["apple_health:watch", "apple_health:phone"]
    let aliases: JSONValue = .object(Dictionary(uniqueKeysWithValues: keys.map { ($0, .string("My devices")) }))
    for key in keys {
        #expect(SourceNames.label(key: key, fallback: "Apple Health", aliases: aliases, catalog: []) == "My devices")
    }
    let records: [JSONValue] = keys.map { .object(["source_key": .string($0), "source_name": .string("Apple Health")]) }
    #expect(Set(records.map(SourceNames.key)).count == 2)
    #expect(SourceNames.label(key: keys[0], fallback: "Original", aliases: .object([:]), catalog: []) == "Original")
    #expect(SourceNames.label(key: keys[0], fallback: keys[0], aliases: .null, catalog: [.object(["source_key": .string(keys[0]), "name": .string("Watch")])]) == "Watch")
}

@Test func sourceKeysMatchReportCatalogAndPreviouslyCachedClinicalData() {
    #expect(SourceNames.reportKey(name: " Function Health ", reportID: "id") == "report-source:function health")
    #expect(SourceNames.reportKey(name: "", reportID: "id") == "report:id")
    #expect(SourceNames.key(.object(["source_id": .string("hospital"), "channel": .string("Apple Health")])) == "apple_health:hospital")
    #expect(SourceNames.key(.object(["source_id": .string("apple_health"), "original": .object(["source_id": .string("sensor")])])) == "apple_health:sensor")
}

@Test func manualSourceUsesCurrentBrandWithoutChangingStoredIdentity() {
    #expect(SourceNames.label(key: "manual:helpyourself", fallback: "helpyourself", aliases: .null, catalog: []) == "Long Live Health")
    #expect(SourceNames.label(key: "manual:helpyourself", fallback: "", aliases: .object(["manual:helpyourself": .string("My log")]), catalog: []) == "My log")
}
