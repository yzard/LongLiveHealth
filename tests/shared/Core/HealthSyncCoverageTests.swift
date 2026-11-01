import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func coverageDoesNotResurrectOldInstallationOrOtherPlatformErrors() {
    let current = UUID()
    let old = UUID()
    func entry(_ platform: String, _ installation: UUID, _ status: String) -> JSONValue {
        .object(["platform": .string(platform), "installation_id": .string(installation.uuidString.lowercased()),
                 "record_type": .string("workout_route"), "status": .string(status)])
    }
    let success = entry("apple_health", current, "no_visible_samples")
    let failure = entry("apple_health", current, "error")
    let stale = entry("apple_health", old, "error")
    let other = entry("health_connect", current, "error")
    let unknown: JSONValue = .object(["record_type": .string("workout_route"), "status": .string("error")])
    #expect(HealthSyncCoverage.currentDevice([stale, other, unknown, success], platform: "apple_health", installationID: current) == [success])
    #expect(HealthSyncCoverage.currentDevice([stale, failure], platform: "apple_health", installationID: current) == [failure])
    #expect(HealthSyncCoverage.currentDevice([success, other], platform: "health_connect", installationID: current) == [other])
    #expect(HealthSyncCoverage.currentDevice([stale, success], platform: "apple_health", installationID: nil).isEmpty)
}
