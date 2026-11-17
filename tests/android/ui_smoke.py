"""UI checks against the synthetic API, inside the repository Android emulator container."""
import argparse
import json
import re
import subprocess
import time
import xml.etree.ElementTree as ET

CONTAINER = "longlivehealth-android-test"


def adb(*arguments):
    return subprocess.check_output(["docker", "exec", CONTAINER, "adb", *arguments], text=True)


def tree():
    for _ in range(5):
        output = adb("shell", "uiautomator", "dump", "/sdcard/window.xml")
        if "dumped to" in output:
            return ET.fromstring(adb("shell", "cat", "/sdcard/window.xml"))
        time.sleep(.25)
    raise AssertionError("Android UI hierarchy is unavailable")


def inspect():
    for node in tree().iter("node"):
        text = node.get("text") or node.get("content-desc") or node.get("hint")
        if text or node.get("class") == "android.widget.EditText":
            print(node.get("class"), repr(text), node.get("bounds"), node.get("enabled"))


def tap_node(node):
    a, b, c, d = map(int, re.findall(r"\d+", node.get("bounds")))
    adb("shell", "input", "tap", str((a + c) // 2), str((b + d) // 2))


def tap(text):
    nodes = list(tree().iter("node"))
    for node in nodes:
        if text in (node.get("text"), node.get("content-desc")):
            tap_node(node)
            return
    raise AssertionError(f"UI item is missing: {text}")


def login():
    fields = [node for node in tree().iter("node") if node.get("class") == "android.widget.EditText"]
    assert len(fields) == 3, f"Expected three login fields, got {len(fields)}"
    for index, value in enumerate(("http://10.0.2.2:18765", "simulator", "simulator-password")):
        fields = [node for node in tree().iter("node") if node.get("class") == "android.widget.EditText"]
        tap_node(fields[index])
        adb("shell", "input", "keyevent", "KEYCODE_MOVE_END")
        adb("shell", "input", "keycombination", "KEYCODE_CTRL_LEFT", "KEYCODE_A")
        adb("shell", "input", "keyevent", "KEYCODE_DEL")
        adb("shell", "input", "text", value)
        adb("shell", "input", "keyevent", "KEYCODE_BACK")
    tap("Sign in")
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        labels = [n.get("text") for n in tree().iter("node")]
        if "Insights" in labels:
            assert "Could not complete the action" not in labels
            print("PASS: Login and Insights")
            return
        time.sleep(0.5)
    inspect()
    raise AssertionError("Login did not open Insights")


def labels():
    return [node.get("text") or node.get("content-desc") for node in tree().iter("node")]


def wait_for(text):
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        current = labels()
        assert "Could not complete the action" not in current, current
        if text in current:
            return
        time.sleep(0.25)
    raise AssertionError(f"Did not display {text}: {labels()}")


def navigation():
    # A process restart must restore the encrypted session without entering a password.
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "wm", "size", "reset")
    adb("shell", "wm", "density", "reset")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Insights")
    current = labels()
    assert all(tab in current for tab in ("Insights", "Browse", "Data")), current
    assert "Overview" not in current and "Trends" not in current, "Retired tabs returned"
    wait_for("Sleep")
    tap("Browse"); wait_for("Health Categories"); tap("Heart")
    wait_for("Heart rate"); tap("Heart rate"); wait_for("RANGE")
    assert sum((text or "").casefold() == "heart rate" for text in labels()) == 1, "Duplicate chart heading"
    tap("Y"); wait_for("RANGE")
    tap("D"); wait_for("Apple Watch")
    tap("Apple Watch"); wait_for("No sources selected")
    tap("Apple Watch"); wait_for("RANGE")
    adb("shell", "wm", "size", "2208x1840")
    adb("shell", "wm", "density", "320")
    wait_for("Heart Rate")
    assert "RANGE" in labels(), "Fold transition lost the detail"
    tap("Data"); wait_for("Blood Work")
    tap("Blood Work"); wait_for("Example Laboratory")
    tap("Example Laboratory"); wait_for("Total Markers")
    assert "126" in labels()
    wait_for("126 results ready to confirm")
    tap("126 results ready to confirm")
    wait_for("Marker 1")
    for _ in range(4):
        adb("shell", "input", "swipe", "1450", "1500", "1450", "450", "350")
    assert "Marker 1" not in labels(), "Report list did not scroll"
    # Switching away and back must keep the report, not reset to the Data menu.
    tap("Browse"); wait_for("RANGE")
    tap("Data")
    wait_for("Ready to Confirm")
    adb("shell", "input", "keyevent", "KEYCODE_BACK")
    wait_for("Confirm"); tap("Confirm"); wait_for("0 results ready to confirm")
    tap("Report Info"); wait_for("Report Info")
    assert "Report Title" in labels(), "Report editor missing title"
    print("PASS: session persistence, chart periods/source toggles, fold state, report scroll/tab state/Confirm All/editor")


def insight_cards():
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "wm", "size", "reset")
    adb("shell", "wm", "density", "reset")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Insights")
    wait_for("Sleep")
    last_card = "Cardiovascular Review"
    size = adb("shell", "wm", "size")
    width, height = map(int, re.findall(r"(\d+)x(\d+)", size)[-1])
    for _ in range(10):
        if last_card in labels():
            break
        adb("shell", "input", "swipe", str(width // 2), str(height * 3 // 4),
            str(width // 2), str(height // 3), "420")
    wait_for(last_card)
    adb("shell", "input", "swipe", str(width // 2), str(height * 3 // 4),
        str(width // 2), str(height // 3), "420")
    current = labels()
    assert last_card in current
    assert not {"Ask about your data", "AI Reviews"}.intersection(current)
    assert not any((label or "").startswith("Updated ") for label in current)
    tap(last_card)
    wait_for("Done")
    tap("Done")
    wait_for(last_card)
    print("PASS: Insights ends at the last card and still opens and closes its details")


def selectors():
    # Mixed static/dynamic Picker content previously crashed the native bridge
    # before the user could select anything, including with an empty catalog.
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Insights")
    assert "Compare measurements" not in labels()
    tap("Insights"); wait_for("Cardiovascular Review")
    assert "Not calculated" in labels()
    assert "Behavior associations" not in labels()
    print("PASS: Integrated Insights render without native crash")


def browse_switch():
    """Replace an open Browse detail by tapping another metric in the sidebar."""

    def catalog_requests():
        counts = subprocess.check_output([
            "docker", "exec", CONTAINER, "curl", "-fsS", "-X", "POST",
            "http://127.0.0.1:18765/api/v1/test/counts",
        ], text=True)
        return json.loads(counts).get("browse/catalog", 0)

    def metric_node(title, sidebar):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            for node in tree().iter("node"):
                label = node.get("text") or node.get("content-desc") or ""
                left = int(re.findall(r"\d+", node.get("bounds"))[0])
                if label.casefold() == title.casefold() and (left < 680) == sidebar:
                    return node
        raise AssertionError(f"{title} missing from the {'sidebar' if sidebar else 'detail'}")

    try:
        adb("shell", "am", "force-stop", "dev.longlivehealth.android")
        adb("shell", "wm", "size", "2208x1840")
        adb("shell", "wm", "density", "320")
        adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
        wait_for("Insights")
        tap("Browse")
        wait_for("Health Categories")
        tap("Heart")
        for title in ("Heart rate", "HRV SDNN", "Heart rate"):
            tap_node(metric_node(title, sidebar=True))
            metric_node(title, sidebar=False)
            wait_for("RANGE")
        requests_before_tab_switch = catalog_requests()
        tap("Data")
        wait_for("Blood Work")
        tap("Browse")
        metric_node("Heart rate", sidebar=False)
        metric_node("HRV SDNN", sidebar=True)
        assert catalog_requests() == requests_before_tab_switch, "Returning to Browse reloaded the catalog"

        adb("shell", "wm", "size", "1080x2400")
        adb("shell", "wm", "density", "420")
        wait_for("RANGE")
        adb("shell", "input", "keyevent", "KEYCODE_BACK")
        wait_for("HRV SDNN")
        assert "RANGE" not in labels(), "Back did not dismiss the folded detail"
        print("PASS: wide Browse switches between metrics, keeps the sidebar, and preserves folded Back")
    finally:
        adb("shell", "wm", "size", "reset")
        adb("shell", "wm", "density", "reset")


def report_date():
    def control(path):
        return json.loads(subprocess.check_output([
            "docker", "exec", CONTAINER, "curl", "-fsS", "-X", "POST",
            "http://127.0.0.1:18765/api/v1/" + path,
        ], text=True))

    control("test/reset")
    control("test/seed-report")
    control("test/report-test-code")
    original = control("reports/get")["observations"][0]
    recommendation = original["lab_test_name"]
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Insights")
    tap("Data")
    wait_for("Blood Work")
    tap("Blood Work")
    wait_for("Example Laboratory")
    nodes = list(tree().iter("node"))

    def bounds(node):
        return tuple(map(int, re.findall(r"\d+", node.get("bounds"))))

    source = next(n for n in nodes if n.get("text") == "Example Laboratory")
    collection = next(n for n in nodes if any("2026-10-01" in (n.get(key) or "") for key in ("text", "content-desc")))
    _, top, _, bottom = bounds(collection)
    upload = next(n for n in nodes if n is not collection and "2026" in (n.get("text") or "")
                  and bounds(n)[1] <= bottom and bounds(n)[3] >= top)
    assert bounds(source)[0] < bounds(collection)[0] < bounds(upload)[0]
    print("PASS: source, YYYY-MM-DD collection date, then upload date in the report row")
    tap_node(source)
    wait_for("Thyroid-stimulating hormone")
    wait_for("LOINC 11580-8")
    for save in (False, True):
        tap("Thyroid-stimulating hormone")
        wait_for("Review result")
        wait_for("TSH")
        wait_for("11580-8")
        assert "TSH (11580-8)" not in labels()
        current = list(tree().iter("node"))
        name = next(node for node in current if node.get("text") == recommendation)
        use = next(node for node in current if node.get("content-desc") == "Use recommended name"
                   or node.get("text") == "Use")
        assert bounds(use)[0] > bounds(name)[0], "Use belongs to the right of the recommendation"
        tap_node(use)
        assert any(node.get("class") == "android.widget.EditText" and node.get("text") == recommendation
                   for node in tree().iter("node")), "Use did not fill the Name field"
        assert control("reports/get")["observations"][0] == original
        tap("Save" if save else "Cancel")
        wait_for("Report Info")

    saved = control("reports/get")["observations"][0]
    assert saved["status"] == original["status"]
    expected = dict(original["payload"], raw_name=f"{recommendation} (11580-8)")
    assert saved["payload"] == expected
    print("PASS: report dates, recommended name fills the draft, Cancel discards it, and Save preserves the LOINC code and source")


def chart_scroll():
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "wm", "size", "reset")
    adb("shell", "wm", "density", "reset")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Insights"); tap("Browse"); wait_for("Health Categories")
    tap("Heart"); tap("Heart rate"); wait_for("RANGE")
    bounds = tree().find("node").get("bounds")
    _, _, width, height = map(int, re.findall(r"\d+", bounds))
    for _ in range(4):
        # Start within the plot, not the header or outer list margin.
        adb("shell", "input", "swipe", str(width // 2), str(int(height * .73)),
            str(width // 2), str(int(height * .30)), "450")
    wait_for("Historical Values")
    tap("Historical Values"); wait_for("Historical Values")
    assert adb("shell", "pidof", "dev.longlivehealth.android").strip()
    print("PASS: vertical gestures over chart reach history without trapping list scroll")


def fixture_state():
    return json.loads(subprocess.check_output(["docker", "exec", CONTAINER, "curl", "-fsS", "-X", "POST", "http://127.0.0.1:18765/api/v1/test/state"], text=True))


def camera_import():
    before = len(fixture_state()["upload_ids"])
    tap("Data"); tap("Blood Work"); tap("Import Blood Work"); tap("Camera")
    wait_for("Take report photos")
    nodes = list(tree().iter("node"))
    heading = next(n for n in nodes if n.get("text") == "Camera report")
    status = next((n for n in nodes if n.get("resource-id", "").endswith(":id/status_bar")), None)
    if status is not None:
        heading_top = int(re.findall(r"\d+", heading.get("bounds"))[1])
        status_bottom = int(re.findall(r"\d+", status.get("bounds"))[3])
        assert heading_top >= status_bottom, "Camera title overlaps the system status bar"
    for page in (1, 2):
        tap("Camera"); wait_for("Shutter"); tap("Shutter"); wait_for("Done"); tap("Done")
        wait_for(f"Page {page} of {page}")
    tap("Delete"); wait_for("Page 1 of 1"); tap("Import")
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        current = labels()
        assert "Could not complete the action" not in current, current
        if len(fixture_state()["upload_ids"]) == before + 1 and "Retry uploads" not in current:
            print("PASS: two camera pages, delete one, one PDF automatically uploaded without Retry")
            return
        time.sleep(0.25)
    raise AssertionError("Camera PDF did not upload automatically")


def original_preview():
    def control(action):
        return json.loads(subprocess.check_output([
            "docker", "exec", CONTAINER, "curl", "-fsS", "-X", "POST",
            "http://127.0.0.1:18765/api/v1/test/" + action,
        ], text=True))

    def open_report():
        adb("shell", "am", "force-stop", "dev.longlivehealth.android")
        adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
        wait_for("Insights")
        tap("Data")
        wait_for("Blood Work")
        tap("Blood Work")
        wait_for("Example Laboratory")
        tap("Example Laboratory")

    def check_page(page):
        wait_for(f"Page {page} of 4")
        wait_for(f"Original page {page}")

    def scroll_to(page, toward_beginning):
        current = tree()
        scroller = next(n for n in current.iter("node") if n.get("class") == "android.widget.ListView")
        left, top, right, bottom = map(int, re.findall(r"\d+", scroller.get("bounds")))
        x = str((left + right) // 2)
        upper = str(top + (bottom - top) // 5)
        lower = str(bottom - (bottom - top) // 5)
        start, end = (upper, lower) if toward_beginning else (lower, upper)
        for _ in range(9):
            if f"Page {page} of 4" in labels():
                break
            adb("shell", "input", "swipe", x, start, x, end, "370")
        check_page(page)

    control("reset")
    control("seed-multipage-report")
    open_report()
    wait_for("View source · page 2")
    before = control("counts")
    tap("View source · page 2")
    check_page(2)
    current = list(tree().iter("node"))
    arrow = next(n for n in current if n.get("content-desc") == "Back")
    x1, y1, x2, y2 = map(int, re.findall(r"\d+", arrow.get("bounds")))
    assert x1 > 0 and y1 > 0 and x2 - x1 == y2 - y1 and arrow.get("clickable") == "true"
    scroll_to(1, toward_beginning=True)
    scroll_to(4, toward_beginning=False)
    tap("Back")
    wait_for("View source · page 2")
    assert before.get("reports/get") == control("counts").get("reports/get")

    tap("View original report")
    check_page(1)
    scroll_to(4, toward_beginning=False)
    adb("shell", "input", "keyevent", "KEYCODE_BACK")
    wait_for("Report Info")
    assert "Open original report" not in labels()

    control("reset")
    control("seed-report")
    open_report()
    wait_for("View original report")
    tap("View original report")
    wait_for("Original report image")
    tap("Back")
    wait_for("Report Info")
    print("PASS: both PDF entry points, vertical page scrolling, inset Back, one-step return, and image preview")


def rescan_confirmation():
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Insights")
    tap("Data"); tap("Blood Work"); wait_for("Example Laboratory"); tap("Example Laboratory")
    wait_for("Rescan report")
    actions = ["View original report", "Add result manually", "Rescan report", "Delete report"]
    nodes = list(tree().iter("node"))
    bounds = []
    for action in actions:
        node = next(n for n in nodes if action in (n.get("text"), n.get("content-desc")))
        bounds.append(list(map(int, re.findall(r"\d+", node.get("bounds")))))
    assert all(bounds[i][0] < bounds[i + 1][0] for i in range(3)), bounds
    assert max(b[1] for b in bounds) - min(b[1] for b in bounds) <= 4, bounds
    tap("Rescan report"); wait_for("Rescan report?")
    wait_for("Rescan returns every result in this report to Pending. Results are removed from Browse and charts until confirmed again.")
    tap("Cancel"); wait_for("Report Info")
    assert adb("shell", "pidof", "dev.longlivehealth.android").strip()
    print("PASS: four report actions in order on one row, Rescan confirmation and cancel")


def blood_work_categories():
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Insights")
    tap("Browse"); wait_for("Health Categories"); tap("Heart")
    wait_for("Heart rate")
    assert "LDL" not in labels(), "Blood lipid incorrectly appears in Heart"
    adb("shell", "input", "keyevent", "KEYCODE_BACK")
    wait_for("Health Categories")
    for _ in range(5):
        if "Blood Work" in labels():
            break
        size = adb("shell", "wm", "size")
        width, height = map(int, re.findall(r"(\d+)x(\d+)", size)[-1])
        adb("shell", "input", "swipe", str(width // 4), str(height * 3 // 4), str(width // 4), str(height // 3), "350")
    wait_for("Blood Work"); tap("Blood Work")
    wait_for("LDL"); wait_for("Calcium")
    print("PASS: Heart keeps heart rate; Blood Work contains LDL and Calcium")


def report_scroll():
    """Reverse scroll through a long Needs Review section in both report layouts."""
    def control(path, payload=None):
        subprocess.check_call([
            "docker", "exec", CONTAINER, "curl", "-fsS", "-X", "POST",
            "-H", "Content-Type: application/json", "-d", json.dumps(payload or {}),
            "http://127.0.0.1:18765/api/v1/" + path,
        ], stdout=subprocess.DEVNULL)

    def visible():
        return {(label or "").casefold() for label in labels()}

    headers = {"report info", "review results", "needs review"}

    try:
        for wide in (False, True):
            control("test/reset")
            control("test/seed-report")
            control("test/expand-report", {"needs_review": True})
            adb("shell", "am", "force-stop", "dev.longlivehealth.android")
            width, height, density = (2208, 1840, 320) if wide else (1080, 2400, 420)
            adb("shell", "wm", "size", f"{width}x{height}")
            adb("shell", "wm", "density", str(density))
            adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
            wait_for("Insights")
            tap("Data")
            wait_for("Blood Work")
            tap("Blood Work")
            wait_for("Example Laboratory")
            tap("Example Laboratory")
            wait_for("Report Info")
            assert headers <= visible(), "Report sections missing at the top"

            x = str(width * 3 // 4 if wide else width // 2)
            upper, lower = str(height * 27 // 100), str(height * 75 // 100)
            for _ in range(3):
                adb("shell", "input", "swipe", x, lower, x, upper, "400")
            scrolled = visible()
            assert headers.isdisjoint(scrolled), "Report headings did not scroll out of view"
            assert any(re.fullmatch(r"marker \d+", label) for label in scrolled), "No review rows after scrolling"

            returned = visible()
            for _ in range(10):
                if headers <= returned:
                    break
                adb("shell", "input", "swipe", x, upper, x, lower, "400")
                returned = visible()
            assert headers <= returned, f"Could not scroll back to sections: {headers - returned}"
            tap("Report Info")
            wait_for("Cancel")
            tap("Cancel")
            wait_for("Report Info")
    finally:
        adb("shell", "wm", "size", "reset")
        adb("shell", "wm", "density", "reset")
    print("PASS: report scrolls back to Report Info and review sections on narrow and wide screens")


def report_metric_review():
    def control(path):
        return json.loads(subprocess.check_output([
            "docker", "exec", CONTAINER, "curl", "-fsS", "-X", "POST",
            "http://127.0.0.1:18765/api/v1/" + path,
        ], text=True))

    for scenario in ("test/report-metric-suggestions", "test/report-negative-suggestion",
                     "test/report-no-metric-suggestions"):
        control("test/reset")
        control("test/seed-report")
        control(scenario)
        adb("shell", "am", "force-stop", "dev.longlivehealth.android")
        adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
        wait_for("Insights")
        wait_for("Data")
        tap("Data")
        wait_for("Blood Work")
        tap("Blood Work")
        wait_for("Example Laboratory")
        tap("Example Laboratory")
        if scenario != "test/report-no-metric-suggestions":
            negative = scenario == "test/report-negative-suggestion"
            wait_for("Negative mg/dL" if negative else "99.1 mg/dL (5.5 mmol/L)")
            tap("Suggested match")
            wait_for("Review result")
            wait_for("Metric maps to")
            wait_for("Appearance - urine" if negative else "Glucose")
            assert "Unmapped" not in labels()
            assert "No matching Browse metric. Review this result." not in labels()
            if not negative:
                tap("Suggested match")
                wait_for("Mark as New")
                tap("Review with the full metric list")
                wait_for("Glucose")
            if negative:
                warnings = labels()
                assert any("Unit mismatch · mg/dL" in label for label in warnings)
                assert any("Verify the assay threshold." in label for label in warnings)
                tap("Appearance - urine")
                wait_for("Glucose")
                tap("Glucose")
                wait_for("Glucose")
                tap("Cancel")
                assert control("reports/get")["observations"][0]["payload"]["metric_id"] is None
                tap("Suggested match")
                wait_for("Appearance - urine")
                size = adb("shell", "wm", "size")
                width, height = map(int, re.findall(r"(\d+)x(\d+)", size)[-1])
                for _ in range(5):
                    unit = next((node for node in tree().iter("node")
                                 if node.get("class") == "android.widget.EditText"
                                 and node.get("text") == "mg/dL"), None)
                    if unit is not None:
                        break
                    adb("shell", "input", "swipe", str(width // 2), str(height * 3 // 4),
                        str(width // 2), str(height // 3), "400")
                assert unit is not None, "Original unit field missing"
                tap_node(unit)
                adb("shell", "input", "keyevent", "KEYCODE_MOVE_END")
                adb("shell", "input", "keycombination", "KEYCODE_CTRL_LEFT", "KEYCODE_A")
                adb("shell", "input", "text", "mg/L")
                adb("shell", "input", "keyevent", "KEYCODE_BACK")
                wait_for("mg/L")
            tap("Save Draft")
            wait_for("0 results ready to confirm" if negative else "1 result ready to confirm")
            saved = control("reports/get")["observations"][0]
            assert saved["payload"]["metric_id"] == ("urine_appearance" if negative else "glucose")
            assert saved["status"] == "pending"
            if negative:
                assert saved["payload"]["raw_result"] == "Negative"
                assert saved["payload"]["raw_unit"] == "mg/L"
            else:
                tap("1 result ready to confirm")
                wait_for("5.5 mmol/L")
        else:
            issue = "New"
            wait_for("5.5 mmol/L")
            tap(issue)
            wait_for("Mark as New")
            wait_for("No similar Browse metrics found.")
            wait_for("OCR suggestion: Zyphorin panel activity · mmol/L")
            tap("Mark as New")
            wait_for("Save Draft")
            assert any(n.get("class") == "android.widget.EditText" for n in tree().iter("node"))
            tap("Cancel")
            wait_for(issue)
            assert control("reports/get")["observations"][0]["payload"]["metric_id"] is None
            tap(issue)
            wait_for("Mark as New")
            tap("Mark as New")
            wait_for("Save Draft")
            tap("Save Draft")
            wait_for("1 result ready to confirm")
            tap("1 result ready to confirm")
            wait_for("Zyphorin panel activity")
            saved = control("reports/get")["observations"][0]
            assert saved["status"] == "pending"
            assert saved["payload"]["metric_id"] is None
            assert saved["payload"]["unmapped_choice"]["kind"] == "new"
            tap("Confirm")
            wait_for("0 results ready to confirm")
            saved = control("reports/get")["observations"][0]
            assert saved["status"] == "confirmed"
            assert saved["created_metric"] == {
                "name": "Zyphorin panel activity", "standard_unit": "mmol/L", "metric_id": "custom:fixture",
            }
            assert saved["payload"]["raw_result"] == "5.5"
    print("PASS: suggested mapping, independent Negative and unit warnings, and cancel/create when no match exists")


def report_mapping_layout():
    def control(path):
        return json.loads(subprocess.check_output([
            "docker", "exec", CONTAINER, "curl", "-fsS", "-X", "POST",
            "http://127.0.0.1:18765/api/v1/" + path,
        ], text=True))

    control("test/reset")
    control("test/seed-report")
    control("test/report-urine-mapping")
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Insights")
    wait_for("Data")
    tap("Data")
    wait_for("Blood Work")
    tap("Blood Work")
    wait_for("Example Laboratory")
    tap("Example Laboratory")
    wait_for("Appearance - urine")
    tap("Appearance - urine")
    wait_for("Review result")
    wait_for("Metric mapping")
    wait_for("Appearance - urine")
    nodes = list(tree().iter("node"))
    position = {n.get("text"): int(re.findall(r"\d+", n.get("bounds"))[1]) for n in nodes if n.get("text")}
    assert position["MAPPING"] < position["Metric mapping"] < position["AS PRINTED ON THE REPORT"]
    assert "COMPARISON" not in position and "About Metric mapping" not in labels()
    tap("Appearance - urine")
    wait_for("Glucose")
    tap("Glucose")
    size = adb("shell", "wm", "size")
    width, height = map(int, re.findall(r"(\d+)x(\d+)", size)[-1])
    for _ in range(4):
        assert not {"REVIEW DECISION", "About Review decision", "Review decision status"}.intersection(labels())
        adb("shell", "input", "swipe", str(width // 2), str(height * 3 // 4),
            str(width // 2), str(height // 3), "400")
    wait_for("Save")
    tap("Save")
    wait_for("Clear")
    row = control("reports/get")["observations"][0]
    assert row["payload"]["metric_id"] == "glucose"
    assert row["payload"]["raw_name"] == "Appearance (5767-9)"
    assert row["status"] == "confirmed"
    print("PASS: resolved mapping can be edited, and Save preserves status without a status picker")


def report_status_actions(check_chart=False):
    def control(path, body=None):
        return json.loads(subprocess.check_output([
            "docker", "exec", CONTAINER, "curl", "-fsS", "-X", "POST",
            "-H", "Content-Type: application/json", "-d", json.dumps(body or {}),
            "http://127.0.0.1:18765/api/v1/" + path,
        ], text=True))

    control("test/reset")
    control("test/seed-report")
    control("test/expand-report", {"count": 3})
    control("test/mark-last-result-uncertain")
    original = control("reports/get")["observations"]
    if check_chart:
        control("test/enable-report-status-chart")
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Insights")
    if check_chart:
        tap("Browse")
        wait_for("Health Categories")
        for _ in range(5):
            if "Blood Work" in labels():
                break
            size = adb("shell", "wm", "size")
            width, height = map(int, re.findall(r"(\d+)x(\d+)", size)[-1])
            adb("shell", "input", "swipe", str(width // 4), str(height * 3 // 4), str(width // 4), str(height // 3), "350")
        wait_for("Blood Work")
        tap("Blood Work")
        wait_for("LDL")
        tap("LDL")
        wait_for("mg/dL")
    tap("Data")
    wait_for("Blood Work")
    tap("Blood Work")
    wait_for("Example Laboratory")
    tap("Example Laboratory")

    wait_for("2 results ready to confirm")
    wait_for("Needs Review")
    tap("2 results ready to confirm")
    wait_for("Ready to Confirm")
    adb("shell", "input", "keyevent", "KEYCODE_BACK")
    wait_for("2 results ready to confirm")

    before = control("test/counts")
    control("test/update-observation-elsewhere")
    tap("Confirm")
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        counts = control("test/counts")
        if counts.get("reports/get", 0) > before.get("reports/get", 0):
            break
        time.sleep(.2)
    else:
        raise AssertionError("A stale batch did not refresh the report")
    assert counts["reports/review"] == before.get("reports/review", 0) + 1
    assert counts["reports/get"] == before.get("reports/get", 0) + 1
    assert all(row["status"] == "pending" for row in control("reports/get")["observations"])

    tap("Confirm")
    wait_for("0 results ready to confirm")
    statuses = {row["observation_id"]: row["status"] for row in control("reports/get")["observations"]}
    assert [statuses[row["observation_id"]] for row in original] == ["confirmed", "confirmed", "pending"]

    tap("Review")
    wait_for("Review result")
    control("test/fail-review")
    tap("Confirm")
    wait_for("Could not save")
    tap("OK")
    wait_for("Review result")
    assert next(row for row in control("reports/get")["observations"]
                if row["observation_id"] == original[-1]["observation_id"])["status"] == "pending"
    tap("Confirm")
    wait_for("Confirmed Results")
    wait_for("View source · page 1")
    rows = control("reports/get")["observations"]
    assert all(row["status"] == "confirmed" for row in rows)
    assert {row["observation_id"] for row in rows} == {row["observation_id"] for row in original}
    assert not {"Needs Review", "Hold", "Rejected"}.intersection(labels())
    if check_chart:
        tap("Browse")
        wait_for("mg/dL")
    print("PASS: Ready batch and detail confirmation, stale-batch refresh and failed-detail retry")


def report_settings():
    def control(path):
        return json.loads(subprocess.check_output(["docker", "exec", CONTAINER, "curl", "-fsS", "-X", "POST", "http://127.0.0.1:18765/api/v1/" + path], text=True))
    def ready():
        for _ in range(30):
            nodes = list(tree().iter("node"))
            if not any(n.get("text") == "Saving…" for n in nodes):
                return
            time.sleep(.2)
        raise AssertionError("Settings write did not finish")
    control("test/reset"); control("test/seed-report"); control("test/report-issue")
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Insights"); tap("Data"); tap("Blood Work"); wait_for("Example Laboratory"); tap("Example Laboratory")
    wait_for("Cannot appear in Browse: unit invalid-unit is not supported for this test.")
    assert "PAGE PROCESSING" not in labels()
    tap("Detail"); wait_for("PAGE PROCESSING"); wait_for("ORIGINAL OCR OUTPUT")
    adb("shell", "input", "keyevent", "KEYCODE_BACK")
    tap("Data"); adb("shell", "input", "keyevent", "KEYCODE_BACK"); wait_for("gearshape"); tap("gearshape"); wait_for("Sleep Goal")
    assert not {"Server processing", "App unlock", "Your server", "Full data export", "Clear this device"}.intersection(labels())
    tap("Sleep Goal"); wait_for("Set a sleep goal")
    switch = next(n for n in tree().iter("node") if n.get("checkable") == "true" and n.get("enabled") == "true")
    if switch.get("checked") != "true":
        tap_node(switch)
    wait_for("Hours"); ready(); tap("8 hr"); tap("7 hr"); ready()
    tap("0 min"); tap("5 min"); ready()
    assert control("wellness/preferences/get")["preferences"]["sleep_target_minutes"] == 425
    assert "Save" not in labels() and "Cancel" not in labels()
    control("test/fail-preferences-once")
    tap("5 min"); tap("6 min"); ready()
    assert any("Change was not saved" in label for label in labels())
    assert control("wellness/preferences/get")["preferences"]["sleep_target_minutes"] == 425
    wait_for("5 min")
    tap("chevron.left"); wait_for("7 hr 5 min")
    tap("Sleep Goal"); wait_for("5 min"); tap("chevron.left")
    assert control("wellness/preferences/get")["preferences"]["sleep_target_minutes"] == 425
    print("PASS: report diagnostics/Detail, Settings layout, automatic favorites and sleep goal persistence, failed-write rollback, Back/reopen")


def edge_scroll():
    """Headers leave the viewport; Browse search stays usable in both pane sizes."""
    def bounds(node):
        return tuple(map(int, re.findall(r"\d+", node.get("bounds"))))

    def matches(text):
        return [n for n in tree().iter("node") if text in (n.get("text"), n.get("content-desc"))]

    def search_box():
        return next(n for n in tree().iter("node") if n.get("class") == "android.widget.EditText")

    try:
        for wide in (False, True):
            adb("shell", "am", "force-stop", "dev.longlivehealth.android")
            adb("shell", "wm", "size", "2208x1840" if wide else "1080x2400")
            adb("shell", "wm", "density", "320" if wide else "420")
            adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
            wait_for("Insights")
            tap("Browse"); wait_for("Health Categories")
            original_search = bounds(search_box())
            header = next(n for n in matches("Browse") if bounds(n)[1] < original_search[1])
            assert bounds(header)[3] <= original_search[1]
            x = 300 if wide else 540
            for _ in range(3):
                adb("shell", "input", "swipe", str(x), "1400", str(x), "420", "500")
            moved_search = bounds(search_box())
            assert moved_search[1] < original_search[1] - 40, (original_search, moved_search)
            assert not any(bounds(n)[1] < moved_search[3] for n in matches("Browse")), "Browse title remained pinned"
            assert not matches("Health Categories"), "Category heading did not scroll away"
            assert moved_search[0] == original_search[0] and moved_search[2] == original_search[2], "Search width changed on scroll"
            # Return to the beginning, then enter a category without losing navigation.
            for _ in range(10):
                if matches("Health Categories"):
                    break
                # Begin below the restored search/header, never on the search field.
                adb("shell", "input", "swipe", str(x), "700", str(x), "1500", "500")
            wait_for("Health Categories"); tap("Heart"); wait_for("Heart rate")
            tap("Heart rate"); wait_for("RANGE")
            if wide:
                assert matches("Heart"), "Expanded sidebar disappeared"
                left_back = next(n for n in tree().iter("node")
                                 if (n.get("content-desc") in ("Back", "chevron.left"))
                                 and bounds(n)[0] < 600)
                tap_node(left_back); wait_for("Health Categories")
                detail_range = bounds(matches("RANGE")[0])
                for _ in range(3):
                    adb("shell", "input", "swipe", "300", "1400", "300", "420", "500")
                assert not matches("Health Categories"), "Expanded category heading remained pinned"
                assert bounds(matches("RANGE")[0]) == detail_range, "Left pane scrolled the right detail"
            tap("Data"); wait_for("Blood Work")
            tap("Browse"); wait_for("RANGE")
            print(f"PASS: {'expanded' if wide else 'folded'} scroll-away title, fixed search, category scroll, navigation and retained detail")
        # The same theme must apply outside Browse, not a per-page workaround.
        adb("shell", "wm", "size", "1080x2400")
        adb("shell", "wm", "density", "420")
        for title in ("Insights", "Browse", "Data"):
            tap(title)
            for _ in range(2):
                adb("shell", "input", "swipe", "540", "1400", "540", "450", "500")
            assert not any(bounds(n)[1] < 500 for n in matches(title)), f"{title} title remained pinned"
            # Restore actions/back buttons by scrolling down.
            adb("shell", "input", "swipe", "540", "450", "540", "1400", "500")
            assert any(bounds(n)[1] < 500 for n in matches(title)), f"{title} title did not return"
        print("PASS: all three tab navigation bars follow the same scroll policy")
    finally:
        adb("shell", "wm", "size", "reset")
        adb("shell", "wm", "density", "reset")


def paired_charts():
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "wm", "size", "reset")
    adb("shell", "wm", "density", "reset")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Insights"); tap("Browse"); wait_for("Health Categories")
    fields = [n for n in tree().iter("node") if n.get("class")=="android.widget.EditText"]
    tap_node(fields[0]); adb("shell","input","text","Blood%spressure")
    adb("shell","input","keyevent","KEYCODE_BACK")
    wait_for("Blood Pressure")
    assert not any((v or "").lower() in ("systolic blood pressure", "diastolic blood pressure") for v in labels())
    tap("Blood Pressure")
    wait_for("Systolic"); wait_for("Diastolic")
    assert adb("shell","pidof","dev.longlivehealth.android").strip()
    print("PASS: single blood pressure entry and paired component chart")
    for _ in range(6):
        if "Historical Values" in labels(): break
        adb("shell","input","swipe","520","1500","520","500","350")
    tap("Historical Values"); wait_for("120 / 80 mmHg")
    nodes = list(tree().iter("node"))
    value = next(n for n in nodes if n.get("text") == "120 / 80 mmHg")
    source = next(n for n in nodes if n.get("text") == "Apple Watch")
    date = next(n for n in nodes if re.fullmatch(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2} [AP]M", n.get("text", "")))
    centers = [(int(re.findall(r"\d+", n.get("bounds"))[1]) + int(re.findall(r"\d+", n.get("bounds"))[3])) / 2 for n in (date, source, value)]
    assert max(centers) - min(centers) < 15, centers
    print("PASS: history time, source and paired value share one row")
    tap("Back"); wait_for("Systolic")
    adb("shell", "input", "swipe", "520", "400", "520", "1600", "350")
    adb("shell", "input", "swipe", "520", "400", "520", "1600", "350")
    tap("Back")
    time.sleep(1)
    fields = [n for n in tree().iter("node") if n.get("class")=="android.widget.EditText"]
    tap_node(fields[0]); adb("shell","input","keycombination","KEYCODE_CTRL_LEFT","KEYCODE_A")
    adb("shell","input","keyevent","KEYCODE_DEL"); adb("shell","input","text","ECG")
    adb("shell","input","keyevent","KEYCODE_BACK")
    matches = [n for n in tree().iter("node") if n.get("text")=="ECG" and n.get("class")=="android.widget.TextView"]
    tap_node(matches[0]); wait_for("ECG voltage waveform, seconds and millivolts")
    assert "ECG voltage waveform, seconds and millivolts" in labels()
    assert adb("shell","pidof","dev.longlivehealth.android").strip()
    choices = [n for n in tree().iter("node") if n.get("text") and re.search(r"20\d\d", n.get("text")) and n.get("clickable")=="true"]
    current = choices[0].get("text"); tap_node(choices[0])
    choices = [n for n in tree().iter("node") if n.get("text") and re.search(r"20\d\d", n.get("text")) and n.get("clickable")=="true" and n.get("text") != current]
    assert choices, "ECG selector did not expose the other recording"
    replacement = choices[0].get("text"); tap_node(choices[0])
    wait_for(replacement); wait_for("ECG voltage waveform, seconds and millivolts")
    print("PASS: dedicated ECG waveform and switching recordings")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=("inspect", "login", "tap", "navigation", "insight-cards", "browse-switch", "camera", "selectors", "chart-scroll", "original-preview", "rescan", "blood-work", "report-date", "report-scroll", "report-metric-review", "report-mapping-layout", "report-status-actions", "report-status-chart", "report-settings", "edge-scroll", "paired-charts"))
    parser.add_argument("text", nargs="?")
    args = parser.parse_args()
    if args.action == "inspect":
        inspect()
    elif args.action == "paired-charts":
        paired_charts()
    elif args.action == "edge-scroll":
        edge_scroll()
    elif args.action == "login":
        login()
    elif args.action == "insight-cards":
        insight_cards()
    elif args.action == "browse-switch":
        browse_switch()
    elif args.action == "chart-scroll":
        chart_scroll()
    elif args.action == "selectors":
        selectors()
    elif args.action == "report-date":
        report_date()
    elif args.action == "report-scroll":
        report_scroll()
    elif args.action == "report-metric-review":
        report_metric_review()
    elif args.action == "report-mapping-layout":
        report_mapping_layout()
    elif args.action in ("report-status-actions", "report-status-chart"):
        report_status_actions(check_chart=args.action == "report-status-chart")
    elif args.action == "report-settings":
        report_settings()
    elif args.action == "blood-work":
        blood_work_categories()
    elif args.action == "rescan":
        rescan_confirmation()
    elif args.action == "original-preview":
        original_preview()
    elif args.action == "camera":
        camera_import()
    elif args.action == "navigation":
        navigation()
    else:
        tap(args.text)
