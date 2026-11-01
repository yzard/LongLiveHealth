package health.app

import android.content.res.ColorStateList
import android.content.res.Configuration
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat

/** Shared, inset-aware presentation for camera pages and original documents. */
internal class ReportPageLayout(private val activity: AppCompatActivity, title: String) {
    private val dark = activity.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
    private val textColor = if (dark) Color.WHITE else Color.rgb(28, 28, 30)
    private val accent = if (dark) Color.rgb(100, 210, 201) else Color.rgb(0, 127, 121)
    private val surface = if (dark) Color.rgb(28, 28, 30) else Color.WHITE
    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density).toInt()
    val root = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setBackgroundColor(if (dark) Color.BLACK else Color.rgb(242, 242, 247))
    }
    val label = TextView(activity).apply {
        textSize = 15f; setTextColor(textColor); gravity = Gravity.CENTER
        setPadding(dp(16), dp(12), dp(16), dp(12))
    }
    val image = ImageView(activity).apply { scaleType = ImageView.ScaleType.FIT_CENTER }
    val slider = SeekBar(activity).apply {
        progressTintList = ColorStateList.valueOf(accent)
        thumbTintList = ColorStateList.valueOf(accent)
        contentDescription = "Report page"
    }
    val controls = LinearLayout(activity).apply { orientation = LinearLayout.HORIZONTAL }
    init {
        val header = FrameLayout(activity)
        header.addView(TextView(activity).apply {
            text = title; textSize = 24f; setTypeface(typeface, Typeface.BOLD)
            setTextColor(textColor); gravity = Gravity.CENTER
            setPadding(dp(60), 0, dp(60), 0)
        }, FrameLayout.LayoutParams(-1, dp(60)))
        header.addView(button("‹") { activity.finish() }.apply {
            textSize = 32f; contentDescription = "Back"
        }, FrameLayout.LayoutParams(dp(48), dp(48), Gravity.START or Gravity.CENTER_VERTICAL))
        root.addView(header)
        root.addView(label)
        root.addView(image, LinearLayout.LayoutParams(-1, 0, 1f))
        root.addView(slider, LinearLayout.LayoutParams(-1, dp(48)))
        root.addView(controls)
        ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets ->
            val safe = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
            view.setPadding(safe.left + dp(16), safe.top, safe.right + dp(16), safe.bottom + dp(12))
            insets
        }
        activity.setContentView(root)
        WindowInsetsControllerCompat(activity.window, root).apply {
            isAppearanceLightStatusBars = !dark; isAppearanceLightNavigationBars = !dark
        }
        ViewCompat.requestApplyInsets(root)
    }
    private fun button(title: String, action: () -> Unit) = Button(activity).apply {
        text = title; isAllCaps = false; textSize = 17f; setTextColor(accent)
        minimumWidth = 0; minimumHeight = dp(48)
        background = GradientDrawable().apply { setColor(surface); cornerRadius = dp(14).toFloat() }
        setOnClickListener { action() }
    }
    fun action(title: String, callback: () -> Unit): Button {
        val button = button(title, callback)
        controls.addView(button, LinearLayout.LayoutParams(0, dp(48), 1f).apply {
            marginStart = dp(4); marginEnd = dp(4)
        })
        return button
    }
}
