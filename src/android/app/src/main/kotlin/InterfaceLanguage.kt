package health.app

import android.content.Context
import android.content.res.Resources
import org.json.JSONObject

internal object InterfaceLanguage {
    fun system(): String = Resources.getSystem().configuration.locales[0].toLanguageTag()

    fun apply(context: Context, value: String) {
        preferences(context).edit().putString("selection", value).apply()
    }

    fun text(context: Context, english: String, values: Array<out Any>): String {
        val selection = preferences(context).getString("selection", null)
        val chinese = selection == "zh-Hans" || (selection == null && system().substringBefore('-').equals("zh", true))
        val template = if (chinese) catalog(context).optString(english, english) else english
        return slots.replace(template) { match ->
            values.getOrNull(match.groupValues[1].toIntOrNull() ?: -1)?.toString() ?: match.value
        }
    }

    private fun preferences(context: Context) = context.getSharedPreferences("interface", Context.MODE_PRIVATE)
    private val slots = Regex("\\{([0-9]+)\\}")
    private var translations: JSONObject? = null

    @Synchronized
    private fun catalog(context: Context): JSONObject {
        translations?.let { return it }
        return context.assets.open("InterfaceChinese.json").bufferedReader().use { JSONObject(it.readText()) }
            .also { translations = it }
    }
}

internal fun Context.interfaceText(english: String, vararg values: Any): String =
    InterfaceLanguage.text(this, english, values)
