package health.app

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.pdf.PdfRenderer
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.view.View
import android.view.ViewGroup
import android.widget.AbsListView
import android.widget.BaseAdapter
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.ListView
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.AppCompatImageView
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import kotlin.math.roundToInt

class ReportPreviewActivity : AppCompatActivity() {
    private lateinit var layout: ReportPageLayout
    private var pages: ListView? = null
    private var renderer: PdfRenderer? = null
    private val renderLock = Any()

    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        enableEdgeToEdge()
        layout = ReportPageLayout(this, interfaceText("Original report"), cameraControls = false)
        try {
            val file = File(requireNotNull(intent.getStringExtra("path")))
            val signature = ByteArray(5)
            file.inputStream().use { it.read(signature) }
            if (String(signature, Charsets.US_ASCII) == "%PDF-") {
                showPdf(file, state)
            } else {
                showImage(file)
            }
        } catch (error: Exception) {
            layout.label.text = interfaceText(error.message ?: "Could not open original report")
        }
    }

    private fun showPdf(file: File, state: Bundle?) {
        val descriptor = ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
        val pdf = try {
            PdfRenderer(descriptor)
        } catch (error: Exception) {
            descriptor.close()
            throw error
        }
        renderer = pdf
        val count = pdf.pageCount
        require(count > 0) { "This report has no pages" }
        val requested = (intent.getIntExtra("page", 1) - 1).coerceIn(0, count - 1)
        val first = (state?.getInt("firstPage") ?: requested).coerceIn(0, count - 1)
        val initialRatio = pdf.openPage(first).use { it.height.toFloat() / it.width }
        layout.label.text = interfaceText("Page {0} of {1}", first + 1, count)

        val list = ListView(this).apply {
            divider = ColorDrawable(Color.TRANSPARENT)
            dividerHeight = (8 * resources.displayMetrics.density).roundToInt()
            adapter = object : BaseAdapter() {
                override fun getCount() = count
                override fun getItem(position: Int) = position
                override fun getItemId(position: Int) = position.toLong()
                override fun hasStableIds() = true

                override fun getView(position: Int, convertView: View?, parent: ViewGroup): View {
                    return ((convertView as? PdfPageView) ?: PdfPageView(initialRatio)).apply { bind(position) }
                }
            }
        }
        layout.content.addView(list, FrameLayout.LayoutParams(-1, -1))
        pages = list
        list.setSelectionFromTop(first, state?.getInt("pageOffset") ?: 0)
        list.setOnScrollListener(object : AbsListView.OnScrollListener {
            override fun onScrollStateChanged(view: AbsListView, state: Int) = Unit

            override fun onScroll(view: AbsListView, firstVisible: Int, visibleCount: Int, total: Int) {
                if (visibleCount == 0 || total == 0) return
                // When the final page is shorter than the viewport, it cannot reach the top.
                val index = if (firstVisible > 0 && view.lastVisiblePosition == total - 1 &&
                    !view.canScrollVertically(1)) total - 1 else firstVisible
                val title = interfaceText("Page {0} of {1}", index + 1, total)
                if (layout.label.text != title) layout.label.text = title
            }
        })
    }

    private fun showImage(file: File) {
        layout.content.addView(layout.image, FrameLayout.LayoutParams(-1, -1))
        lifecycleScope.launch {
            try {
                val bitmap = withContext(Dispatchers.IO) {
                    val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                    BitmapFactory.decodeFile(file.path, options)
                    while (maxOf(options.outWidth, options.outHeight) / options.inSampleSize.coerceAtLeast(1) > 2200) {
                        options.inSampleSize = options.inSampleSize.coerceAtLeast(1) * 2
                    }
                    options.inJustDecodeBounds = false
                    BitmapFactory.decodeFile(file.path, options) ?: error("Could not open report image")
                }
                layout.image.setImageBitmap(bitmap)
                layout.image.contentDescription = interfaceText("Original report image")
            } catch (error: CancellationException) {
                throw error
            } catch (error: Exception) {
                layout.label.text = interfaceText(error.message ?: "Could not open report image")
            }
        }
    }

    override fun onSaveInstanceState(state: Bundle) {
        pages?.let {
            state.putInt("firstPage", it.firstVisiblePosition)
            state.putInt("pageOffset", it.getChildAt(0)?.top ?: 0)
        }
        super.onSaveInstanceState(state)
    }

    override fun onDestroy() {
        pages?.adapter = null
        layout.image.setImageDrawable(null)
        synchronized(renderLock) {
            renderer?.close()
            renderer = null
        }
        super.onDestroy()
    }

    private inner class PdfPageView(private val initialRatio: Float) : AppCompatImageView(this@ReportPreviewActivity) {
        private var index = -1
        private var ratio = initialRatio
        private var rendering: Job? = null

        init {
            scaleType = ScaleType.FIT_CENTER
            setBackgroundColor(Color.WHITE)
            layoutParams = AbsListView.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT)
        }

        fun bind(position: Int) {
            if (index == position) return
            rendering?.cancel()
            index = position
            ratio = initialRatio
            setImageDrawable(null)
            contentDescription = null
            requestLayout()
            if (isAttachedToWindow) render()
        }

        override fun onMeasure(widthSpec: Int, heightSpec: Int) {
            val width = MeasureSpec.getSize(widthSpec)
            setMeasuredDimension(width, maxOf(1, (width * ratio).roundToInt()))
        }

        override fun onAttachedToWindow() {
            super.onAttachedToWindow()
            if (index >= 0) render()
        }

        override fun onDetachedFromWindow() {
            rendering?.cancel()
            setImageDrawable(null)
            contentDescription = null
            super.onDetachedFromWindow()
        }

        private fun render() {
            val position = index
            rendering?.cancel()
            rendering = lifecycleScope.launch {
                try {
                    val (bitmap, pageRatio) = withContext(Dispatchers.IO) {
                        synchronized(renderLock) {
                            ensureActive()
                            val pdf = checkNotNull(renderer)
                            pdf.openPage(position).use { page ->
                                val scale = minOf(2f, 1600f / maxOf(page.width, page.height))
                                val bitmap = Bitmap.createBitmap(
                                    maxOf(1, (page.width * scale).roundToInt()),
                                    maxOf(1, (page.height * scale).roundToInt()),
                                    Bitmap.Config.ARGB_8888
                                )
                                bitmap.eraseColor(Color.WHITE)
                                page.render(bitmap, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
                                bitmap to (page.height.toFloat() / page.width)
                            }
                        }
                    }
                    if (index != position || !isAttachedToWindow) return@launch
                    if (ratio != pageRatio) {
                        ratio = pageRatio
                        requestLayout()
                    }
                    setImageBitmap(bitmap)
                    contentDescription = context.interfaceText("Original page {0}", position + 1)
                } catch (error: CancellationException) {
                    throw error
                } catch (error: Exception) {
                    if (index == position) contentDescription = context.interfaceText("Could not render page {0}", position + 1)
                }
            }
        }
    }
}
