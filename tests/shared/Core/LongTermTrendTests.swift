import Testing
@testable import LongLiveHealthCore

@Test func longTermCardKeysUseServerIdentityAcrossSourcesForOneMetric() {
    let cards = ["watch-a", "watch-b"].map {
        LongTermTrend(.object(["metric_id": .string("vo2_max"), "id": .string("vo2-" + $0)]))
    }
    #expect(Set(cards.map(\.id)).count == 2)
    #expect(cards.map(\.id) == ["vo2-watch-a", "vo2-watch-b"])
}
