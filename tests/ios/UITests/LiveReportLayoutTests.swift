import XCTest

nonisolated final class LiveReportLayoutTests: XCTestCase {
    @MainActor func testHistoricalKetonesUnitMismatch() throws {
        guard ProcessInfo.processInfo.environment["LLH_LIVE_DEVICE_AUDIT"] == "1" else {
            throw XCTSkip("Requires a signed-in physical device")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()
        let data = app.tabBars.buttons["Data"]
        XCTAssertTrue(data.waitForExistence(timeout: 25))
        data.tap()
        let bloodWork = app.buttons.containing(.staticText, identifier: "Blood Work").firstMatch
        XCTAssertTrue(bloodWork.waitForExistence(timeout: 15))
        bloodWork.tap()
        let report = app.buttons.matching(NSPredicate(
            format: "identifier BEGINSWITH %@ AND label CONTAINS %@", "report-row-", "2018-10-20"
        )).firstMatch
        for _ in 0..<8 where !report.isHittable { app.swipeUp() }
        XCTAssertTrue(report.isHittable)
        report.tap()
        let heading = app.staticTexts.matching(NSPredicate(format: "label ==[c] %@", "Needs Review")).firstMatch
        let issue = app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "Unit mismatch · Ketones - urine")).firstMatch
        for _ in 0..<8 where !issue.isHittable { app.swipeUp() }
        XCTAssertTrue(issue.isHittable)
        XCTAssertTrue(heading.exists)
        XCTAssertLessThan(heading.frame.minY, issue.frame.minY)
        XCTAssertTrue(issue.label.contains("report unit mg/dL"))
        XCTAssertTrue(issue.label.contains("database standard unit unitless (qualitative result)"))
        let ketonesTitle = app.buttons.containing(.staticText, identifier: "Ketones").firstMatch
        let manualReview = app.buttons.matching(identifier: "report-result-status-action")
            .allElementsBoundByIndex.first { button in
                button.label == "Review" && button.frame.minY > ketonesTitle.frame.minY
                    && button.frame.maxY < issue.frame.minY
            }
        XCTAssertNotNil(manualReview)
        issue.tap()
        XCTAssertTrue(app.staticTexts["Ketones - urine"].waitForExistence(timeout: 10))
        app.buttons["Cancel"].tap()
        let egfr = app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "LOINC 33914-3")).firstMatch
        for _ in 0..<8 where !egfr.isHittable { app.swipeUp() }
        XCTAssertTrue(egfr.isHittable)
        let newBadge = app.buttons.matching(identifier: "report-result-new").allElementsBoundByIndex
            .first { abs($0.frame.midY - egfr.frame.midY) < 20 }
        XCTAssertNotNil(newBadge)

    }

    @MainActor func testInstalledReportLayout() throws {
        guard ProcessInfo.processInfo.environment["LLH_LIVE_DEVICE_AUDIT"] == "1" else {
            throw XCTSkip("Requires a signed-in physical device")
        }
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()
        let data = app.tabBars.buttons["Data"]
        XCTAssertTrue(data.waitForExistence(timeout: 20))
        data.tap()
        let bloodWork = app.buttons.containing(.staticText, identifier: "Blood Work").firstMatch
        XCTAssertTrue(bloodWork.waitForExistence(timeout: 10))
        bloodWork.tap()
        let reports = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "report-row-"))
        XCTAssertTrue(reports.firstMatch.waitForExistence(timeout: 20))
        let identifiers = reports.allElementsBoundByIndex.map(\.identifier)
        var foundReady = false
        for id in identifiers {
            let report = app.buttons[id]
            guard report.isHittable else { continue }
            report.tap()
            let ready = app.buttons["ready-to-submit-results"]
            XCTAssertTrue(ready.waitForExistence(timeout: 15))
            let submit = app.buttons["submit-all-results"]
            XCTAssertEqual(submit.label, "Submit")
            XCTAssertEqual(ready.frame.midY, submit.frame.midY, accuracy: 2)
            let review = app.staticTexts["Needs Review"]
            if review.exists { XCTAssertGreaterThan(review.frame.minY, ready.frame.maxY) }
            if ready.isEnabled {
                ready.tap()
                XCTAssertTrue(app.navigationBars["Ready to Submit"].waitForExistence(timeout: 10))
                foundReady = true
                app.navigationBars["Ready to Submit"].buttons.firstMatch.tap()
            }
            app.navigationBars.firstMatch.buttons.firstMatch.tap()
        }
        if !foundReady { throw XCTSkip("No report with eligible pending results on this device") }
    }
}
