package health.app

import android.Manifest
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.speech.*
import kotlinx.coroutines.*
import org.json.JSONObject
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

object SpeechServices {
    suspend fun transcribe(): JSONObject {
        val owner = NativeServices.activity.get() ?: error("Open the app to dictate")
        if (Build.VERSION.SDK_INT < 31) error("On-device dictation requires Android 12 or later. Type your question.")
        check(SpeechRecognizer.isOnDeviceRecognitionAvailable(owner)) { "On-device dictation is unavailable. Type your question." }
        check(owner.requestSystemPermission(Manifest.permission.RECORD_AUDIO)) { "Microphone permission was not granted" }
        return withContext(Dispatchers.Main) {
            withTimeout(60_000) {
                suspendCancellableCoroutine { continuation ->
                    val recognizer = SpeechRecognizer.createOnDeviceSpeechRecognizer(owner)
                    var finished = false
                    fun finish(text: String?, error: String?) {
                        if (finished) return
                        finished = true; recognizer.destroy()
                        if (!continuation.isActive) return
                        if (error != null) continuation.resumeWithException(IllegalStateException(error))
                        else continuation.resume(JSONObject().put("text", text ?: ""))
                    }
                    recognizer.setRecognitionListener(object : RecognitionListener {
                        override fun onReadyForSpeech(params: Bundle?) {}
                        override fun onBeginningOfSpeech() {}
                        override fun onRmsChanged(value: Float) {}
                        override fun onBufferReceived(buffer: ByteArray?) {}
                        override fun onEndOfSpeech() {}
                        override fun onPartialResults(results: Bundle?) {}
                        override fun onEvent(type: Int, params: Bundle?) {}
                        override fun onError(error: Int) { finish(null, "On-device dictation failed ($error). Try again or type your question.") }
                        override fun onResults(results: Bundle?) { finish(results?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)?.firstOrNull(), null) }
                    })
                    continuation.invokeOnCancellation { owner.runOnUiThread { if (!finished) { finished = true; recognizer.cancel(); recognizer.destroy() } } }
                    recognizer.startListening(Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM).putExtra(RecognizerIntent.EXTRA_PREFER_OFFLINE, true))
                }
            }
        }
    }
}
