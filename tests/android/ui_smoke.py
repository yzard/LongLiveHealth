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
    adb("shell", "uiautomator", "dump", "/sdcard/window.xml")
    return ET.fromstring(adb("shell", "cat", "/sdcard/window.xml"))


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
        if "Overview" in labels:
            assert "Could not complete the action" not in labels
            print("PASS: Login and Overview")
            return
        time.sleep(0.5)
    inspect()
    raise AssertionError("Login did not open Overview")


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
    wait_for("Overview")
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
    for _ in range(4):
        adb("shell", "input", "swipe", "1450", "1500", "1450", "450", "350")
    assert "Marker 1" not in labels(), "Report list did not scroll"
    # Switching away and back must keep the report, not reset to the Data menu.
    tap("Browse"); wait_for("RANGE")
    tap("Data")
    assert "fixture.png" in labels(), "Tab return lost the report"
    for _ in range(5):
        adb("shell", "input", "swipe", "1450", "450", "1450", "1500", "350")
    wait_for("Confirm All"); tap("Confirm All"); wait_for("0 pending")
    tap("Report Info"); wait_for("Report Info")
    assert "Report Title" in labels(), "Report editor missing title"
    print("PASS: session persistence, chart periods/source toggles, fold state, report scroll/tab state/Confirm All/editor")


def selectors():
    # Mixed static/dynamic Picker content previously crashed the native bridge
    # before the user could select anything, including with an empty catalog.
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Overview")
    tap("Trends"); wait_for("Recorded trends")
    assert "Compare measurements" not in labels()
    tap("Insights"); tap("Behavior associations"); wait_for("DECLARE AN ANALYSIS")
    assert "Run declared analysis" in labels()
    tap("Back")
    print("PASS: Trends and association selectors render without native crash")


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
        wait_for("Overview")
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


def chart_scroll():
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "wm", "size", "reset")
    adb("shell", "wm", "density", "reset")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Overview"); tap("Browse"); wait_for("Health Categories")
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
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Overview")
    tap("Data"); tap("Blood Work"); wait_for("Example Laboratory"); tap("Example Laboratory")
    tap("View original report")
    wait_for("Original report")
    time.sleep(1)
    assert not any("Could not" in label for label in labels()), "Original image did not render"
    assert adb("shell", "pidof", "dev.longlivehealth.android").strip()
    adb("shell", "input", "keyevent", "KEYCODE_BACK")
    wait_for("Open original report")
    tap("Done")
    wait_for("Report Info")
    print("PASS: original document downloads, opens, and returns to report without crashing")


def rescan_confirmation():
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Overview")
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
    tap("Cancel"); wait_for("Report Info")
    assert adb("shell", "pidof", "dev.longlivehealth.android").strip()
    print("PASS: four report actions in order on one row, Rescan confirmation and cancel")


def blood_work_categories():
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Overview")
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
    wait_for("Overview"); tap("Data"); tap("Blood Work"); wait_for("Example Laboratory"); tap("Example Laboratory")
    wait_for("Cannot appear in Browse: unit invalid-unit is not supported for this test.")
    assert "PAGE PROCESSING" not in labels()
    tap("Detail"); wait_for("PAGE PROCESSING"); wait_for("ORIGINAL OCR OUTPUT")
    adb("shell", "input", "keyevent", "KEYCODE_BACK")
    tap("Overview"); tap("Customize"); wait_for("FAVORITE METRICS")
    assert "Your daily sleep target" not in labels()
    assert "Save preferences" not in labels()
    switch = next(n for n in tree().iter("node") if n.get("checkable") == "true" and n.get("enabled") == "true")
    tap_node(switch); ready()
    assert control("wellness/preferences/get")["preferences"]["favorite_metrics"] == ["sleep"]
    tap("chevron.left"); wait_for("gearshape"); tap("gearshape"); wait_for("Sleep Goal")
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
            wait_for("Overview")
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
        for title in ("Overview", "Trends", "Insights", "Data"):
            tap(title)
            for _ in range(2):
                adb("shell", "input", "swipe", "540", "1400", "540", "450", "500")
            assert not any(bounds(n)[1] < 500 for n in matches(title)), f"{title} title remained pinned"
            # Restore actions/back buttons by scrolling down.
            adb("shell", "input", "swipe", "540", "450", "540", "1400", "500")
            assert any(bounds(n)[1] < 500 for n in matches(title)), f"{title} title did not return"
        print("PASS: all five tab navigation bars follow the same scroll policy")
    finally:
        adb("shell", "wm", "size", "reset")
        adb("shell", "wm", "density", "reset")


def paired_charts():
    adb("shell", "am", "force-stop", "dev.longlivehealth.android")
    adb("shell", "wm", "size", "reset")
    adb("shell", "wm", "density", "reset")
    adb("shell", "am", "start", "-n", "dev.longlivehealth.android/health.app.MainActivity")
    wait_for("Overview"); tap("Browse"); wait_for("Health Categories")
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
    parser.add_argument("action", choices=("inspect", "login", "tap", "navigation", "browse-switch", "camera", "selectors", "chart-scroll", "original-preview", "rescan", "blood-work", "report-settings", "edge-scroll", "paired-charts"))
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
    elif args.action == "browse-switch":
        browse_switch()
    elif args.action == "chart-scroll":
        chart_scroll()
    elif args.action == "selectors":
        selectors()
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
