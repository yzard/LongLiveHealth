package health.app

import android.app.*
import android.content.Context
import android.content.Intent
import android.os.Handler
import android.os.IBinder
import android.os.Looper
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
        synchronized(lock) {
            active = this
            starting = false
            stopWhenIdle()
        }
        return START_NOT_STICKY
    }
    override fun onTimeout(startId: Int, fgsType: Int) {
        HealthAppAppDelegate.shared.pauseForSystem()
        synchronized(lock) {
            leases.clear()
            stopWhenIdle()
        }
    }
    override fun onDestroy() {
        synchronized(lock) {
            if (active === this) active = null
        }
        super.onDestroy()
    }
    private fun stopWhenIdle() {
        if (active === this && !starting && leases.isEmpty()) {
            active = null
            stopSelf()
        }
    }
    companion object {
        private const val CHANNEL = "health-transfers"
        private val leases = ConcurrentHashMap<String, String>()
        private val lock = Any()
        private val main = Handler(Looper.getMainLooper())
        private var starting = false
        private var active: TransferService? = null

        fun acquire(context: Context, label: String): String = synchronized(lock) {
            val id = UUID.randomUUID().toString()
            leases[id] = label
            if (active == null && !starting) {
                starting = true
                try {
                    ContextCompat.startForegroundService(context, Intent(context, TransferService::class.java))
                } catch (error: Exception) {
                    starting = false
                    leases.remove(id)
                    throw error
                }
            }
            id
        }

        fun release(id: String) {
            synchronized(lock) { leases.remove(id) }
            main.post { synchronized(lock) { active?.stopWhenIdle() } }
        }
    }
}
