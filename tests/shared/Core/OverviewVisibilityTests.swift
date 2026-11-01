import Testing
@testable import LongLiveHealthCore

@Test func overviewControlsMatchRowsIncludingDefaultAllAndExplicitNone() {
    let kinds = ["sleep", "active_energy", "basal_energy", "steps"]
    var preferences = JSONValue.object(["favorite_metrics": .array([])])
    #expect(kinds.allSatisfy { OverviewVisibility.isVisible($0, preferences: preferences) })
    for kind in kinds {
        for (key, value) in OverviewVisibility.change(kind, visible: false, available: kinds, preferences: preferences) { preferences[key] = value }
        #expect(!OverviewVisibility.isVisible(kind, preferences: preferences))
    }
    #expect(kinds.allSatisfy { !OverviewVisibility.isVisible($0, preferences: preferences) })
    for (key, value) in OverviewVisibility.change("active_energy", visible: true, available: kinds, preferences: preferences) { preferences[key] = value }
    #expect(OverviewVisibility.isVisible("active_energy", preferences: preferences))
    #expect(!OverviewVisibility.isVisible("sleep", preferences: preferences))
    #expect(!OverviewVisibility.isVisible("steps", preferences: .object(["favorite_metrics": .array([.string("sleep")])])))
}
