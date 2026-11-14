import XCTest

/// Explicit opt-in: observes the signed-in account without editing health data or preferences.
nonisolated final class LiveInsightsTests: XCTestCase {
    @MainActor func testInstalledInsightsNavigationAndEvidence() throws {
        guard ProcessInfo.processInfo.environment["LLH_LIVE_DEVICE_AUDIT"] == "1" else {
            throw XCTSkip("Requires an explicitly authorized, signed-in physical device")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.tabBars.buttons["Insights"].waitForExistence(timeout: 20), "Unlock the device and sign in before this read-only check")
        XCTAssertTrue(app.tabBars.buttons["Insights"].isSelected)
        XCTAssertEqual(app.tabBars.buttons.allElementsBoundByIndex.map(\.label), ["Insights", "Browse", "Data"])
        app.tabBars.buttons["Insights"].tap()
        XCTAssertTrue(app.buttons["insight-card-sleep"].waitForExistence(timeout: 60))
        capture(app, "Dashboard top")
        for id in ["sleep", "readiness", "vitals", "heart", "fitness", "cardiovascular"] {
            let card = app.buttons["insight-card-\(id)"]
            reveal(app, card)
            capture(app, "Card \(id)")
            card.tap()
            XCTAssertTrue(app.buttons["Done"].waitForExistence(timeout: 10))
            let titles = ["sleep": "Sleep", "readiness": "Readiness", "vitals": "Vitals", "heart": "Heart", "fitness": "Fitness", "cardiovascular": "Cardiovascular Review"]
            XCTAssertTrue(app.navigationBars[titles[id]!].waitForExistence(timeout: 5), "The selected card must supply its detail on the first presentation")
            capture(app, "Detail \(id) top")
            app.swipeUp()
            capture(app, "Detail \(id) lower")
            let method = app.buttons["Assessment method and sources"].firstMatch
            reveal(app, method)
            method.tap()
            XCTAssertTrue(app.navigationBars["Method & Sources"].waitForExistence(timeout: 5))
            capture(app, "Method \(id)")
            app.navigationBars["Method & Sources"].buttons["Done"].tap()
            app.buttons["Done"].firstMatch.tap()
            XCTAssertTrue(card.waitForExistence(timeout: 5))
        }
        capture(app, "Dashboard bottom")
        app.tabBars.buttons["Browse"].tap()
        app.tabBars.buttons["Insights"].tap()
        XCTAssertTrue(app.buttons["insight-card-cardiovascular"].exists)
        capture(app, "Dashboard restored after tab return")
    }

    @MainActor private func reveal(_ app: XCUIApplication, _ element: XCUIElement) {
        for _ in 0..<16 {
            if element.exists {
                let frame = element.frame
                let lower = app.tabBars.firstMatch.exists ? app.tabBars.firstMatch.frame.minY : app.frame.maxY
                if element.isHittable && frame.midY > 150 && frame.midY < lower - 30 { return }
                if frame.midY < 150 { app.swipeDown() } else { app.swipeUp() }
            } else { app.swipeUp() }
        }
        XCTAssertTrue(element.exists && element.isHittable)
    }

    @MainActor private func capture(_ app: XCUIApplication, _ name: String) {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
