package health.app

import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import androidx.core.content.FileProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.UUID

object DocumentServices {
    fun copyDocument(uri: Uri, fallbackType: String): JSONObject {
        val context = NativeServices.context
        val resolver = context.contentResolver
        val type = resolver.getType(uri) ?: fallbackType
        var name = "Report"
        resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
            if (it.moveToFirst()) name = it.getString(0)
        }
        val directory = File(context.cacheDir, "reports").apply { mkdirs() }
        val file = File(directory, UUID.randomUUID().toString() + "." + name.substringAfterLast('.', "data").filter { it.isLetterOrDigit() }.take(10))
        try {
            resolver.openInputStream(uri)?.use { source -> file.outputStream().use { source.copyTo(it) } }
                ?: error("Could not read the selected document")
        } catch (error: Exception) { file.delete(); throw error }
        return JSONObject().put("path", file.absolutePath).put("name", name).put("type", type)
    }

    suspend fun perform(operation: String, input: JSONObject): JSONObject {
        val activity = NativeServices.activity.get() ?: error("Open the app to choose a document")
        return when (operation) {
            "document.import" -> {
                val kind = input.getString("kind")
                val uris = withContext(Dispatchers.Main) { activity.chooseDocuments(kind) }
                val files = uris.map { uri ->
                    copyDocument(uri, if (kind == "pdf") "application/pdf" else "application/octet-stream")
                }
                JSONObject().put("files", JSONArray(files))
            }
            "document.open" -> {
                val source = File(input.getString("path"))
                require(source.isFile) { "Original report is unavailable" }
                withContext(Dispatchers.Main) {
                    activity.startActivity(Intent(activity, ReportPreviewActivity::class.java)
                        .putExtra("path", source.absolutePath).putExtra("page", input.getInt("page")))
                }
                JSONObject()
            }
            else -> error("Unknown document action")
        }
    }
}
