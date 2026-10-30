package health.app

import androidx.health.connect.client.HealthConnectClient
import androidx.health.connect.client.records.*
import androidx.health.connect.client.records.metadata.Metadata
import androidx.health.connect.client.records.metadata.Device
import androidx.health.connect.client.units.Mass
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.time.Instant
import java.util.UUID

/** Synthetic data only. Cleanup removes every record created by this test. */
@RunWith(AndroidJUnit4::class)
class HealthConnectIntegrationTest {
    @Test fun historicalPagesAndIncrementalDeletionPreserveSamples() = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext
        val permissions = listOf("READ_HEART_RATE", "WRITE_HEART_RATE", "READ_SLEEP", "WRITE_SLEEP", "READ_WEIGHT", "WRITE_WEIGHT", "READ_HEALTH_DATA_HISTORY")
        permissions.forEach { permission ->
            instrumentation.uiAutomation.executeShellCommand("pm grant ${context.packageName} android.permission.health.$permission").use {
                android.os.ParcelFileDescriptor.AutoCloseInputStream(it).readBytes()
            }
        }
        val client = HealthConnectClient.getOrCreate(context)
        val at = Instant.now().minusSeconds(60L * 86400)
        val marker = UUID.randomUUID().toString()
        val token = HealthConnectService.perform("health.token", JSONObject().put("kind", "heart_rate")).getString("token")
        var heartIds = emptyList<String>(); var sleepIds = emptyList<String>(); var weightIds = emptyList<String>()
        try {
            heartIds = client.insertRecords(listOf(HeartRateRecord(startTime = at, startZoneOffset = null, endTime = at.plusSeconds(60), endZoneOffset = null,
                samples = listOf(HeartRateRecord.Sample(at.plusSeconds(5), 62), HeartRateRecord.Sample(at.plusSeconds(10), 79)),
                metadata = Metadata.manualEntry(clientRecordId = "$marker-heart")))).recordIdsList
            sleepIds = client.insertRecords(listOf(SleepSessionRecord(startTime = at, startZoneOffset = null, endTime = at.plusSeconds(3600), endZoneOffset = null,
                stages = listOf(SleepSessionRecord.Stage(at, at.plusSeconds(1800), SleepSessionRecord.STAGE_TYPE_LIGHT),
                    SleepSessionRecord.Stage(at.plusSeconds(1800), at.plusSeconds(3600), SleepSessionRecord.STAGE_TYPE_REM)),
                metadata = Metadata.manualEntry(clientRecordId = "$marker-sleep")))).recordIdsList
            weightIds = client.insertRecords(listOf(
                WeightRecord(time = at, zoneOffset = null, weight = Mass.kilograms(71.25), metadata = Metadata.manualEntry(clientRecordId = "$marker-weight-watch", device = Device(type = Device.TYPE_WATCH, manufacturer = "Synthetic", model = "Test Watch"))),
                WeightRecord(time = at, zoneOffset = null, weight = Mass.kilograms(71.5), metadata = Metadata.manualEntry(clientRecordId = "$marker-weight-phone", device = Device(type = Device.TYPE_PHONE, manufacturer = "Synthetic", model = "Test Phone")))
            )).recordIdsList
            fun request(kind: String) = JSONObject().put("kind", kind).put("until", Instant.now().toString()).put("page", "")
            val heart = HealthConnectService.perform("health.page", request("heart_rate")).getJSONArray("records")
            val row = (0 until heart.length()).map { heart.getJSONObject(it) }.single { it.getString("record_id") == heartIds.single() }
            val samples = row.getJSONObject("payload").getJSONArray("quantity_samples")
            assertEquals(2, samples.length())
            assertEquals(79.0, samples.getJSONObject(1).getDouble("value"), 0.0)
            assertEquals(at.plusSeconds(10).epochSecond.toDouble(), samples.getJSONObject(1).getDouble("at"), 1.0)
            val sleep = HealthConnectService.perform("health.page", request("sleep")).getJSONArray("records")
            val night = (0 until sleep.length()).map { sleep.getJSONObject(it) }.single { it.getString("record_id") == sleepIds.single() }
            assertEquals(2, night.getJSONObject("payload").getJSONArray("stages").length())
            val weight = HealthConnectService.perform("health.page", request("body_mass")).getJSONArray("records")
            val mass = (0 until weight.length()).map { weight.getJSONObject(it) }.single { it.getString("record_id") == weightIds.first() }
            assertEquals(71.25, mass.getJSONObject("payload").getDouble("value"), 0.0)
            val phone = (0 until weight.length()).map { weight.getJSONObject(it) }.single { it.getString("record_id") == weightIds.last() }
            assertNotEquals(mass.getString("source_id"), phone.getString("source_id"))
            assertTrue(mass.getJSONObject("payload").getJSONObject("device").getString("name").contains("Watch"))
            val changes = HealthConnectService.perform("health.changes", JSONObject().put("kind", "heart_rate").put("token", token))
            assertTrue(changes.getJSONArray("records").length() > 0)
            client.deleteRecords(HeartRateRecord::class, heartIds, emptyList())
            val deleted = HealthConnectService.perform("health.changes", JSONObject().put("kind", "heart_rate").put("token", changes.getString("token"))).getJSONArray("records")
            assertTrue((0 until deleted.length()).any { deleted.getJSONObject(it).optBoolean("deleted") && deleted.getJSONObject(it).getString("record_id") == heartIds.single() })
        } finally {
            if (heartIds.isNotEmpty()) client.deleteRecords(HeartRateRecord::class, heartIds, emptyList())
            if (sleepIds.isNotEmpty()) client.deleteRecords(SleepSessionRecord::class, sleepIds, emptyList())
            if (weightIds.isNotEmpty()) client.deleteRecords(WeightRecord::class, weightIds, emptyList())
        }
    }
}
