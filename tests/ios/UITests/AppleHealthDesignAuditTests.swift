import XCTest

/// Read-only inspection of the owner's Apple Health layout; captures stay in local test results.
nonisolated final class AppleHealthDesignAuditTests: XCTestCase {
    @MainActor func testReadOnlyInsightsReference() throws {
        guard ProcessInfo.processInfo.environment["LLH_APPLE_HEALTH_DESIGN_AUDIT"] == "1" else {
            throw XCTSkip("Requires explicit authorization to inspect Apple Health")
        }
        let app = XCUIApplication(bundleIdentifier: "com.apple.Health")
        app.activate()
        let insights = app.tabBars.buttons["Insights"]
        XCTAssertTrue(insights.waitForExistence(timeout: 10))
        insights.tap()
        showSummary(app)
        for _ in 0..<5 { app.swipeDown() }
        for category in ["Sleep", "Readiness", "Vitals", "Heart", "Fitness"] {
            let tile = app.buttons["UIA.Health.Dashboard.\(category == "Fitness" ? "Activity" : category).Tile"]
            for _ in 0..<12 {
                if tile.exists && tile.frame.midY > 180 && tile.frame.midY < 800 { break }
                app.swipeUp()
            }
            XCTAssertTrue(tile.exists)
            tile.tap()
            capture(app, "\(category) detail top")
            app.swipeUp()
            capture(app, "\(category) detail lower")
            showSummary(app)
            for _ in 0..<5 { app.swipeDown() }
        }
    }
    @MainActor private func showSummary(_ app: XCUIApplication) {
        let summary = app.buttons["UIA.Health.Dashboard.PaneSelector.Today"]
        for _ in 0..<5 {
            if summary.exists && summary.frame.minX >= 0 { break }
            app.coordinate(withNormalizedOffset: CGVector(dx: 0.15, dy: 0.10))
                .press(forDuration: 0.1, thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.10)))
        }
        summary.tap()
    }
    @MainActor private func capture(_ app: XCUIApplication, _ name: String) {
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = "Apple Health \(name)"; shot.lifetime = .keepAlways; add(shot)
        let tree = XCTAttachment(string: app.debugDescription)
        tree.name = "Hierarchy \(name)"; tree.lifetime = .keepAlways; add(tree)
    }
}
