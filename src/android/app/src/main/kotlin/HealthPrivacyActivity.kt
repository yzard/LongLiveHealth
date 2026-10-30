package health.app

/** Available before consent and without a server account. */
class HealthPrivacyActivity : androidx.appcompat.app.AppCompatActivity() {
    override fun onCreate(state: android.os.Bundle?) {
        super.onCreate(state)
        val content = android.widget.LinearLayout(this).apply {
            orientation = android.widget.LinearLayout.VERTICAL
            val padding = (24 * resources.displayMetrics.density).toInt()
            setPadding(padding, padding, padding, padding)
            addView(android.widget.TextView(context).apply { text = "Health Connect access"; textSize = 24f })
            addView(android.widget.TextView(context).apply {
                textSize = 17f
                text = "Long Live Health reads the health data you allow and uploads it to the server you choose, under your account. This data provides your health history, charts and summaries. Background and history permissions allow ongoing updates and older measurements. You can pause sync in the app and revoke access in Health Connect. Revoking access does not delete records already uploaded to your server. Writing a reviewed glucose result requires a separate action and permission."
            })
            addView(android.widget.Button(context).apply { text = "Done"; setOnClickListener { finish() } })
        }
        setContentView(android.widget.ScrollView(this).apply { addView(content) })
    }
}
