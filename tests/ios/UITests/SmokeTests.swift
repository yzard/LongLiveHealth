import XCTest

nonisolated final class SmokeTests: XCTestCase {
    @MainActor private func reveal(_ app: XCUIApplication, _ element: XCUIElement) {
        for _ in 0..<5 {
            if element.exists && element.isHittable { return }
            app.swipeUp()
        }
    }
    @MainActor private func tapButton(_ app: XCUIApplication, _ name: String) {
        let frame = app.buttons[name].frame
        app.coordinate(withNormalizedOffset: .zero).withOffset(CGVector(dx: frame.midX, dy: frame.midY)).tap()
    }
    @MainActor private func dismissPasswordPrompts(_ app: XCUIApplication) {
        for _ in 0..<3 {
            guard app.buttons["Not Now"].waitForExistence(timeout: 2) else { return }
            tapButton(app, "Not Now")
            _ = app.buttons["Not Now"].waitForNonExistence(timeout: 2)
        }
    }
    @MainActor private func tapTab(_ app: XCUIApplication, _ name: String) {
        let tab = (name == "Health" || name == "Archive") ? "Data" : name == "Settings" ? "Overview" : name
        let frame = app.tabBars.buttons[tab].frame
        app.coordinate(withNormalizedOffset: .zero).withOffset(CGVector(dx: frame.midX, dy: frame.midY)).tap()
        if name == "Health", !app.navigationBars["Apple Health"].exists {
            for _ in 0..<4 {
                if app.buttons["Apple Health"].exists { break }
                app.navigationBars.buttons.firstMatch.tap()
            }
            app.buttons["Apple Health"].tap()
        }
        if name == "Archive" {
            if app.navigationBars["Apple Health"].exists { app.navigationBars.buttons.firstMatch.tap() }
            if app.navigationBars["Data"].exists { app.buttons["Health Report"].tap() }
        }
        if name == "Settings", !app.navigationBars["Settings"].exists { app.buttons["Settings"].firstMatch.tap() }
    }
    @MainActor func testPDFShareOpensHealthReportImport() async throws {
        let safari = XCUIApplication(bundleIdentifier: "com.apple.mobilesafari")
        safari.open(URL(string: "http://127.0.0.1:18765/shared-report.pdf")!)
        if safari.buttons["Continue"].waitForExistence(timeout: 3) { safari.buttons["Continue"].tap() }
        let share = safari.buttons["Share"]
        if !share.waitForExistence(timeout: 2) {
            safari.buttons["More"].tap()
        }
        XCTAssertTrue(share.waitForExistence(timeout: 15))
        share.tap()
        let sharing = XCUIApplication(bundleIdentifier: "com.apple.SharingUIService")
        let open = sharing.descendants(matching: .any).matching(NSPredicate(format: "label ==[c] %@", "Long Live Health")).firstMatch
        XCTAssertTrue(open.waitForExistence(timeout: 10))
        open.tap()
        let app = XCUIApplication()
        XCTAssertTrue(app.navigationBars["Import Health Report"].waitForExistence(timeout: 15))
        // Safari may suffix a repeated download (shared-report-1.pdf).
        let shared = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH %@ AND label ENDSWITH %@", "shared-report", ".pdf")).firstMatch
        XCTAssertTrue(shared.waitForExistence(timeout: 5))
        XCTAssertTrue(app.textFields["Clinic or laboratory"].firstMatch.exists)
        for _ in 0..<10 {
            guard shared.exists else { break }
            shared.swipeLeft()
            app.buttons["Discard"].tap()
        }
        app.buttons["Later"].tap()
    }

    @MainActor func testAdminMustChangePasswordBeforeOpeningArchive() async throws {
        var reset = URLRequest(url: URL(string: "http://127.0.0.1:18765/api/v1/test/reset")!)
        reset.httpMethod = "POST"
        _ = try await URLSession.shared.data(for: reset)
        let app = XCUIApplication(); app.launch()
        let server = app.textFields["http://192.168.1.20:8080"]
        XCTAssertTrue(server.waitForExistence(timeout: 10))
        server.tap()
        let previous = server.value as? String ?? ""
        if previous != "http://192.168.1.20:8080" { server.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: previous.count)) }
        server.typeText("http://127.0.0.1:18765")
        app.textFields["Username"].tap(); app.textFields["Username"].typeText("admin")
        app.secureTextFields["Password"].tap(); app.secureTextFields["Password"].typeText("admin")
        app.buttons["Sign in"].tap()
        dismissPasswordPrompts(app)
        XCTAssertTrue(app.navigationBars["Change password"].waitForExistence(timeout: 10))
        XCTAssertFalse(app.tabBars.buttons["Data"].exists)
        let current = app.secureTextFields["Current password"]
        current.tap(); current.typeText("admin")
        let replacement = app.secureTextFields["New password (at least 12 characters)"]
        replacement.tap(); replacement.typeText("new-admin-password")
        let confirmation = app.secureTextFields["Confirm new password"]
        confirmation.tap(); confirmation.typeText("new-admin-password")
        app.buttons["Change password and sign out"].tap()
        XCTAssertTrue(app.buttons["Sign in"].waitForExistence(timeout: 10))
        app.terminate()
    }

    @MainActor func testLoginTabsAndHealthAuthorization() async throws {
        addUIInterruptionMonitor(withDescription: "Synthetic password prompt") { alert in
            if alert.buttons["Not Now"].exists { alert.buttons["Not Now"].tap(); return true }
            return false
        }
        let app = XCUIApplication()
        app.launch()
        dismissPasswordPrompts(app)
        if app.buttons["Settings"].exists {
            app.buttons["Settings"].tap()
            app.buttons["Sign out and clear this device"].tap()
        }
        XCTAssertTrue(app.textFields["http://192.168.1.20:8080"].waitForExistence(timeout: 10))
        app.textFields["http://192.168.1.20:8080"].tap()
        let oldServer = app.textFields["http://192.168.1.20:8080"].value as? String ?? ""
        if oldServer != "http://192.168.1.20:8080" {
            app.textFields["http://192.168.1.20:8080"].typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: oldServer.count))
        }
        app.textFields["http://192.168.1.20:8080"].typeText("http://localhost:18765")
        app.textFields["Username"].tap()
        app.textFields["Username"].typeText("simulator")
        app.secureTextFields["Password"].tap()
        app.secureTextFields["Password"].typeText("synthetic-test-password")
        app.buttons["Sign in"].tap()
        XCTAssertTrue(app.tabBars.buttons["Data"].waitForExistence(timeout: 20))
        tapTab(app, "Archive")
        dismissPasswordPrompts(app)
        tapTab(app, "Archive")
        XCTAssertTrue(app.staticTexts["Add your first report"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Import PDF"].exists)
        XCTAssertTrue(app.buttons["Import PDF"].isHittable)
        app.buttons["Import PDF"].tap()
        XCTAssertTrue(app.buttons["Choose PDF from Files"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Choose PDF from Files"].isEnabled)
        app.textFields["Clinic or laboratory"].tap()
        app.textFields["Clinic or laboratory"].typeText("Function Health")
        app.buttons["Choose PDF from Files"].tap()
        XCTAssertTrue(app.buttons["Cancel"].waitForExistence(timeout: 10))
        app.buttons["Cancel"].tap()
        XCTAssertTrue(app.buttons["Import PDF"].waitForExistence(timeout: 5))

        // Relaunch after the system password sheet to verify Keychain restoration.
        app.terminate()
        app.launch()
        XCTAssertTrue(app.tabBars.buttons["Data"].waitForExistence(timeout: 10))
        tapTab(app, "Archive")
        app.buttons["Import"].tap()
        XCTAssertTrue(app.buttons["Import PDF or image"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Import PDF or image"].isEnabled)
        app.textFields["Clinic or laboratory"].tap()
        app.textFields["Clinic or laboratory"].typeText("Function Health")
        app.buttons["Import PDF or image"].tap()
        XCTAssertTrue(app.buttons["Cancel"].waitForExistence(timeout: 10))
        app.buttons["Cancel"].tap()
        XCTAssertTrue(app.buttons["Cancel"].waitForNonExistence(timeout: 5))
        app.buttons["Import"].tap()
        XCTAssertTrue(app.buttons["Choose photo"].waitForExistence(timeout: 5))
        app.buttons["Choose photo"].tap()
        XCTAssertTrue(app.buttons["Cancel"].waitForExistence(timeout: 10))
        app.buttons["Cancel"].tap()
        XCTAssertTrue(app.buttons["Cancel"].waitForNonExistence(timeout: 5))
        let system = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        if system.buttons["Not Now"].exists { system.buttons["Not Now"].tap() }
        tapTab(app, "Overview")
        XCTAssertTrue(app.staticTexts["Yesterday"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["7 hr 30 min"].firstMatch.waitForExistence(timeout: 10))
        XCTAssertFalse(app.buttons["Device trends"].exists)
        XCTAssertFalse(app.buttons["Timeline"].exists)
        XCTAssertFalse(app.buttons["Breathing practice"].exists)
        XCTAssertFalse(app.datePickers.firstMatch.exists)
        XCTAssertFalse(app.buttons["LDL"].exists)
        func aliasRequestCounts() async throws -> [String: Int] {
            var request = URLRequest(url: URL(string: "http://127.0.0.1:18765/api/v1/test/counts")!)
            request.httpMethod = "POST"
            let (data, _) = try await URLSession.shared.data(for: request)
            return try JSONDecoder().decode([String: Int].self, from: data)
        }
        let beforeAliases = try await aliasRequestCounts()
        // Alias changes update retained summaries; editor cancellation leaves the saved alias intact.
        tapTab(app, "Data")
        if app.navigationBars["Health Report"].exists { app.navigationBars.buttons.firstMatch.tap() }
        reveal(app, app.buttons["Source aliases"])
        app.buttons["Source aliases"].tap()
        let aliasRow = app.buttons["sourceAliasRow-apple_health:watch"]
        XCTAssertTrue(aliasRow.waitForExistence(timeout: 10))
        aliasRow.tap()
        let aliasField = app.textFields["sourceAliasName"]
        XCTAssertTrue(aliasField.waitForExistence(timeout: 5))
        aliasField.tap(); aliasField.typeText("My Watch")
        app.buttons["Save"].tap()
        XCTAssertTrue(aliasRow.waitForExistence(timeout: 10))
        aliasRow.tap()
        XCTAssertTrue(aliasField.waitForExistence(timeout: 5))
        aliasField.tap(); aliasField.typeText(" unsaved")
        app.buttons["Cancel"].tap()
        app.buttons["Discard"].tap()
        XCTAssertTrue(aliasRow.waitForExistence(timeout: 5))
        tapTab(app, "Overview")
        XCTAssertTrue(app.staticTexts["My Watch"].firstMatch.waitForExistence(timeout: 5))
        let afterAliases = try await aliasRequestCounts()
        XCTAssertEqual(afterAliases["wellness/day"], beforeAliases["wellness/day"])
        XCTAssertEqual(afterAliases["wellness/preferences/save", default: 0] - beforeAliases["wellness/preferences/save", default: 0], 1)
        tapTab(app, "Browse")
        XCTAssertTrue(app.buttons["LDL"].waitForExistence(timeout: 10))
        app.buttons["LDL"].tap()
        XCTAssertTrue(app.staticTexts["All sources"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["Function Health"].firstMatch.waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["Local Clinic"].firstMatch.exists)
        app.navigationBars.buttons.firstMatch.tap()
        XCTAssertTrue(app.buttons["Calcium"].waitForExistence(timeout: 10))
        app.buttons["Calcium"].tap()
        XCTAssertTrue(app.staticTexts["About this metric"].waitForExistence(timeout: 10))
        let aboutFrame = app.staticTexts["About this metric"].frame
        reveal(app, app.staticTexts["Historical values"])
        XCTAssertTrue(app.staticTexts["Historical values"].exists)
        XCTAssertGreaterThan(app.staticTexts["Historical values"].frame.minY, 0)
        XCTAssertGreaterThan(aboutFrame.minY, 0)
        reveal(app, app.staticTexts["See laboratory note"])
        XCTAssertTrue(app.staticTexts["See laboratory note"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["NewYork-Presbyterian"].firstMatch.exists)
        app.navigationBars.buttons.firstMatch.tap()
        tapTab(app, "Trends")
        XCTAssertTrue(app.switches["Compare a second metric"].waitForExistence(timeout: 5))
        app.switches["Compare a second metric"].coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5)).tap()
        XCTAssertTrue(app.descendants(matching: .any).matching(NSPredicate(format: "label BEGINSWITH %@", "Second metric")).firstMatch.waitForExistence(timeout: 5))
        tapTab(app, "Insights")
        XCTAssertTrue(app.staticTexts["Personal research preview"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Review lipid history and recent health data"].isEnabled)
        tapTab(app, "Health")
        XCTAssertTrue(app.buttons["Sync Now"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH %@", "Last 30 days")).firstMatch.exists)
        XCTAssertFalse(app.buttons["Sync now"].exists)
        XCTAssertFalse(app.buttons["Connect Apple Health"].exists)
        app.buttons["Sync Now"].tap()
        // Exercise the actual simulator Health permission UI, including denial.
        let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        if springboard.buttons["Cancel"].waitForExistence(timeout: 8) { springboard.buttons["Cancel"].tap() }
        if app.buttons["Cancel"].exists { app.buttons["Cancel"].tap() }
        if app.alerts.firstMatch.waitForExistence(timeout: 5) { app.alerts.buttons["OK"].tap() }
        var seed = URLRequest(url: URL(string: "http://127.0.0.1:18765/api/v1/test/seed-report")!)
        seed.httpMethod = "POST"
        _ = try await URLSession.shared.data(for: seed)
        app.terminate()
        app.launch()
        tapTab(app, "Archive")
        XCTAssertTrue(app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "fixture.png")).firstMatch.waitForExistence(timeout: 10))
        app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "fixture.png")).firstMatch.tap()
        XCTAssertTrue(app.buttons["Collection context"].waitForExistence(timeout: 10))
        app.buttons["Collection context"].tap()
        XCTAssertTrue(app.navigationBars["Collection context"].waitForExistence(timeout: 5))
        // Cancel preserves the underlying report and makes no read request.
        var countsRequest = URLRequest(url: URL(string: "http://127.0.0.1:18765/api/v1/test/reset-counts")!)
        countsRequest.httpMethod = "POST"
        _ = try await URLSession.shared.data(for: countsRequest)
        tapButton(app, "Cancel")
        XCTAssertTrue(app.buttons["Collection context"].waitForExistence(timeout: 5))
        tapTab(app, "Trends")
        tapTab(app, "Archive")
        XCTAssertTrue(app.buttons["Collection context"].waitForExistence(timeout: 5))
        countsRequest.url = URL(string: "http://127.0.0.1:18765/api/v1/test/counts")!
        let (countData, _) = try await URLSession.shared.data(for: countsRequest)
        let counts = try JSONSerialization.jsonObject(with: countData) as? [String: Int]
        XCTAssertNil(counts?["reports/get"])
        XCTAssertNil(counts?["reports/list"])
        app.buttons["Collection context"].tap()
        XCTAssertTrue(app.navigationBars["Collection context"].waitForExistence(timeout: 5))
        tapButton(app, "Save")
        XCTAssertTrue(app.navigationBars["Collection context"].waitForNonExistence(timeout: 10))
        app.buttons["View original report"].tap()
        XCTAssertTrue(app.navigationBars["Original · page 1"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.staticTexts["Preview unavailable"].exists)
        tapButton(app, "Done")
        XCTAssertTrue(app.navigationBars["Original · page 1"].waitForNonExistence(timeout: 5))
        app.buttons["Save to Apple Health"].tap()
        XCTAssertTrue(app.buttons["Save reviewed glucose"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Save reviewed glucose"].isEnabled)
        app.switches["I verified the collection date and time"].coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5)).tap()
        app.buttons["Save reviewed glucose"].tap()
        let glucoseAccess = app.switches["UIA.Health.BloodGlucose.SwitchCell.Switch"]
        if glucoseAccess.waitForExistence(timeout: 5) {
            if glucoseAccess.value as? String == "0" { glucoseAccess.tap() }
            XCTAssertTrue(app.buttons["Allow"].isEnabled)
            app.buttons["Allow"].tap()
        }
        XCTAssertTrue(app.navigationBars["Save to Apple Health"].waitForNonExistence(timeout: 15))
        XCTAssertTrue(app.buttons["Collection context"].waitForExistence(timeout: 15))
        tapTab(app, "Health")
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label CONTAINS %@ OR label CONTAINS %@ OR label CONTAINS %@", "saved to Apple Health", "Sync complete", "Some types could not be read")).firstMatch.waitForExistence(timeout: 15))
        tapTab(app, "Settings")
        XCTAssertTrue(app.buttons["Create ZIP export"].waitForExistence(timeout: 10))
        app.buttons["Create ZIP export"].tap()
        XCTAssertTrue(app.buttons["Download and share"].waitForExistence(timeout: 10))
        app.buttons["Download and share"].tap()
        XCTAssertTrue(app.staticTexts["Your complete archive is ready"].waitForExistence(timeout: 10))
        tapButton(app, "Done")
        app.buttons["Sign out and clear this device"].tap()
        XCTAssertTrue(app.buttons["Sign in"].waitForExistence(timeout: 10))
    }
    @MainActor func testReportEditingEvidenceRelationshipsAndAnalysis() async throws {
        func control(_ path: String) async throws {
            var request = URLRequest(url: URL(string: "http://127.0.0.1:18765/api/v1/\(path)")!)
            request.httpMethod = "POST"
            _ = try await URLSession.shared.data(for: request)
        }
        try await control("test/reset")
        try await control("test/seed-report")
        try await control("test/enable-workflows")
        let app = XCUIApplication()
        app.launch()
        if app.tabBars.buttons["Data"].exists {
            tapTab(app, "Settings")
            app.buttons["Sign out and clear this device"].tap()
        }
        let server = app.textFields["http://192.168.1.20:8080"]
        XCTAssertTrue(server.waitForExistence(timeout: 10))
        server.tap()
        let previous = server.value as? String ?? ""
        if previous != "http://192.168.1.20:8080" { server.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: previous.count)) }
        server.typeText("http://127.0.0.1:18765")
        app.textFields["Username"].tap(); app.textFields["Username"].typeText("simulator")
        app.secureTextFields["Password"].tap(); app.secureTextFields["Password"].typeText("synthetic-test-password")
        app.buttons["Sign in"].tap()
        XCTAssertTrue(app.tabBars.buttons["Data"].waitForExistence(timeout: 15))
        tapTab(app, "Archive")
        dismissPasswordPrompts(app)
        app.terminate(); app.launch()
        tapTab(app, "Archive")
        XCTAssertTrue(app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "fixture.png")).firstMatch.waitForExistence(timeout: 10))
        app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "fixture.png")).firstMatch.tap()
        XCTAssertTrue(app.buttons["Add result manually"].waitForExistence(timeout: 5))
        app.buttons["Add result manually"].tap()
        XCTAssertTrue(app.navigationBars["Add result"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Save"].isEnabled)
        let name = app.textFields["Name"].exists ? app.textFields["Name"] : app.textViews["Name"]
        name.tap(); name.typeText("Manual fixture result")
        let result = app.textFields["Result, including < or >"].exists ? app.textFields["Result, including < or >"] : app.textViews["Result, including < or >"]
        result.tap(); result.typeText("100")
        tapButton(app, "Save")
        XCTAssertTrue(app.navigationBars["Add result"].waitForNonExistence(timeout: 10))
        XCTAssertTrue(app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "Manual fixture result")).firstMatch.waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons.matching(identifier: "Revision history").firstMatch.exists)
        let evidence = app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "text layer:")).firstMatch
        reveal(app, evidence)
        evidence.tap()
        XCTAssertTrue(app.staticTexts["Synthetic page text"].waitForExistence(timeout: 5))
        app.navigationBars.buttons.firstMatch.tap()
        reveal(app, app.buttons["Page 1 · ocr"])
        app.buttons["Page 1 · ocr"].tap()
        XCTAssertTrue(app.staticTexts["Synthetic OCR output"].waitForExistence(timeout: 5))
        app.navigationBars.buttons.firstMatch.tap()
        app.swipeDown()
        app.buttons["Duplicate or revised report"].tap()
        XCTAssertTrue(app.navigationBars["Prefer another report"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["other.png"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["This is a duplicate; prefer selected report"].waitForExistence(timeout: 5))
        app.buttons["This is a duplicate; prefer selected report"].tap()
        XCTAssertTrue(app.navigationBars["Prefer another report"].waitForNonExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["Excluded from trends: duplicate. Original data is retained."].waitForExistence(timeout: 5))
        app.buttons["Treat as independent report"].tap()
        XCTAssertTrue(app.staticTexts["Excluded from trends: duplicate. Original data is retained."].waitForNonExistence(timeout: 10))
        tapTab(app, "Trends")
        let datedResult = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "2026-10-01")).firstMatch
        reveal(app, datedResult)
        XCTAssertTrue(datedResult.waitForExistence(timeout: 10))
        datedResult.tap()
        XCTAssertTrue(app.navigationBars["Original · page 1"].waitForExistence(timeout: 5))
        tapButton(app, "Done")
        XCTAssertTrue(app.navigationBars["Original · page 1"].waitForNonExistence(timeout: 5))
        tapTab(app, "Insights")
        XCTAssertTrue(app.buttons["Review lipid history and recent health data"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Review lipid history and recent health data"].isEnabled)
        app.buttons["Review lipid history and recent health data"].tap()
        let ready = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "Ready")).firstMatch
        XCTAssertTrue(ready.waitForExistence(timeout: 10)); ready.tap()
        XCTAssertTrue(app.staticTexts["Synthetic lipid review"].waitForExistence(timeout: 5))
        let feedback = app.textFields["Incorrect, helpful, or discussed with your clinician"].exists ? app.textFields["Incorrect, helpful, or discussed with your clinician"] : app.textViews["Incorrect, helpful, or discussed with your clinician"]
        feedback.tap(); feedback.typeText("Synthetic feedback")
        app.buttons["Save feedback"].tap()
        XCTAssertTrue(app.staticTexts["Synthetic feedback"].waitForExistence(timeout: 10))
        app.navigationBars.buttons.firstMatch.tap()
        tapTab(app, "Settings")
        app.buttons["Create ZIP export"].tap()
        XCTAssertTrue(app.buttons["Delete export"].waitForExistence(timeout: 10))
        app.buttons["Delete export"].tap()
        XCTAssertTrue(app.buttons["Delete export"].waitForNonExistence(timeout: 10))
        app.buttons["Delete account and all server data"].tap()
        XCTAssertTrue(app.alerts["Delete your account?"].waitForExistence(timeout: 5))
        app.alerts.textFields.firstMatch.tap(); app.alerts.textFields.firstMatch.typeText("simulator")
        app.alerts.buttons["Delete"].tap()
        XCTAssertTrue(app.buttons["Sign in"].waitForExistence(timeout: 10))
    }
}
