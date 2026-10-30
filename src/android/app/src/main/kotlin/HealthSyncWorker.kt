package health.app

import android.app.*
import android.content.Context
import android.content.pm.ServiceInfo
import android.os.Build
import androidx.work.*
import java.util.concurrent.TimeUnit

class HealthSyncWorker(context: Context, parameters: WorkerParameters) : CoroutineWorker(context, parameters) {
    override suspend fun doWork(): Result {
        val manager = applicationContext.getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel("health-transfers", "Health transfers", NotificationManager.IMPORTANCE_LOW))
        val notification = Notification.Builder(applicationContext, "health-transfers").setSmallIcon(android.R.drawable.stat_sys_upload)
            .setContentTitle("Long Live Health").setContentText("Syncing health data").setOngoing(true).build()
        setForeground(if (Build.VERSION.SDK_INT >= 29) ForegroundInfo(911, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC) else ForegroundInfo(911, notification))
        running = true
        return try {
            if (HealthAppAppDelegate.shared.syncInBackground()) Result.success() else Result.retry()
        } finally { running = false }
    }
    companion object {
        @Volatile var running = false
        fun schedule(context: Context) {
            val constraints = Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()
            val request = PeriodicWorkRequestBuilder<HealthSyncWorker>(15, TimeUnit.MINUTES).setConstraints(constraints).build()
            WorkManager.getInstance(context).enqueueUniquePeriodicWork("health-sync", ExistingPeriodicWorkPolicy.KEEP, request)
        }
    }
}
