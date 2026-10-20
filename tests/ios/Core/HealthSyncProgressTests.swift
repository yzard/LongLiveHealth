import Testing
@testable import LongLiveHealthCore

@Test func healthStatusUsesReadableNamesAndServerTotals() {
    #expect(HealthSyncProgress.name("HKQuantityTypeIdentifierActiveEnergyBurned") == "Active Energy")
    #expect(HealthSyncProgress.name("HKQuantityTypeIdentifierRestingHeartRate") == "Resting Heart Rate")
    #expect(HealthSyncProgress.name("HKCategoryTypeIdentifierAppleStandHour") == "Apple Stand Hour")
    #expect(HealthSyncProgress.name("heart_rate") == "Heart Rate")
    #expect(HealthSyncProgress.name("HKQuantityTypeIdentifierUVExposure") == "UV Exposure")
    #expect(HealthSyncProgress.status(recordType: "active_energy", total: 12) == "Syncing 12 - Active Energy")
    #expect(HealthSyncProgress.status(recordType: "active_energy", total: nil) == "Syncing - Active Energy")
    #expect(HealthSyncProgress.status(recordType: "active_energy", total: 0) == "Syncing 0 - Active Energy")
}

@Test func healthTotalsAreAuthoritativeNotIncrementedOnReplay() throws {
    let reply: JSONValue = .object(["total_records": .number(29330), "replayed": .bool(true)])
    #expect(try HealthSyncProgress.totalRecords(in: reply) == 29330)
    #expect(throws: APIError.self) { try HealthSyncProgress.totalRecords(in: .object([:])) }
    #expect(throws: APIError.self) { try HealthSyncProgress.totalRecords(in: .object(["total_records": .number(-1)])) }
}
