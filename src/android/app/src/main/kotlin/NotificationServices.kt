package health.app

import android.Manifest
import android.app.*
import android.content.Context
import android.os.Build
import androidx.core.app.NotificationManagerCompat
import androidx.work.*
import org.json.JSONObject
import java.util.concurrent.TimeUnit

object NotificationServices {
    private const val TAG = "longlivehealth-reminders"
    suspend fun perform(operation: String, input: JSONObject): JSONObject {
        val context = NativeServices.context
        val manager = WorkManager.getInstance(context)
        when (operation) {
            "notifications.authorize" -> {
                val owner = NativeServices.activity.get() ?: error("Open the app to allow notifications")
                val allowed = Build.VERSION.SDK_INT < 33 || owner.requestSystemPermission(Manifest.permission.POST_NOTIFICATIONS)
                return JSONObject().put("granted", allowed && NotificationManagerCompat.from(context).areNotificationsEnabled())
            }
            "notifications.clearReminders" -> {
                manager.cancelAllWorkByTag(TAG).result.get()
                val notifications = context.getSystemService(NotificationManager::class.java)
                notifications.activeNotifications.filter { it.tag?.startsWith("longlivehealth-reminder:") == true }.forEach { notifications.cancel(it.tag, it.id) }
            }
            "notifications.remove" -> {
                manager.cancelUniqueWork(input.getString("id")).result.get()
                context.getSystemService(NotificationManager::class.java).cancel(input.getString("id"), 0)
            }
            "notifications.schedule" -> {
                val id = input.getString("id")
                val data = workDataOf("id" to id, "title" to input.getString("title"), "body" to input.getString("body"))
                val delay = maxOf(0, (input.getDouble("at") * 1000).toLong() - System.currentTimeMillis())
                val request = OneTimeWorkRequestBuilder<ReminderWorker>().setInputData(data).setInitialDelay(delay, TimeUnit.MILLISECONDS).addTag(TAG).build()
                manager.enqueueUniqueWork(id, ExistingWorkPolicy.REPLACE, request).result.get()
            }
            else -> error("Unknown notification action")
        }
        return JSONObject()
    }
}

class ReminderWorker(context: Context, parameters: WorkerParameters) : CoroutineWorker(context, parameters) {
    override suspend fun doWork(): Result {
        val id = inputData.getString("id") ?: return Result.failure()
        val manager = applicationContext.getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel("reminders", "Reminders", NotificationManager.IMPORTANCE_DEFAULT))
        if (!NotificationManagerCompat.from(applicationContext).areNotificationsEnabled()) return Result.failure()
        val notification = Notification.Builder(applicationContext, "reminders").setSmallIcon(android.R.drawable.ic_popup_reminder)
            .setContentTitle(inputData.getString("title")).setContentText(inputData.getString("body"))
            .setVisibility(Notification.VISIBILITY_PRIVATE).setAutoCancel(true).build()
        manager.notify(id, 0, notification)
        return Result.success()
    }
}
