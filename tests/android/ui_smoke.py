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
    tap("Browse"); wait_for("Heart rate"); tap("Heart rate"); wait_for("RANGE")
    assert labels().count("Heart rate") == 1, "Duplicate chart heading"
    tap("Y"); wait_for("RANGE")
    tap("D"); wait_for("Apple Watch")
    tap("Apple Watch"); wait_for("No sources selected")
    tap("Apple Watch"); wait_for("RANGE")
    adb("shell", "wm", "size", "2208x1840")
    adb("shell", "wm", "density", "320")
    wait_for("Search metrics")
    assert "RANGE" in labels(), "Fold transition lost the detail"
    tap("Data"); wait_for("Health Report")
    tap("Health Report"); wait_for("fixture.png")
    tap("fixture.png"); wait_for("Total Markers")
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


def fixture_state():
    return json.loads(subprocess.check_output(["docker", "exec", CONTAINER, "curl", "-fsS", "-X", "POST", "http://127.0.0.1:18765/api/v1/test/state"], text=True))


def camera_import():
    before = len(fixture_state()["upload_ids"])
    tap("Data"); tap("Health Report"); tap("square.and.arrow.up"); tap("Camera")
    wait_for("Take report photos")
    for page in (1, 2):
        tap("CAMERA"); wait_for("Shutter"); tap("Shutter"); wait_for("Done"); tap("Done")
        wait_for(f"Page {page} of {page}")
    tap("DELETE"); wait_for("Page 1 of 1"); tap("IMPORT")
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        current = labels()
        assert "Could not complete the action" not in current, current
        if len(fixture_state()["upload_ids"]) == before + 1 and "Retry uploads" not in current:
            print("PASS: two camera pages, delete one, one PDF automatically uploaded without Retry")
            return
        time.sleep(0.25)
    raise AssertionError("Camera PDF did not upload automatically")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=("inspect", "login", "tap", "navigation", "camera"))
    parser.add_argument("text", nargs="?")
    args = parser.parse_args()
    if args.action == "inspect":
        inspect()
    elif args.action == "login":
        login()
    elif args.action == "camera":
        camera_import()
    elif args.action == "navigation":
        navigation()
    else:
        tap(args.text)
