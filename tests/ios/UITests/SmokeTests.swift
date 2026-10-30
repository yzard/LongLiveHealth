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
    @MainActor func testBrowseCategoryNavigation() async throws {
        let app = XCUIApplication(); app.launch()
        dismissPasswordPrompts(app)
        if app.buttons["Sign in"].exists {
            let server = app.textFields["http://192.168.1.20:8080"]
            server.tap()
            let old = server.value as? String ?? ""
            if old != "http://192.168.1.20:8080" { server.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: old.count)) }
            server.typeText("http://127.0.0.1:18765")
            app.textFields["Username"].tap(); app.textFields["Username"].typeText("simulator")
            app.secureTextFields["Password"].tap(); app.secureTextFields["Password"].typeText("test-password")
            app.buttons["Sign in"].tap(); dismissPasswordPrompts(app)
        }
        XCTAssertTrue(app.tabBars.buttons["Browse"].waitForExistence(timeout: 10))
        tapTab(app, "Browse")
        XCTAssertTrue(app.staticTexts["Health Categories"].waitForExistence(timeout: 10))
        XCTAssertFalse(app.buttons["Heart rate"].exists)
        let heart = app.buttons["Heart"]
        reveal(app, heart); heart.tap()
        XCTAssertTrue(app.buttons["Heart rate"].waitForExistence(timeout: 5))
        reveal(app, app.buttons["LDL"])
        XCTAssertTrue(app.buttons["LDL"].exists)
        XCTAssertFalse(app.buttons["Calcium"].exists)
        for _ in 0..<5 {
            if app.buttons["Heart rate"].exists && app.buttons["Heart rate"].isHittable { break }
            app.swipeDown()
        }
        var countRequest = URLRequest(url: URL(string: "http://127.0.0.1:18765/api/v1/test/reset-counts")!)
        countRequest.httpMethod = "POST"
        _ = try await URLSession.shared.data(for: countRequest)
        app.buttons["Heart rate"].tap()
        XCTAssertTrue(app.staticTexts["metric-detail-title"].waitForExistence(timeout: 10))
        app.navigationBars.buttons.firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Heart"].exists)
        tapTab(app, "Overview"); tapTab(app, "Browse")
        XCTAssertTrue(app.navigationBars["Heart"].exists)
        app.navigationBars.buttons.firstMatch.tap()
        let blood = app.buttons["Blood Work"]
        reveal(app, blood); blood.tap()
        XCTAssertTrue(app.buttons["Calcium"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["LDL"].exists)
        app.navigationBars.buttons.firstMatch.tap()
        countRequest.url = URL(string: "http://127.0.0.1:18765/api/v1/test/counts")!
        let (countData, _) = try await URLSession.shared.data(for: countRequest)
        let counts = try JSONSerialization.jsonObject(with: countData) as? [String: Int]
        for route in ["wellness/sources", "metrics/browse", "wellness/day"] {
            XCTAssertEqual(counts?[route] ?? 0, 0, "Returning to Browse must preserve the loaded directory")
        }
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = "Browse health categories"; shot.lifetime = .keepAlways; add(shot)
    }

    @MainActor func testAllAppleCatalogBrowseEntries() async throws {
        let app = XCUIApplication(); app.launch()
        dismissPasswordPrompts(app)
        if app.buttons["Sign in"].exists {
            let server = app.textFields["http://192.168.1.20:8080"]
            server.tap()
            let old = server.value as? String ?? ""
            if old != "http://192.168.1.20:8080" { server.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: old.count)) }
            server.typeText("http://127.0.0.1:18765")
            app.textFields["Username"].tap(); app.textFields["Username"].typeText("simulator")
            app.secureTextFields["Password"].tap(); app.secureTextFields["Password"].typeText("test-password")
            app.buttons["Sign in"].tap(); dismissPasswordPrompts(app)
        }
        XCTAssertTrue(app.tabBars.buttons["Browse"].waitForExistence(timeout: 10))
        tapTab(app, "Browse")
        let search = app.searchFields["Search metrics"]
        for (query, title) in [("body mass index", "Body Mass Index (BMI)"), ("systolic", "Systolic Blood Pressure"), ("physical effort", "Physical Effort")] {
            search.tap(); search.typeText(query)
            XCTAssertTrue(app.buttons[title].waitForExistence(timeout: 10))
            app.buttons[title].tap()
            XCTAssertTrue(app.staticTexts["chart-range-value"].waitForExistence(timeout: 10))
            XCTAssertNotEqual(app.staticTexts["chart-range-value"].label, "No data")
            app.navigationBars.buttons.firstMatch.tap()
            search.buttons["Clear text"].tap()
        }
        search.tap(); search.typeText("stand hour")
        XCTAssertTrue(app.buttons["Apple Stand Hour"].waitForExistence(timeout: 10))
        app.buttons["Apple Stand Hour"].tap()
        XCTAssertTrue(app.staticTexts["Historical Values"].waitForExistence(timeout: 10))
        let source = app.staticTexts["Apple Watch"].firstMatch
        XCTAssertTrue(source.waitForExistence(timeout: 10)); source.tap()
        XCTAssertTrue(app.staticTexts["Recorded details"].waitForExistence(timeout: 10))
    }

    @MainActor func testOverviewTodayWeekMonthTable() async throws {
        let app = XCUIApplication(); app.launch()
        dismissPasswordPrompts(app)
        if app.buttons["Sign in"].exists {
            let server = app.textFields["http://192.168.1.20:8080"]
            server.tap()
            let old = server.value as? String ?? ""
            if old != "http://192.168.1.20:8080" { server.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: old.count)) }
            server.typeText("http://127.0.0.1:18765")
            app.textFields["Username"].tap(); app.textFields["Username"].typeText("simulator")
            app.secureTextFields["Password"].tap(); app.secureTextFields["Password"].typeText("test-password")
            app.buttons["Sign in"].tap(); dismissPasswordPrompts(app)
        }
        XCTAssertTrue(app.tabBars.buttons["Overview"].waitForExistence(timeout: 10))
        tapTab(app, "Overview")
        for label in ["Today", "Last 7 days", "Last 30 days"] {
            XCTAssertTrue(app.staticTexts[label].waitForExistence(timeout: 10))
            XCTAssertTrue(app.staticTexts[label].isHittable)
        }
        XCTAssertFalse(app.staticTexts["Yesterday"].exists)
        XCTAssertLessThan(app.staticTexts["Today"].frame.midX, app.staticTexts["Last 7 days"].frame.midX)
        XCTAssertLessThan(app.staticTexts["Last 7 days"].frame.midX, app.staticTexts["Last 30 days"].frame.midX)
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = "Overview today week month"
        shot.lifetime = .keepAlways
        add(shot)
    }

    @MainActor func testBrowseDetailsUseSharedPresentation() async throws {
        let app = XCUIApplication(); app.launch()
        dismissPasswordPrompts(app)
        if app.buttons["Sign in"].exists {
            let server = app.textFields["http://192.168.1.20:8080"]
            server.tap()
            let old = server.value as? String ?? ""
            if old != "http://192.168.1.20:8080" { server.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: old.count)) }
            server.typeText("http://127.0.0.1:18765")
            app.textFields["Username"].tap(); app.textFields["Username"].typeText("simulator")
            app.secureTextFields["Password"].tap(); app.secureTextFields["Password"].typeText("test-password")
            app.buttons["Sign in"].tap(); dismissPasswordPrompts(app)
        }
        XCTAssertTrue(app.tabBars.buttons["Browse"].waitForExistence(timeout: 10))
        tapTab(app, "Browse")
        let search = app.searchFields["Search metrics"]
        for (query, item, title) in [("sleep", "Sleep", "Sleep"), ("vo2", "VO₂ Max", "VO2 Max"), ("LDL", "LDL", "LDL")] {
            search.tap()
            if search.buttons["Clear text"].exists { search.buttons["Clear text"].tap() }
            search.typeText(query)
            XCTAssertTrue(app.buttons[item].waitForExistence(timeout: 10))
            app.buttons[item].tap()
            let heading = app.staticTexts["metric-detail-title"]
            XCTAssertTrue(heading.waitForExistence(timeout: 10))
            XCTAssertEqual(heading.label, title)
            XCTAssertLessThan(abs(heading.frame.midX - app.frame.midX), 25)
            let shot = XCTAttachment(screenshot: app.screenshot())
            shot.name = "Shared metric style - " + item
            shot.lifetime = .keepAlways
            add(shot)
            app.navigationBars.buttons.firstMatch.tap()
            XCTAssertTrue(search.waitForExistence(timeout: 5))
            XCTAssertEqual(search.value as? String, query)
        }
    }

    @MainActor func testRoundedHeartRangeAppearance() async throws {
        let app = XCUIApplication(); app.launch()
        dismissPasswordPrompts(app)
        if app.buttons["Sign in"].exists {
            let server = app.textFields["http://192.168.1.20:8080"]
            server.tap()
            let old = server.value as? String ?? ""
            if old != "http://192.168.1.20:8080" { server.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: old.count)) }
            server.typeText("http://127.0.0.1:18765")
            app.textFields["Username"].tap(); app.textFields["Username"].typeText("simulator")
            app.secureTextFields["Password"].tap(); app.secureTextFields["Password"].typeText("test-password")
            app.buttons["Sign in"].tap(); dismissPasswordPrompts(app)
        }
        XCTAssertTrue(app.tabBars.buttons["Browse"].waitForExistence(timeout: 10))
        tapTab(app, "Browse")
        let search = app.searchFields["Search metrics"]
        search.tap(); search.typeText("heart rate")
        XCTAssertTrue(app.buttons["Heart rate"].waitForExistence(timeout: 10))
        app.buttons["Heart rate"].tap()
        XCTAssertTrue(app.staticTexts["70–90"].firstMatch.waitForExistence(timeout: 10))
        let screenshot = XCTAttachment(screenshot: app.screenshot())
        screenshot.name = "Rounded heart range"
        screenshot.lifetime = .keepAlways
        add(screenshot)
    }

    @MainActor func testMetricEducationAndReopenRefresh() async throws {
        let app = XCUIApplication(); app.launch()
        dismissPasswordPrompts(app)
        if app.buttons["Sign in"].exists {
            let server = app.textFields["http://192.168.1.20:8080"]
            server.tap()
            let old = server.value as? String ?? ""
            if old != "http://192.168.1.20:8080" { server.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: old.count)) }
            server.typeText("http://127.0.0.1:18765")
            app.textFields["Username"].tap(); app.textFields["Username"].typeText("simulator")
            app.secureTextFields["Password"].tap(); app.secureTextFields["Password"].typeText("test-password")
            app.buttons["Sign in"].tap(); dismissPasswordPrompts(app)
        }
        XCTAssertTrue(app.tabBars.buttons["Browse"].waitForExistence(timeout: 10))
        tapTab(app, "Browse")
        let search = app.searchFields["Search metrics"]
        XCTAssertTrue(search.waitForExistence(timeout: 10))
        search.tap()
        search.typeText("LDL")
        XCTAssertTrue(app.buttons["LDL"].waitForExistence(timeout: 10))
        app.buttons["LDL"].tap()
        XCTAssertTrue(app.staticTexts["Function Health"].firstMatch.waitForExistence(timeout: 10))
        XCTAssertFalse(app.staticTexts["All sources"].exists)
        let chartAppearance = XCTAttachment(screenshot: app.screenshot())
        chartAppearance.name = "Metric chart rounded ends"
        chartAppearance.lifetime = .keepAlways
        add(chartAppearance)
        reveal(app, app.staticTexts["metric-explanation"])
        XCTAssertTrue(app.staticTexts["metric-explanation"].label.contains("LDL cholesterol"))
        XCTAssertFalse(app.buttons["Refresh history"].exists)
        XCTAssertFalse(app.buttons["About Reading the chart"].exists)
        var refreshRequest = URLRequest(url: URL(string: "http://127.0.0.1:18765/api/v1/test/reset-counts")!)
        refreshRequest.httpMethod = "POST"
        _ = try await URLSession.shared.data(for: refreshRequest)
        app.navigationBars.buttons.firstMatch.tap()
        app.buttons["LDL"].tap()
        XCTAssertTrue(app.staticTexts["Function Health"].firstMatch.waitForExistence(timeout: 10))
        refreshRequest.url = URL(string: "http://127.0.0.1:18765/api/v1/test/counts")!
        let (refreshData, _) = try await URLSession.shared.data(for: refreshRequest)
        let refreshCounts = try XCTUnwrap(JSONSerialization.jsonObject(with: refreshData) as? [String: Int])
        XCTAssertEqual(refreshCounts["metrics/history/get"], 1)
        app.navigationBars.buttons.firstMatch.tap()
    }

    @MainActor func testContextualHelpAndVisibleMetricExplanation() async throws {
        let app = XCUIApplication(); app.launch()
        dismissPasswordPrompts(app)
        if app.buttons["Sign in"].exists {
            let server = app.textFields["http://192.168.1.20:8080"]
            server.tap()
            let old = server.value as? String ?? ""
            if old != "http://192.168.1.20:8080" { server.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: old.count)) }
            server.typeText("http://127.0.0.1:18765")
            app.textFields["Username"].tap(); app.textFields["Username"].typeText("simulator")
            app.secureTextFields["Password"].tap(); app.secureTextFields["Password"].typeText("test-password")
            app.buttons["Sign in"].tap(); dismissPasswordPrompts(app)
        }
        XCTAssertTrue(app.tabBars.buttons["Browse"].waitForExistence(timeout: 10))
        tapTab(app, "Health")
        for title in ["Selected records", "Automatic sync", "Health access", "Available Health data"] {
            XCTAssertFalse(app.buttons["About " + title].exists)
            XCTAssertFalse(app.staticTexts[title].exists)
        }
        XCTAssertTrue(app.navigationBars["Apple Health"].exists)
        tapTab(app, "Browse")
        app.searchFields["Search metrics"].tap(); app.searchFields["Search metrics"].typeText("sleep")
        app.buttons["Sleep"].tap()
        let metricExplanation = app.staticTexts["metric-explanation"]
        reveal(app, metricExplanation)
        XCTAssertTrue(metricExplanation.isHittable)
        XCTAssertFalse(app.buttons["About this metric"].exists)
    }

    @MainActor func testBrowseWeightAndActivityPlacement() async throws {
        let app = XCUIApplication(); app.launch()
        dismissPasswordPrompts(app)
        if app.buttons["Sign in"].exists {
            let server = app.textFields["http://192.168.1.20:8080"]
            server.tap()
            let old = server.value as? String ?? ""
            if old != "http://192.168.1.20:8080" { server.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: old.count)) }
            server.typeText("http://127.0.0.1:18765")
            app.textFields["Username"].tap(); app.textFields["Username"].typeText("simulator")
            app.secureTextFields["Password"].tap(); app.secureTextFields["Password"].typeText("test-password")
            app.buttons["Sign in"].tap(); dismissPasswordPrompts(app)
        }
        XCTAssertTrue(app.tabBars.buttons["Browse"].waitForExistence(timeout: 10))
        tapTab(app, "Browse")
        XCTAssertFalse(app.buttons["Device trends"].exists)
        XCTAssertFalse(app.buttons["Timeline"].exists)
        XCTAssertFalse(app.buttons["Breathing practice"].exists)
        app.searchFields["Search metrics"].tap(); app.searchFields["Search metrics"].typeText("sleep")
        XCTAssertTrue(app.buttons["Sleep"].waitForExistence(timeout: 5))
        app.buttons["Sleep"].tap()
        XCTAssertTrue(app.segmentedControls.firstMatch.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Hours"].firstMatch.exists)
        XCTAssertTrue(app.staticTexts["Apple Watch"].firstMatch.exists)
        reveal(app, app.buttons["Historical Values"])
        app.buttons["Historical Values"].tap()
        XCTAssertTrue(app.staticTexts["Daily records"].exists)
        XCTAssertFalse(app.buttons["2020-01-01"].exists)
        app.navigationBars.buttons.firstMatch.tap()
        app.swipeDown(); app.swipeDown()
        let chartShot = XCTAttachment(screenshot: app.screenshot())
        chartShot.name = "Sleep chart controls and original-source legend"
        chartShot.lifetime = .keepAlways
        add(chartShot)
        XCTAssertFalse(app.buttons["Zoom in"].exists)
        XCTAssertFalse(app.buttons["Show all"].exists)
        let range = app.staticTexts["chart-date-range"].label
        app.segmentedControls.buttons["Year"].tap()
        XCTAssertTrue(app.segmentedControls.buttons["Year"].isSelected)
        XCTAssertNotEqual(range, app.staticTexts["chart-date-range"].label)
        app.segmentedControls.buttons["Day"].tap()
        XCTAssertTrue(app.segmentedControls.buttons["Day"].isSelected)
        app.segmentedControls.buttons["Week"].tap()
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.75, dy: 0.30)).press(forDuration: 0.1, thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.3, dy: 0.30)))
        app.navigationBars.buttons.firstMatch.tap()
        let search = app.searchFields["Search metrics"]
        if search.buttons["Clear text"].exists { search.buttons["Clear text"].tap() }
        for (query, title, unit) in [("resting heart", "Resting heart rate", "BPM"), ("workouts", "Workouts", "Hours"), ("active energy", "Active Energy", "kcal"), ("resting energy", "Resting Energy", "kcal"), ("hrv", "HRV SDNN", "ms")] {
            search.tap(); search.typeText(query)
            XCTAssertTrue(app.buttons[title].waitForExistence(timeout: 5))
            app.buttons[title].tap()
            XCTAssertTrue(app.staticTexts[unit].firstMatch.waitForExistence(timeout: 5))
            XCTAssertNotEqual(app.staticTexts["chart-range-value"].label, "No data")
            XCTAssertTrue(app.buttons["chart-source-apple_health:watch"].exists)
            app.navigationBars.buttons.firstMatch.tap()
            search.buttons["Clear text"].tap()
        }
        search.tap(); search.typeText("weight")
        XCTAssertTrue(app.buttons["Weight"].waitForExistence(timeout: 5))
        app.buttons["Weight"].tap()
        XCTAssertTrue(app.segmentedControls.firstMatch.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["kg"].firstMatch.exists)
        reveal(app, app.buttons["Historical Values"])
        app.buttons["Historical Values"].tap()
        XCTAssertTrue(app.staticTexts["72.5 kg"].waitForExistence(timeout: 5))
        app.navigationBars.buttons.firstMatch.tap()
        app.navigationBars.buttons.firstMatch.tap()
        XCTAssertEqual(search.value as? String, "weight")
        search.buttons["Clear text"].tap()
        search.typeText("heart rate")
        XCTAssertTrue(app.buttons["Heart rate"].waitForExistence(timeout: 5))
        XCTAssertEqual(app.buttons.matching(identifier: "Heart rate").count, 1)
        app.buttons["Heart rate"].tap()
        XCTAssertTrue(app.segmentedControls.firstMatch.waitForExistence(timeout: 5))
        XCTAssertTrue(app.segmentedControls.buttons["Day"].isSelected)
        XCTAssertFalse(app.buttons["Apply sampling protocol"].exists)
        XCTAssertTrue(app.segmentedControls.buttons["Hour"].exists)
        XCTAssertTrue(app.staticTexts["70–90"].firstMatch.waitForExistence(timeout: 5))
        let rangeChart = app.otherElements["metric-range-chart"].firstMatch
        XCTAssertTrue(rangeChart.waitForExistence(timeout: 5))
        rangeChart.coordinate(withNormalizedOffset: CGVector(dx: 0.50, dy: 0.45)).tap()
        XCTAssertTrue(app.staticTexts["SELECTED RANGE"].waitForExistence(timeout: 5))
        let dayShot = XCTAttachment(screenshot: app.screenshot()); dayShot.name = "Heart day selected range"; dayShot.lifetime = .keepAlways; add(dayShot)
        app.segmentedControls.buttons["Hour"].tap()
        XCTAssertTrue(app.segmentedControls.buttons["Hour"].isSelected)
        XCTAssertTrue(app.staticTexts["70–90"].firstMatch.exists)
        let hourShot = XCTAttachment(screenshot: app.screenshot()); hourShot.name = "Heart hourly minute ranges"; hourShot.lifetime = .keepAlways; add(hourShot)
        let sourceToggle = app.buttons["chart-source-apple_health:watch"]
        reveal(app, sourceToggle)
        XCTAssertEqual(sourceToggle.value as? String, "Shown")
        sourceToggle.tap()
        XCTAssertEqual(sourceToggle.value as? String, "Hidden")
        XCTAssertTrue(app.staticTexts["No sources selected"].exists)
        app.segmentedControls.buttons["Day"].tap()
        XCTAssertEqual(sourceToggle.value as? String, "Hidden")
        sourceToggle.tap()
        XCTAssertEqual(sourceToggle.value as? String, "Shown")
        XCTAssertTrue(app.staticTexts["70–90"].firstMatch.exists)


        reveal(app, app.buttons["Historical Values"])
        app.buttons["Historical Values"].tap()
        XCTAssertTrue(app.staticTexts["90 count/min"].waitForExistence(timeout: 5))
        app.navigationBars.buttons.firstMatch.tap()
        app.navigationBars.buttons.firstMatch.tap()
        search.buttons["Clear text"].tap()
        search.typeText("vo2")
        XCTAssertTrue(app.buttons["VO₂ Max"].waitForExistence(timeout: 5))
        var countRequest = URLRequest(url: URL(string: "http://127.0.0.1:18765/api/v1/test/reset-counts")!)
        countRequest.httpMethod = "POST"
        _ = try await URLSession.shared.data(for: countRequest)
        app.buttons["VO₂ Max"].tap()
        XCTAssertTrue(app.staticTexts["ml/kg*min"].firstMatch.waitForExistence(timeout: 5))
        XCTAssertFalse(app.staticTexts["All available VO₂ Max history."].exists)
        XCTAssertFalse(app.staticTexts["Chart"].exists)
        XCTAssertFalse(app.staticTexts["observed"].exists)
        XCTAssertEqual(app.staticTexts.matching(identifier: "ml/kg*min").count, 1)
        for label in ["Year", "Month", "Week"] {
            app.segmentedControls.buttons[label].tap()
            XCTAssertTrue(app.segmentedControls.buttons[label].isSelected)
        }
        countRequest.url = URL(string: "http://127.0.0.1:18765/api/v1/test/counts")!
        let (countData, _) = try await URLSession.shared.data(for: countRequest)
        let counts = try XCTUnwrap(JSONSerialization.jsonObject(with: countData) as? [String: Int])
        XCTAssertGreaterThanOrEqual(counts["wellness/chart"] ?? 0, 3)
        XCTAssertNil(counts["wellness/series"])
        XCTAssertNil(counts["wellness/sources"])

        reveal(app, app.buttons["Historical Values"])
        app.buttons["Historical Values"].tap()
        XCTAssertTrue(app.staticTexts["42 ml/kg*min"].waitForExistence(timeout: 10))
        tapTab(app, "Overview")
        XCTAssertTrue(app.staticTexts["Today"].exists)
        XCTAssertTrue(app.staticTexts["Last 7 days"].exists)
        XCTAssertLessThan(abs(app.staticTexts["Today"].frame.minY - app.staticTexts["Last 7 days"].frame.minY), 30)
        XCTAssertFalse(app.staticTexts["No reference range set"].exists)
        XCTAssertFalse(app.staticTexts["Apple Watch"].exists)
        reveal(app, app.buttons["Breathing practice"])
        XCTAssertTrue(app.buttons["Breathing practice"].exists)
        tapTab(app, "Trends")
        reveal(app, app.buttons["Past day"])
        XCTAssertTrue(app.buttons["Past day"].exists)
        app.buttons["Past day"].tap()
        XCTAssertTrue(app.buttons["Refresh activity"].exists)
        tapTab(app, "Data")
        XCTAssertFalse(app.staticTexts["Archived sources"].exists)
        XCTAssertFalse(app.staticTexts["10 records"].exists)
        reveal(app, app.buttons["Sources Record Count"])
        app.buttons["Sources Record Count"].tap()
        XCTAssertTrue(app.staticTexts["10 records"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["Apple Watch"].firstMatch.exists)
        app.navigationBars.buttons.firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Data"].exists)
        XCTAssertFalse(app.staticTexts["10 records"].exists)
    }

    @MainActor func testReportDeleteConfirmation() async throws {
        func control(_ path: String) async throws -> Data {
            var request = URLRequest(url: URL(string: "http://127.0.0.1:18765/api/v1/\(path)")!)
            request.httpMethod = "POST"
            return try await URLSession.shared.data(for: request).0
        }
        _ = try await control("test/reset")
        _ = try await control("test/seed-report")
        let app = XCUIApplication(); app.launch(); dismissPasswordPrompts(app)
        if app.buttons["Sign in"].exists {
            let server = app.textFields["http://192.168.1.20:8080"]
            server.tap()
            let old = server.value as? String ?? ""
            if old != "http://192.168.1.20:8080" { server.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: old.count)) }
            server.typeText("http://127.0.0.1:18765")
            app.textFields["Username"].tap(); app.textFields["Username"].typeText("simulator")
            app.secureTextFields["Password"].tap(); app.secureTextFields["Password"].typeText("test-password")
            app.buttons["Sign in"].tap(); dismissPasswordPrompts(app)
        }
        XCTAssertTrue(app.tabBars.buttons["Data"].waitForExistence(timeout: 10))
        tapTab(app, "Archive")

        let report = app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "fixture.png")).firstMatch
        XCTAssertTrue(report.waitForExistence(timeout: 10)); report.tap()
        let resultRow = app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "5.5 mmol/L (99.1 mg/dL)")).firstMatch
        XCTAssertTrue(resultRow.waitForExistence(timeout: 5))
        XCTAssertFalse(app.staticTexts["Original reference: not provided"].exists)
        XCTAssertFalse(app.staticTexts.matching(NSPredicate(format: "label CONTAINS %@", "Reference retained as printed")).firstMatch.exists)
        XCTAssertFalse(app.staticTexts["No reference interval was printed"].exists)
        let sourceIcon = app.buttons["View source · page 1"]
        XCTAssertTrue(sourceIcon.exists)
        XCTAssertGreaterThan(sourceIcon.frame.midX, resultRow.frame.midX)
        sourceIcon.tap()
        XCTAssertTrue(app.navigationBars["Original · page 1"].waitForExistence(timeout: 5))
        tapButton(app, "Done")
        let info = app.buttons["report-info"]
        let count = app.descendants(matching: .any).matching(identifier: "report-marker-count").firstMatch
        let confirmAll = app.buttons["confirm-all-results"]
        let delete = app.buttons["report-delete"]
        XCTAssertTrue(delete.waitForExistence(timeout: 5))
        XCTAssertTrue(info.label.contains("Report Info"))
        XCTAssertLessThan(info.frame.midY, count.frame.midY)
        XCTAssertLessThan(count.frame.midY, confirmAll.frame.midY)
        XCTAssertLessThan(confirmAll.frame.midY, delete.frame.midY)
        info.tap()
        XCTAssertTrue(app.navigationBars["Report Info"].waitForExistence(timeout: 5))
        app.buttons["Cancel"].tap()

        _ = try await control("test/reset-counts")
        delete.tap()
        XCTAssertTrue(app.alerts["Delete report?"].waitForExistence(timeout: 5))
        app.alerts.buttons["Cancel"].tap()
        let cancelledCounts = try JSONSerialization.jsonObject(with: await control("test/counts")) as? [String: Int]
        XCTAssertNil(cancelledCounts?["reports/delete"])
        delete.tap()
        let confirm = app.alerts.buttons["Delete report"]
        XCTAssertTrue(confirm.waitForExistence(timeout: 5))
        confirm.tap()
        XCTAssertTrue(app.staticTexts["Add your first report"].waitForExistence(timeout: 10))
        let deletedCounts = try JSONSerialization.jsonObject(with: await control("test/counts")) as? [String: Int]
        XCTAssertEqual(deletedCounts?["reports/delete"], 1)
    }

    @MainActor func testLargeReportCountAndSourceEditing() async throws {
        func control(_ path: String) async throws -> Data {
            var request = URLRequest(url: URL(string: "http://127.0.0.1:18765/api/v1/\(path)")!)
            request.httpMethod = "POST"
            return try await URLSession.shared.data(for: request).0
        }
        _ = try await control("test/reset")
        _ = try await control("test/seed-report")
        _ = try await control("test/expand-report")
        let app = XCUIApplication(); app.launch(); dismissPasswordPrompts(app)
        if app.buttons["Sign in"].exists {
            let server = app.textFields["http://192.168.1.20:8080"]
            server.tap()
            let old = server.value as? String ?? ""
            if old != "http://192.168.1.20:8080" { server.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: old.count)) }
            server.typeText("http://127.0.0.1:18765")
            app.textFields["Username"].tap(); app.textFields["Username"].typeText("simulator")
            app.secureTextFields["Password"].tap(); app.secureTextFields["Password"].typeText("test-password")
            app.buttons["Sign in"].tap(); dismissPasswordPrompts(app)
        }
        XCTAssertTrue(app.tabBars.buttons["Data"].waitForExistence(timeout: 10))
        tapTab(app, "Archive")

        let report = app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "fixture.png")).firstMatch
        XCTAssertTrue(report.waitForExistence(timeout: 10)); report.tap()
        let count = app.descendants(matching: .any).matching(identifier: "report-marker-count").firstMatch
        XCTAssertTrue(count.waitForExistence(timeout: 10))
        XCTAssertEqual(count.value as? String, "126")
        let top = XCTAttachment(screenshot: app.screenshot()); top.lifetime = .keepAlways; add(top)
        let original = app.buttons["report-original"]
        let manual = app.buttons["report-add-result"]
        let delete = app.buttons["report-delete"]
        XCTAssertTrue(original.exists && manual.exists && delete.exists)
        XCTAssertEqual(original.frame.midY, manual.frame.midY, accuracy: 2)
        XCTAssertEqual(original.frame.midY, delete.frame.midY, accuracy: 2)
        XCTAssertFalse(app.buttons["Duplicate or revised report"].exists)
        let source = app.buttons["report-info"]
        XCTAssertTrue(source.exists); source.tap()
        XCTAssertTrue(app.navigationBars["Report Info"].waitForExistence(timeout: 5))
        let sourceExplanation = "The clinic, laboratory, or organization that issued this report."
        XCTAssertFalse(app.staticTexts[sourceExplanation].exists)
        app.buttons["About Source"].tap()
        XCTAssertTrue(app.staticTexts[sourceExplanation].waitForExistence(timeout: 5))
        app.buttons["Done"].tap()
        XCTAssertFalse(app.staticTexts[sourceExplanation].exists)
        XCTAssertLessThan(app.staticTexts["Source"].frame.minY, app.staticTexts["Report Title"].frame.minY)
        let field = app.textFields["Source name"].exists ? app.textFields["Source name"] : app.textViews["Source name"]
        field.tap(); field.typeText("Fixture Laboratory")
        tapButton(app, "Save")
        XCTAssertTrue(app.navigationBars["Report Info"].waitForNonExistence(timeout: 10))
        XCTAssertTrue(source.label.contains("Fixture Laboratory"))
        _ = try await control("test/reset-counts")
        source.tap(); tapButton(app, "Cancel")
        XCTAssertTrue(source.waitForExistence(timeout: 5))
        let data = try await control("test/counts")
        let counts = try JSONSerialization.jsonObject(with: data) as? [String: Int]
        XCTAssertNil(counts?["reports/get"])
        XCTAssertNil(counts?["reports/review"])
        let confirm = app.buttons["confirm-all-results"]
        XCTAssertTrue(confirm.isEnabled)
        _ = try await control("test/fail-review")
        confirm.tap()
        XCTAssertTrue(confirm.waitForExistence(timeout: 10))
        let retry = NSPredicate(format: "enabled == true")
        await fulfillment(of: [XCTNSPredicateExpectation(predicate: retry, object: confirm)], timeout: 10)
        _ = try await control("test/reset-counts")
        confirm.tap()
        let finished = NSPredicate(format: "label CONTAINS %@", "0 pending")
        await fulfillment(of: [XCTNSPredicateExpectation(predicate: finished, object: confirm)], timeout: 10)
        XCTAssertFalse(confirm.isEnabled)
        let confirmationCounts = try JSONSerialization.jsonObject(with: await control("test/counts")) as? [String: Int]
        XCTAssertEqual(confirmationCounts?["reports/review"], 1)
        XCTAssertNil(confirmationCounts?["reports/get"])
        // Traverse the entire long report in both directions, not only its first few rows.
        for _ in 0..<55 {
            if app.buttons["Page 1 · ocr"].isHittable { break }
            app.swipeUp(velocity: .fast)
        }
        XCTAssertTrue(app.buttons["Page 1 · ocr"].isHittable)
        for _ in 0..<55 {
            if source.isHittable { break }
            app.swipeDown(velocity: .fast)
        }
        XCTAssertTrue(source.isHittable)
        for _ in 0..<4 { app.swipeUp() }
        let marker = app.buttons.matching(NSPredicate(format: "label CONTAINS %@", "Marker")).allElementsBoundByIndex.first { $0.isHittable && $0.frame.minY > 200 && $0.frame.maxY < 650 }
        XCTAssertNotNil(marker); marker?.tap()
        XCTAssertTrue(app.buttons["Cancel"].waitForExistence(timeout: 5))
        tapButton(app, "Cancel")
        let shot = XCTAttachment(screenshot: app.screenshot()); shot.lifetime = .keepAlways; add(shot)

    }

    @MainActor func testReportImportChoosesFilesAndScreenshotsWithoutNamingFirst() async throws {
        let app = XCUIApplication(); app.launch(); dismissPasswordPrompts(app)
        if app.buttons["Sign in"].exists {
            let server = app.textFields["http://192.168.1.20:8080"]
            server.tap()
            let old = server.value as? String ?? ""
            if old != "http://192.168.1.20:8080" { server.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: old.count)) }
            server.typeText("http://127.0.0.1:18765")
            app.textFields["Username"].tap(); app.textFields["Username"].typeText("simulator")
            app.secureTextFields["Password"].tap(); app.secureTextFields["Password"].typeText("test-password")
            app.buttons["Sign in"].tap(); dismissPasswordPrompts(app)
        }
        XCTAssertTrue(app.tabBars.buttons["Data"].waitForExistence(timeout: 10))
        tapTab(app, "Archive")
        XCTAssertFalse(app.buttons["Import PDF"].exists)
        XCTAssertFalse(app.buttons["Import files"].exists)
        app.buttons["Upload"].tap()
        XCTAssertTrue(app.buttons["Camera"].waitForExistence(timeout: 5))
        app.buttons["Import PDF"].tap()
        XCTAssertFalse(app.textFields["Clinic or laboratory"].exists)
        XCTAssertTrue(app.buttons["Cancel"].waitForExistence(timeout: 10))
        app.buttons["Cancel"].tap()
        app.buttons["Upload"].tap()
        XCTAssertTrue(app.buttons["Import PDF"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Import PDF"].isEnabled)
        XCTAssertFalse(app.textFields["Clinic or laboratory"].exists)
        app.buttons["Import Images"].tap()
        XCTAssertTrue(app.buttons["Cancel"].waitForExistence(timeout: 10))
        app.buttons["Cancel"].tap()
        XCTAssertTrue(app.buttons["Upload"].waitForExistence(timeout: 5))
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
        XCTAssertFalse(app.textFields["Clinic or laboratory"].firstMatch.exists)
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
        if app.navigationBars["Change password"].exists { app.buttons["Sign out"].tap() }
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
        XCTAssertFalse(app.buttons["Import PDF"].exists)
        app.buttons["Upload"].tap()
        XCTAssertTrue(app.buttons["Import PDF"].isHittable)
        app.buttons["Import PDF"].tap()
        XCTAssertFalse(app.textFields["Clinic or laboratory"].exists)
        XCTAssertTrue(app.buttons["Cancel"].waitForExistence(timeout: 10))
        app.buttons["Cancel"].tap()
        XCTAssertTrue(app.buttons["Upload"].waitForExistence(timeout: 5))

        // Relaunch after the system password sheet to verify Keychain restoration.
        app.terminate()
        app.launch()
        XCTAssertTrue(app.tabBars.buttons["Data"].waitForExistence(timeout: 10))
        tapTab(app, "Archive")
        app.buttons["Upload"].tap()
        XCTAssertTrue(app.buttons["Import PDF"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Import PDF"].isEnabled)
        XCTAssertFalse(app.textFields["Clinic or laboratory"].exists)
        app.buttons["Import PDF"].tap()
        XCTAssertTrue(app.buttons["Cancel"].waitForExistence(timeout: 10))
        app.buttons["Cancel"].tap()
        XCTAssertTrue(app.buttons["Cancel"].waitForNonExistence(timeout: 5))
        app.buttons["Upload"].tap()
        XCTAssertTrue(app.buttons["Import Images"].waitForExistence(timeout: 5))
        app.buttons["Import Images"].tap()
        XCTAssertTrue(app.buttons["Cancel"].waitForExistence(timeout: 10))
        app.buttons["Cancel"].tap()
        XCTAssertTrue(app.buttons["Cancel"].waitForNonExistence(timeout: 5))
        let system = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        if system.buttons["Not Now"].exists { system.buttons["Not Now"].tap() }
        tapTab(app, "Overview")
        XCTAssertTrue(app.staticTexts["Today"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["7 hr 30 min"].firstMatch.waitForExistence(timeout: 10))
        XCTAssertFalse(app.buttons["Device trends"].exists)
        XCTAssertFalse(app.buttons["Timeline"].exists)
        XCTAssertTrue(app.buttons["Breathing practice"].exists)
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
        XCTAssertFalse(app.staticTexts["My Watch"].firstMatch.exists)
        let afterAliases = try await aliasRequestCounts()
        XCTAssertEqual(afterAliases["wellness/day"], beforeAliases["wellness/day"])
        XCTAssertEqual(afterAliases["wellness/preferences/save", default: 0] - beforeAliases["wellness/preferences/save", default: 0], 1)
        tapTab(app, "Data")
        for _ in 0..<4 {
            if app.navigationBars["Data"].exists { break }
            app.navigationBars.buttons.firstMatch.tap()
        }
        reveal(app, app.buttons["Manual entries"])
        app.buttons["Manual entries"].tap()
        XCTAssertTrue(app.buttons["Record weight"].waitForExistence(timeout: 5))
        app.buttons["Record weight"].tap()
        XCTAssertTrue(app.textFields["Weight (kg)"].waitForExistence(timeout: 5))
        app.textFields["Weight (kg)"].tap()
        app.textFields["Weight (kg)"].typeText("72.5")
        app.buttons["Cancel"].tap()
        XCTAssertTrue(app.buttons["Record an event or observation"].exists)
        app.navigationBars.buttons.firstMatch.tap()
        tapTab(app, "Browse")
        XCTAssertFalse(app.buttons["Record weight"].exists)
        let searchField = app.searchFields["Search metrics"]
        XCTAssertTrue(searchField.waitForExistence(timeout: 5))
        searchField.tap()
        searchField.typeText("calcium")
        XCTAssertTrue(app.buttons["Calcium"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["LDL"].exists)
        searchField.buttons["Clear text"].tap(); searchField.typeText("LDL")
        XCTAssertTrue(app.buttons["LDL"].waitForExistence(timeout: 10))
        app.buttons["LDL"].tap()
        XCTAssertFalse(app.staticTexts["All sources"].exists)
        XCTAssertTrue(app.staticTexts["Function Health"].firstMatch.waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["Local Clinic"].firstMatch.exists)
        app.navigationBars.buttons.firstMatch.tap()
        searchField.buttons["Clear text"].tap(); searchField.typeText("calcium")
        XCTAssertTrue(app.buttons["Calcium"].waitForExistence(timeout: 10))
        app.buttons["Calcium"].tap()
        XCTAssertTrue(app.staticTexts["About this metric"].waitForExistence(timeout: 10))
        XCTAssertFalse(app.staticTexts["See laboratory note"].exists)
        reveal(app, app.buttons["Historical Values"])
        app.buttons["Historical Values"].tap()
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
        XCTAssertTrue(app.buttons["report-info"].waitForExistence(timeout: 10))
        app.buttons["report-info"].tap()
        XCTAssertTrue(app.navigationBars["Report Info"].waitForExistence(timeout: 5))
        // Cancel preserves the underlying report and makes no read request.
        var countsRequest = URLRequest(url: URL(string: "http://127.0.0.1:18765/api/v1/test/reset-counts")!)
        countsRequest.httpMethod = "POST"
        _ = try await URLSession.shared.data(for: countsRequest)
        tapButton(app, "Cancel")
        XCTAssertTrue(app.buttons["report-info"].waitForExistence(timeout: 5))
        tapTab(app, "Trends")
        tapTab(app, "Archive")
        XCTAssertTrue(app.buttons["report-info"].waitForExistence(timeout: 5))
        countsRequest.url = URL(string: "http://127.0.0.1:18765/api/v1/test/counts")!
        let (countData, _) = try await URLSession.shared.data(for: countsRequest)
        let counts = try JSONSerialization.jsonObject(with: countData) as? [String: Int]
        XCTAssertNil(counts?["reports/get"])
        XCTAssertNil(counts?["reports/list"])
        app.buttons["report-info"].tap()
        XCTAssertTrue(app.navigationBars["Report Info"].waitForExistence(timeout: 5))
        tapButton(app, "Save")
        XCTAssertTrue(app.navigationBars["Report Info"].waitForNonExistence(timeout: 10))
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
        XCTAssertTrue(app.buttons["report-info"].waitForExistence(timeout: 15))
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
