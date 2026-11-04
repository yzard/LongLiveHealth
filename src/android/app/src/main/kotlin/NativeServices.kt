package health.app

import android.app.KeyguardManager
import android.app.NotificationManager
import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.biometric.BiometricManager.Authenticators
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import kotlinx.coroutines.*
import org.json.JSONObject
import java.lang.ref.WeakReference
import java.security.KeyStore
import java.security.SecureRandom
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

/** Native capabilities only. Never logs tokens, report contents or health values. */
class NativeServices {
companion object {
    lateinit var context: Context
    var activity = WeakReference<MainActivity>(null)
    @Volatile var active = false
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val jobs = ConcurrentHashMap<String, Deferred<JSONObject>>()
    private val preferences get() = context.getSharedPreferences("secure-session", Context.MODE_PRIVATE)
    private const val keyAlias = "longlivehealth.session.aes"

    private fun encryptionKey(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(keyAlias, null) as? SecretKey)?.let { return it }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(keyAlias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())
        }.generateKey()
    }

    @JvmStatic fun invoke(operation: String, input: String): String = try {
        val value = when (operation) {
            "session.save" -> {
                val cipher = Cipher.getInstance("AES/GCM/NoPadding")
                cipher.init(Cipher.ENCRYPT_MODE, encryptionKey())
                val bytes = cipher.iv + cipher.doFinal(input.toByteArray(Charsets.UTF_8))
                check(preferences.edit().putString("session", Base64.encodeToString(bytes, Base64.NO_WRAP)).commit()) { "Could not save secure session" }
                ""
            }
            "session.load" -> preferences.getString("session", null)?.let {
                val bytes = Base64.decode(it, Base64.NO_WRAP)
                require(bytes.size > 12) { "Secure session is damaged" }
                val cipher = Cipher.getInstance("AES/GCM/NoPadding")
                cipher.init(Cipher.DECRYPT_MODE, encryptionKey(), GCMParameterSpec(128, bytes.copyOfRange(0, 12)))
                String(cipher.doFinal(bytes.copyOfRange(12, bytes.size)), Charsets.UTF_8)
            } ?: ""
            "session.clear" -> {
                check(preferences.edit().remove("session").commit()) { "Could not remove secure session" }; ""
            }
            "clipboard.copy" -> { context.getSystemService(android.content.ClipboardManager::class.java).setPrimaryClip(android.content.ClipData.newPlainText("Long Live Health", input)); "" }
            "random" -> ByteArray(32).also { SecureRandom().nextBytes(it) }.joinToString("") { "%02x".format(it) }
            "screen.awake" -> {
                val owner = activity.get() ?: error("Screen is unavailable")
                owner.runOnUiThread {
                    if (input == "true") owner.window.addFlags(android.view.WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
                    else owner.window.clearFlags(android.view.WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
                }
                ""
            }
            "active" -> active.toString()
            "unlocked" -> context.getSystemService(android.os.UserManager::class.java).isUserUnlocked.toString()
            "notifications.clear" -> { context.getSystemService(NotificationManager::class.java).cancelAll(); "" }
            "background.schedule" -> { HealthSyncWorker.schedule(context); "" }
            "lease.begin" -> if (HealthSyncWorker.running) "worker" else TransferService.acquire(context, input)
            "lease.end" -> { if (input != "worker") TransferService.release(input); "" }
            "start" -> {
                val request = JSONObject(input)
                val id = UUID.randomUUID().toString()
                jobs[id] = scope.async { perform(request.getString("operation"), JSONObject(request.getString("input"))) }
                id
            }
            "poll" -> {
                val job = jobs[input] ?: error("Operation is no longer available")
                if (!job.isCompleted) "" else {
                    jobs.remove(input)
                    runBlocking { job.await().toString() }
                }
            }
            "cancel" -> { jobs.remove(input)?.cancel(); "" }
            else -> error("Unsupported system operation: $operation")
        }
        JSONObject().put("value", value).toString()
    } catch (error: Exception) {
        JSONObject().put("error", error.message ?: "Android system operation failed").toString()
    }

    private suspend fun perform(operation: String, input: JSONObject): JSONObject = when (operation) {
        "authenticate" -> authenticate(input)
        "speech.transcribe" -> SpeechServices.transcribe()
        "health.authorize" -> {
            val owner = activity.get() ?: error("Open the app to allow Health Connect access")
            JSONObject().put("granted", org.json.JSONArray(owner.requestHealthPermissions(emptySet()).toList()))
        }
        "calendar.review" -> withContext(Dispatchers.Main) {
            val owner = activity.get() ?: error("Open the app to review a calendar event")
            owner.startActivity(android.content.Intent(android.content.Intent.ACTION_INSERT).setData(android.provider.CalendarContract.Events.CONTENT_URI)
                .putExtra(android.provider.CalendarContract.Events.TITLE, input.getString("title"))
                .putExtra(android.provider.CalendarContract.EXTRA_EVENT_BEGIN_TIME, (input.getDouble("start") * 1000).toLong())
                .putExtra(android.provider.CalendarContract.EXTRA_EVENT_END_TIME, ((input.getDouble("start") + maxOf(1.0, input.getDouble("duration")) * 60) * 1000).toLong()))
            JSONObject()
        }
        else -> if (operation.startsWith("notifications.")) NotificationServices.perform(operation, input)
            else if (operation.startsWith("document.")) DocumentServices.perform(operation, input)
            else if (operation.startsWith("health.")) HealthConnectService.perform(operation, input)
            else error("Unsupported asynchronous operation: $operation")
    }

    private suspend fun authenticate(input: JSONObject): JSONObject = withContext(Dispatchers.Main) {
        val owner = activity.get() ?: error("Open the app to authenticate")
        suspendCancellableCoroutine { continuation ->
            val prompt = BiometricPrompt(owner, ContextCompat.getMainExecutor(owner), object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    if (continuation.isActive) continuation.resume(JSONObject().put("authenticated", true))
                }
                override fun onAuthenticationError(code: Int, message: CharSequence) {
                    if (!continuation.isActive) return
                    if (code == BiometricPrompt.ERROR_USER_CANCELED || code == BiometricPrompt.ERROR_NEGATIVE_BUTTON || code == BiometricPrompt.ERROR_CANCELED) {
                        continuation.resume(JSONObject().put("authenticated", false))
                    } else continuation.resumeWithException(IllegalStateException(message.toString()))
                }
            })
            val biometricOnly = input.getBoolean("biometrics")
            val builder = BiometricPrompt.PromptInfo.Builder().setTitle(input.getString("reason"))
                .setAllowedAuthenticators(if (biometricOnly) Authenticators.BIOMETRIC_STRONG else Authenticators.BIOMETRIC_STRONG or Authenticators.DEVICE_CREDENTIAL)
            if (biometricOnly) builder.setNegativeButtonText("Cancel")
            continuation.invokeOnCancellation { owner.runOnUiThread { prompt.cancelAuthentication() } }
            prompt.authenticate(builder.build())
        }
    }
}

}
