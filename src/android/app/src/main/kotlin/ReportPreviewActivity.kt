package health.app

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.pdf.PdfRenderer
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.*
import java.io.File

/** Bounded-resolution rendering inside the app; no external PDF reader is required. */
class ReportPreviewActivity : AppCompatActivity() {
    private lateinit var image: ImageView
    private lateinit var label: TextView
    private lateinit var slider: SeekBar
    private var renderer: PdfRenderer? = null
    private var descriptor: ParcelFileDescriptor? = null
    private var rendering: Job? = null
    private var bitmap: Bitmap? = null
    private val renderLock = Any()
    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        val layout = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        layout.addView(Button(this).apply { text = "Done"; setOnClickListener { finish() } })
        label = TextView(this).apply { textSize = 18f }; layout.addView(label)
        image = ImageView(this).apply { scaleType = ImageView.ScaleType.FIT_CENTER }
        layout.addView(image, LinearLayout.LayoutParams(-1, 0, 1f))
        slider = SeekBar(this); layout.addView(slider)
        setContentView(layout)
        try {
            val file = File(requireNotNull(intent.getStringExtra("path")))
            val header = ByteArray(5); file.inputStream().use { it.read(header) }
            if (String(header) == "%PDF-") {
                descriptor = ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
                renderer = PdfRenderer(descriptor!!)
                slider.max = (renderer!!.pageCount - 1).coerceAtLeast(0)
                slider.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
                    override fun onStartTrackingTouch(view: SeekBar?) {}
                    override fun onStopTrackingTouch(view: SeekBar?) {}
                    override fun onProgressChanged(view: SeekBar?, value: Int, user: Boolean) { if (user) render(value) }
                })
                val page = (intent.getIntExtra("page", 1) - 1).coerceIn(0, slider.max)
                slider.progress = page; render(page)
            } else {
                slider.visibility = android.view.View.GONE
                lifecycleScope.launch {
                    try {
                        val decoded = withContext(Dispatchers.IO) {
                            val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                            BitmapFactory.decodeFile(file.path, options)
                            options.inSampleSize = 1
                            while (maxOf(options.outWidth, options.outHeight) / options.inSampleSize > 2200) options.inSampleSize *= 2
                            options.inJustDecodeBounds = false
                            BitmapFactory.decodeFile(file.path, options) ?: error("Could not open report image")
                        }
                        bitmap = decoded; image.setImageBitmap(decoded); label.text = "Original report"
                    } catch (cancelled: CancellationException) { throw cancelled }
                    catch (error: Exception) { label.text = error.message }
                }
            }
        } catch (error: Exception) { label.text = error.message ?: "Could not open original report" }
    }
    private fun render(index: Int) {
        rendering?.cancel()
        rendering = lifecycleScope.launch {
            var next: Bitmap? = null
            try {
                withContext(Dispatchers.IO) {
                    synchronized(renderLock) {
                        renderer?.openPage(index)?.use { page ->
                            val scale = minOf(2f, 1600f / maxOf(page.width, page.height))
                            val output = Bitmap.createBitmap(maxOf(1, (page.width * scale).toInt()), maxOf(1, (page.height * scale).toInt()), Bitmap.Config.ARGB_8888)
                            next = output
                            output.eraseColor(android.graphics.Color.WHITE)
                            page.render(output, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
                        }
                    }
                }
                ensureActive()
                val old = bitmap
                bitmap = next; next = null; image.setImageBitmap(bitmap); old?.recycle()
                label.text = "Page ${index + 1} of ${slider.max + 1}"
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { label.text = error.message ?: "Could not render page" }
            finally { next?.recycle() }
        }
    }
    override fun onDestroy() {
        rendering?.cancel()
        synchronized(renderLock) { renderer?.close(); renderer = null; descriptor?.close(); descriptor = null }
        image.setImageDrawable(null); bitmap?.recycle(); bitmap = null
        super.onDestroy()
    }
}
