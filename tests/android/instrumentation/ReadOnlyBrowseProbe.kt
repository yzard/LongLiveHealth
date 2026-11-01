package health.app

import android.graphics.Rect
import android.os.SystemClock
import android.view.InputDevice
import android.view.MotionEvent
import android.os.Bundle
import android.os.Build
import android.view.accessibility.AccessibilityNodeInfo
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.net.InetAddress
import java.net.ServerSocket

/** Opt-in, loopback-only UI inspection during an operator-directed device audit. Not in the APK. */
@RunWith(AndroidJUnit4::class)
class ReadOnlyBrowseProbe {
    @Test fun inspectBrowsePages() {
        assumeTrue(InstrumentationRegistry.getArguments().getString("browseProbe") == "true")
        val automation = InstrumentationRegistry.getInstrumentation().uiAutomation
        fun nodes(): List<AccessibilityNodeInfo> {
            if (Build.VERSION.SDK_INT >= 34) automation.clearCache()
            val result = mutableListOf<AccessibilityNodeInfo>()
            fun visit(node: AccessibilityNodeInfo?) {
                if (node == null) return
                if (!node.refresh()) { node.recycle(); return }
                result.add(node)
                for (i in 0 until node.childCount) visit(node.getChild(i))
            }
            val root = automation.rootInActiveWindow
            if (root?.packageName?.toString() == "dev.longlivehealth.android") visit(root)
            else root?.recycle()
            return result
        }
        ServerSocket(38766, 1, InetAddress.getByName("127.0.0.1")).use { server ->
            server.soTimeout = 30 * 60 * 1000
            var running = true
            while (running) {
                server.accept().use { socket ->
                    socket.soTimeout = 10000
                    val request = JSONObject(socket.getInputStream().bufferedReader().readLine())
                    val response = JSONObject()
                    val all = nodes()
                    try {
                        when (request.getString("action")) {
                            "pinch" -> {
                                val x = request.getDouble("x").toFloat()
                                val y = request.getDouble("y").toFloat()
                                val from = request.getDouble("from").toFloat()
                                val to = request.getDouble("to").toFloat()
                                require(from > 0 && to > 0)
                                val start = SystemClock.uptimeMillis()
                                val properties = Array(2) { index -> MotionEvent.PointerProperties().apply {
                                    id = index; toolType = MotionEvent.TOOL_TYPE_FINGER
                                } }
                                fun send(action: Int, count: Int, distance: Float) {
                                    val coords = Array(count) { index -> MotionEvent.PointerCoords().apply {
                                        this.x = x + (if (index == 0) -distance else distance) / 2
                                        this.y = y; pressure = 1f; size = 1f
                                    } }
                                    val event = MotionEvent.obtain(start, SystemClock.uptimeMillis(), action, count,
                                        properties, coords, 0, 0, 1f, 1f, 0, 0, InputDevice.SOURCE_TOUCHSCREEN, 0)
                                    try { check(automation.injectInputEvent(event, true)) } finally { event.recycle() }
                                }
                                send(MotionEvent.ACTION_DOWN, 1, from)
                                send(MotionEvent.ACTION_POINTER_DOWN or (1 shl MotionEvent.ACTION_POINTER_INDEX_SHIFT), 2, from)
                                for (step in 1..24) {
                                    SystemClock.sleep(20)
                                    send(MotionEvent.ACTION_MOVE, 2, from + (to-from)*step/24)
                                }
                                send(MotionEvent.ACTION_POINTER_UP or (1 shl MotionEvent.ACTION_POINTER_INDEX_SHIFT), 2, to)
                                send(MotionEvent.ACTION_UP, 1, to)
                                response.put("ok", true)
                            }
                            "dump" -> {
                                val values = JSONArray()
                                all.forEach { node ->
                                    val bounds = Rect(); node.getBoundsInScreen(bounds)
                                    values.put(JSONObject().put("text", node.text?.toString() ?: "")
                                        .put("desc", node.contentDescription?.toString() ?: "")
                                        .put("class", node.className?.toString() ?: "")
                                        .put("bounds", JSONArray(listOf(bounds.left,bounds.top,bounds.right,bounds.bottom))))
                                }
                                response.put("nodes", values)
                            }
                            "click" -> {
                                val label = request.getString("text")
                                val after = request.optString("after", "")
                                val start = if (after.isEmpty()) 0 else all.indexOfFirst { it.text?.toString() == after || it.contentDescription?.toString() == after } + 1
                                var target = if (after.isNotEmpty() && start == 0) null else all.drop(start).firstOrNull { !it.isEditable && (it.text?.toString() == label || it.contentDescription?.toString() == label) }
                                while (target != null && !target.isClickable) target = target.parent
                                response.put("ok", target?.performAction(AccessibilityNodeInfo.ACTION_CLICK) ?: false)
                            }
                            "search" -> {
                                val editor = all.firstOrNull { it.isEditable }
                                val args = Bundle(); args.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, request.getString("text"))
                                response.put("ok", editor?.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args) ?: false)
                            }
                            "stop" -> { running = false; response.put("ok", true) }
                            else -> response.put("error", "Unsupported inspection command")
                        }
                    } finally { all.forEach { it.recycle() } }
                    socket.getOutputStream().bufferedWriter().use { it.write(response.toString()); it.newLine(); it.flush() }
                }
            }
        }
    }
}
