package health.app

import android.app.Activity
import android.content.Intent
import android.graphics.BitmapFactory
import android.graphics.pdf.PdfDocument
import android.os.Bundle
import android.widget.*
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.FileProvider
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.*
import java.io.File
import java.util.UUID

/** A camera session becomes one ordered PDF, with individual page review and deletion. */
class ReportCaptureActivity : AppCompatActivity() {
    private val pages = mutableListOf<File>()
    private var pending: File? = null
    private var index = 0
    private lateinit var image: ImageView
    private lateinit var label: TextView
    private lateinit var slider: SeekBar
    private lateinit var controls: LinearLayout
    private var preview: Job? = null
    private val camera = registerForActivityResult(ActivityResultContracts.TakePicture()) { saved ->
        pending?.let { if (saved) { pages.add(it); index = pages.lastIndex } else it.delete() }
        pending = null; render()
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        savedInstanceState?.getStringArrayList("pages")?.mapTo(pages) { File(it) }
        index = savedInstanceState?.getInt("index") ?: 0
        pending = savedInstanceState?.getString("pending")?.let { File(it) }
        val layout = ReportPageLayout(this, "Camera report")
        label = layout.label
        image = layout.image
        slider = layout.slider.apply { setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onStartTrackingTouch(seek: SeekBar?) {}
            override fun onStopTrackingTouch(seek: SeekBar?) {}
            override fun onProgressChanged(seek: SeekBar?, value: Int, user: Boolean) { if (user) { index = value; render() } }
        }) }
        controls = layout.controls
        fun button(title: String, action: () -> Unit) { layout.action(title, action) }
        button("Camera") { capture() }
        button("Delete") { if (pages.isNotEmpty()) { pages.removeAt(index).delete(); index = index.coerceAtMost(pages.lastIndex).coerceAtLeast(0); render() } }
        button("Import") { if (pages.isNotEmpty()) export() }
        render()
    }
    override fun onSaveInstanceState(state: Bundle) {
        state.putStringArrayList("pages", ArrayList(pages.map { it.absolutePath }))
        state.putInt("index", index); state.putString("pending", pending?.absolutePath)
        super.onSaveInstanceState(state)
    }
    private fun capture() {
        val directory = File(cacheDir, "reports").apply { mkdirs() }
        val file = File(directory, "camera-${UUID.randomUUID()}.jpg"); pending = file
        camera.launch(FileProvider.getUriForFile(this, "$packageName.documents", file))
    }
    private fun render() {
        preview?.cancel()
        label.text = if (pages.isEmpty()) "Take report photos" else "Page ${index + 1} of ${pages.size}"
        controls.getChildAt(1).isEnabled = pages.isNotEmpty()
        controls.getChildAt(2).isEnabled = pages.isNotEmpty()
        slider.isEnabled = pages.size > 1
        slider.max = (pages.size - 1).coerceAtLeast(0); slider.progress = index
        val previous = (image.drawable as? android.graphics.drawable.BitmapDrawable)?.bitmap
        image.setImageDrawable(null)
        previous?.recycle()
        val file = pages.getOrNull(index) ?: return
        preview = lifecycleScope.launch {
            try {
                val bitmap = withContext(Dispatchers.IO) { decode(file) }
                if (isActive) image.setImageBitmap(bitmap) else bitmap.recycle()
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { label.text = error.message ?: "Could not preview photo" }
        }
    }
    private fun decode(file: File): android.graphics.Bitmap {
        val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(file.path, options)
        options.inSampleSize = 1
        while (maxOf(options.outWidth, options.outHeight) / options.inSampleSize > 2200) options.inSampleSize *= 2
        options.inJustDecodeBounds = false
        val bitmap = BitmapFactory.decodeFile(file.path, options) ?: error("Could not decode report photo")
        val orientation = androidx.exifinterface.media.ExifInterface(file.path).getAttributeInt(androidx.exifinterface.media.ExifInterface.TAG_ORIENTATION, 1)
        val matrix = android.graphics.Matrix()
        when (orientation) {
            2 -> matrix.setScale(-1f, 1f)
            3 -> matrix.setRotate(180f)
            4 -> matrix.setScale(1f, -1f)
            5 -> { matrix.setRotate(90f); matrix.postScale(-1f, 1f) }
            6 -> matrix.setRotate(90f)
            7 -> { matrix.setRotate(-90f); matrix.postScale(-1f, 1f) }
            8 -> matrix.setRotate(-90f)
        }
        if (matrix.isIdentity) return bitmap
        return android.graphics.Bitmap.createBitmap(bitmap, 0, 0, bitmap.width, bitmap.height, matrix, true).also {
            if (it !== bitmap) bitmap.recycle()
        }
    }
    private fun export() {
        for (i in 0 until controls.childCount) controls.getChildAt(i).isEnabled = false
        lifecycleScope.launch {
            try {
                val file = withContext(Dispatchers.IO) {
                    val output = File(cacheDir, "reports/Camera report-${UUID.randomUUID()}.pdf")
                    val document = PdfDocument()
                    try {
                        pages.forEachIndexed { index, file ->
                            val bitmap = decode(file)
                            val page = document.startPage(PdfDocument.PageInfo.Builder(bitmap.width, bitmap.height, index + 1).create())
                            page.canvas.drawBitmap(bitmap, 0f, 0f, null); document.finishPage(page); bitmap.recycle()
                        }
                        output.outputStream().use { document.writeTo(it) }
                    } finally { document.close() }
                    output
                }
                setResult(Activity.RESULT_OK, Intent().setData(FileProvider.getUriForFile(this@ReportCaptureActivity, "$packageName.documents", file)).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION))
                pages.forEach { it.delete() }; finish()
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) {
                label.text = error.message ?: "Could not create report"
                for (i in 0 until controls.childCount) controls.getChildAt(i).isEnabled = true
            }
        }
    }
}
