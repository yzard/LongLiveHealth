package health.app

import android.app.*
import android.content.Context
import android.content.Intent
import android.os.IBinder
import androidx.core.content.ContextCompat
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/** Keeps user-initiated transfers alive while the app is in the background. */
class TransferService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel(CHANNEL, "Health transfers", NotificationManager.IMPORTANCE_LOW))
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE)
        startForeground(910, Notification.Builder(this, CHANNEL).setSmallIcon(android.R.drawable.stat_sys_upload)
            .setContentTitle("Long Live Health").setContentText(leases.values.firstOrNull() ?: "Finishing transfer")
            .setContentIntent(open).setOngoing(true).build())
        if (leases.isEmpty()) stopSelf()
        return START_NOT_STICKY
    }
    override fun onTimeout(startId: Int, fgsType: Int) {
        HealthAppAppDelegate.shared.pauseForSystem()
        leases.clear()
        stopSelf()
    }
    companion object {
        private const val CHANNEL = "health-transfers"
        private val leases = ConcurrentHashMap<String, String>()
        fun acquire(context: Context, label: String): String {
            val id = UUID.randomUUID().toString()
            leases[id] = label
            try { ContextCompat.startForegroundService(context, Intent(context, TransferService::class.java)) }
            catch (error: Exception) { leases.remove(id); throw error }
            return id
        }
        fun release(context: Context, id: String) {
            leases.remove(id)
            if (leases.isEmpty()) context.stopService(Intent(context, TransferService::class.java))
        }
    }
}
