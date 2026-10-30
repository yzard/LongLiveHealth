import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func browseClassifiesByMeaningRatherThanSource() {
    #expect(BrowseCategory.classify(id: "hrv_rmssd", name: "HRV RMSSD", laboratory: false) == .heart)
    #expect(BrowseCategory.classify(id: "HKQuantityTypeIdentifierHeartRateVariabilityRMSSD", name: "Recovery HRV", laboratory: false) == .heart)
    #expect(BrowseCategory.classify(id: "apob", name: "Apolipoprotein B", laboratory: true) == .heart)
    #expect(BrowseCategory.classify(id: "ldl", name: "LDL", laboratory: true) == .heart)
    #expect(BrowseCategory.classify(id: "sodium", name: "Sodium", laboratory: true) == .bloodWork)
    #expect(BrowseCategory.classify(id: "HKQuantityTypeIdentifierDietarySodium", name: "Dietary Sodium", laboratory: false) == .nutrition)
    #expect(BrowseCategory.classify(id: "HKQuantityTypeIdentifierDietaryCholesterol", name: "Dietary Cholesterol", laboratory: false) == .nutrition)
    #expect(BrowseCategory.classify(id: "new_lab", name: "New lab test", laboratory: true) == .bloodWork)
    #expect(BrowseCategory.classify(id: "future_device_type", name: "New reading", laboratory: false) == .other)
}

@Test func browsePartitionsEntireAppleCatalog() throws {
    let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    let data = try Data(contentsOf: root.appendingPathComponent("src/backend_api/apple_catalog.json"))
    let catalog = try JSONDecoder().decode([JSONValue].self, from: data)
    var buckets: [BrowseCategory: Set<String>] = [:]
    for item in catalog {
        let kind = item["kind"].stringValue
        let category = BrowseCategory.classify(id: kind, name: item["title"].stringValue, laboratory: false)
        buckets[category, default: []].insert(kind)
    }
    #expect(buckets.values.reduce(0) { $0 + $1.count } == catalog.count)
    // Only these intentional miscellaneous records should use the fallback.
    #expect(buckets[.other] == Set([
        "HKCategoryTypeIdentifierHandwashingEvent", "HKCategoryTypeIdentifierToothbrushingEvent",
        "HKClinicalTypeIdentifierCoverageRecord", "HKQuantityTypeIdentifierTimeInDaylight",
        "HKQuantityTypeIdentifierUVExposure", "HKQuantityTypeIdentifierUnderwaterDepth",
        "HKQuantityTypeIdentifierWaterTemperature"
    ]))
    #expect(buckets[.sleep]?.contains("sleep") == true)
    #expect(buckets[.activity]?.contains("basal_energy") == true)
    #expect(buckets[.bloodWork]?.contains("HKClinicalTypeIdentifierLabResultRecord") == true)
}
