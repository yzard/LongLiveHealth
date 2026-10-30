package health.app

import androidx.health.connect.client.HealthConnectClient
import androidx.health.connect.client.permission.HealthPermission
import androidx.health.connect.client.records.*
import androidx.health.connect.client.records.metadata.Device
import androidx.health.connect.client.request.ReadRecordsRequest
import androidx.health.connect.client.request.ChangesTokenRequest
import androidx.health.connect.client.changes.UpsertionChange
import androidx.health.connect.client.changes.DeletionChange
import androidx.health.connect.client.time.TimeRangeFilter
import org.json.JSONArray
import org.json.JSONObject
import java.time.Instant
import kotlin.reflect.KClass

/** Paged native reads. Cursor persistence belongs to shared Swift after server acknowledgement. */
object HealthConnectService {
    val types: Map<String, KClass<out Record>> = linkedMapOf(
        "sleep" to SleepSessionRecord::class, "workout" to ExerciseSessionRecord::class,
        "steps" to StepsRecord::class, "heart_rate" to HeartRateRecord::class,
        "resting_heart_rate" to RestingHeartRateRecord::class,
        "active_energy" to ActiveCaloriesBurnedRecord::class,
        "body_mass" to WeightRecord::class, "height" to HeightRecord::class,
        "body_fat" to BodyFatRecord::class, "vo2_max" to Vo2MaxRecord::class,
        "oxygen_saturation" to OxygenSaturationRecord::class,
        "respiratory_rate" to RespiratoryRateRecord::class,
        "hrv_rmssd" to HeartRateVariabilityRmssdRecord::class,
        "blood_glucose" to BloodGlucoseRecord::class,
        "body_temperature" to BodyTemperatureRecord::class,
        "walking_distance" to DistanceRecord::class
    )
    val permissions get() = types.values.map { HealthPermission.getReadPermission(it) }.toMutableSet().apply {
        val features = client().features
        if (features.getFeatureStatus(androidx.health.connect.client.HealthConnectFeatures.FEATURE_READ_HEALTH_DATA_HISTORY) == androidx.health.connect.client.HealthConnectFeatures.FEATURE_STATUS_AVAILABLE)
            add(HealthPermission.PERMISSION_READ_HEALTH_DATA_HISTORY)
        if (features.getFeatureStatus(androidx.health.connect.client.HealthConnectFeatures.FEATURE_READ_HEALTH_DATA_IN_BACKGROUND) == androidx.health.connect.client.HealthConnectFeatures.FEATURE_STATUS_AVAILABLE)
            add(HealthPermission.PERMISSION_READ_HEALTH_DATA_IN_BACKGROUND)
    }
    private fun client(): HealthConnectClient {
        check(HealthConnectClient.getSdkStatus(NativeServices.context) == HealthConnectClient.SDK_AVAILABLE) {
            "Health Connect is unavailable. Install or update Health Connect in Android Settings."
        }
        return HealthConnectClient.getOrCreate(NativeServices.context)
    }
    suspend fun perform(operation: String, input: JSONObject): JSONObject {
        val client = client()
        if (operation == "health.permissions") {
            val granted = client.permissionController.getGrantedPermissions()
            return JSONObject().put("types", JSONArray(types.filter { HealthPermission.getReadPermission(it.value) in granted }.keys.toList()))
        }
        if (operation == "health.writeGlucose") {
            val owner = NativeServices.activity.get() ?: error("Open the app to allow Health Connect access")
            val permission = HealthPermission.getWritePermission(BloodGlucoseRecord::class)
            check(permission in owner.requestHealthPermissions(setOf(permission))) { "Writing blood glucose was not allowed" }
            val unit = input.getString("unit")
            require(unit == "mg/dL" || unit == "mmol/L") { "Unsupported blood glucose unit" }
            val level = if (unit == "mg/dL") androidx.health.connect.client.units.BloodGlucose.milligramsPerDeciliter(input.getDouble("value"))
                else androidx.health.connect.client.units.BloodGlucose.millimolesPerLiter(input.getDouble("value"))
            val identity = java.security.MessageDigest.getInstance("SHA-256").digest(input.getString("identity").toByteArray()).joinToString("") { "%02x".format(it) }
            client.insertRecords(listOf(BloodGlucoseRecord(time = Instant.ofEpochMilli((input.getDouble("at") * 1000).toLong()), zoneOffset = null,
                level = level, metadata = androidx.health.connect.client.records.metadata.Metadata.manualEntry(clientRecordId = identity, clientRecordVersion = input.getLong("version")))))
            return JSONObject()
        }
        val kind = input.getString("kind")
        val type = types[kind] ?: error("Unsupported health type: $kind")
        check(HealthPermission.getReadPermission(type) in client.permissionController.getGrantedPermissions()) { "Health Connect permission is missing for $kind" }
        return when (operation) {
            "health.token" -> JSONObject().put("token", client.getChangesToken(ChangesTokenRequest(setOf(type))))
            "health.page" -> {
                val response = client.readRecords(ReadRecordsRequest(type,
                    timeRangeFilter = TimeRangeFilter.between(Instant.EPOCH, Instant.parse(input.getString("until"))),
                    pageSize = 1000, pageToken = input.optString("page").ifEmpty { null }))
                JSONObject().put("records", JSONArray(response.records.map { encode(it, kind) }))
                    .put("page", response.pageToken ?: "")
            }
            "health.changes" -> {
                val response = client.getChanges(input.getString("token"))
                if (response.changesTokenExpired) return JSONObject().put("expired", true)
                val records = response.changes.map { change -> when (change) {
                    is UpsertionChange -> encode(change.record, kind)
                    is DeletionChange -> JSONObject().put("record_id", change.recordId).put("source_id", "*")
                        .put("record_type", kind).put("start_at", 0).put("end_at", 0)
                        .put("version", 9007199254740991L).put("deleted", true).put("payload", JSONObject())
                    else -> error("Unsupported Health Connect change")
                } }
                JSONObject().put("records", JSONArray(records)).put("token", response.nextChangesToken).put("more", response.hasMore)
            }
            else -> error("Unsupported Health Connect operation")
        }
    }
    private fun encode(record: Record, kind: String): JSONObject {
        val metadata = record.metadata
        val origin = metadata.dataOrigin.packageName
        val payload = JSONObject().put("platform_identifier", record.javaClass.simpleName)
            .put("source_name", sourceName(origin))
        var source = origin
        metadata.device?.let { device ->
            val category = when (device.type) {
                Device.TYPE_WATCH -> "Watch"
                Device.TYPE_PHONE -> "Phone"
                Device.TYPE_FITNESS_BAND -> "Wrist band"
                Device.TYPE_CHEST_STRAP -> "Chest strap"
                else -> "Device"
            }
            val deviceName = listOfNotNull(device.manufacturer, device.model, category).joinToString(" ")
            val identity = JSONArray(listOf(device.manufacturer, device.model, device.type)).toString()
            val digest = java.security.MessageDigest.getInstance("SHA-256").digest(identity.toByteArray()).joinToString("") { "%02x".format(it) }
            source = "$origin:$digest"
            payload.put("source_name", "${sourceName(origin)} · $deviceName")
            payload.put("device", JSONObject().put("manufacturer", device.manufacturer)
                .put("model", device.model).put("type", device.type).put("name", deviceName))
        }
        val (start, end) = when (record) {
            is SleepSessionRecord -> record.startTime to record.endTime
            is ExerciseSessionRecord -> record.startTime to record.endTime
            is StepsRecord -> record.startTime to record.endTime
            is HeartRateRecord -> record.startTime to record.endTime
            is ActiveCaloriesBurnedRecord -> record.startTime to record.endTime
            is DistanceRecord -> record.startTime to record.endTime
            is RestingHeartRateRecord -> record.time to record.time
            is WeightRecord -> record.time to record.time
            is HeightRecord -> record.time to record.time
            is BodyFatRecord -> record.time to record.time
            is Vo2MaxRecord -> record.time to record.time
            is OxygenSaturationRecord -> record.time to record.time
            is RespiratoryRateRecord -> record.time to record.time
            is HeartRateVariabilityRmssdRecord -> record.time to record.time
            is BloodGlucoseRecord -> record.time to record.time
            is BodyTemperatureRecord -> record.time to record.time
            else -> error("Record has no supported timestamp")
        }
        payload.put("start_at_precise", start.toEpochMilli() / 1000.0).put("end_at_precise", end.toEpochMilli() / 1000.0)
        payload.put("metadata", JSONObject().put("id", metadata.id).put("client_record_id", metadata.clientRecordId)
            .put("client_record_version", metadata.clientRecordVersion).put("last_modified_at", metadata.lastModifiedTime.toString())
            .put("recording_method", metadata.recordingMethod))
        val value: Double?
        val unit: String
        when (record) {
            is SleepSessionRecord -> {
                value = null; unit = "seconds"
                payload.put("title", record.title).put("notes", record.notes)
                payload.put("stages", JSONArray(record.stages.map { JSONObject().put("start_at", it.startTime.epochSecond).put("end_at", it.endTime.epochSecond).put("stage", it.stage) }))
            }
            is ExerciseSessionRecord -> {
                value = null; unit = "seconds"
                payload.put("activity_type", record.exerciseType).put("title", record.title).put("notes", record.notes)
                    .put("duration_seconds", java.time.Duration.between(start, end).seconds)
                payload.put("segments", JSONArray(record.segments.map { JSONObject().put("start_at", it.startTime.epochSecond).put("end_at", it.endTime.epochSecond).put("segment_type", it.segmentType).put("repetitions", it.repetitions) }))
                payload.put("laps", JSONArray(record.laps.map { JSONObject().put("start_at", it.startTime.epochSecond).put("end_at", it.endTime.epochSecond).put("length_m", it.length?.inMeters) }))
            }
            is StepsRecord -> { value = record.count.toDouble(); unit = "count" }
            is RestingHeartRateRecord -> { value = record.beatsPerMinute.toDouble(); unit = "count/min" }
            is ActiveCaloriesBurnedRecord -> { value = record.energy.inKilocalories; unit = "kcal" }
            is WeightRecord -> { value = record.weight.inKilograms; unit = "kg" }
            is HeightRecord -> { value = record.height.inMeters; unit = "m" }
            is BodyFatRecord -> { value = record.percentage.value; unit = "%" }
            is Vo2MaxRecord -> { value = record.vo2MillilitersPerMinuteKilogram; unit = "ml/kg*min" }
            is OxygenSaturationRecord -> { value = record.percentage.value; unit = "%" }
            is RespiratoryRateRecord -> { value = record.rate; unit = "count/min" }
            is HeartRateVariabilityRmssdRecord -> { value = record.heartRateVariabilityMillis; unit = "ms" }
            is BloodGlucoseRecord -> { value = record.level.inMilligramsPerDeciliter; unit = "mg/dL" }
            is BodyTemperatureRecord -> { value = record.temperature.inCelsius; unit = "degC" }
            is DistanceRecord -> { value = record.distance.inMeters; unit = "m" }
            is HeartRateRecord -> {
                require(record.samples.isNotEmpty()) { "Heart rate record contains no samples" }
                value = record.samples.last().beatsPerMinute.toDouble(); unit = "count/min"
                payload.put("quantity_samples", JSONArray(record.samples.map { JSONObject()
                    .put("at", it.time.toEpochMilli() / 1000.0).put("value", it.beatsPerMinute) }))
            }
            else -> error("Unsupported health value: $kind")
        }
        payload.put("unit", unit)
        if (value != null) payload.put("value", value)
        return JSONObject().put("record_id", metadata.id).put("source_id", source).put("record_type", kind)
            .put("start_at", start.epochSecond).put("end_at", end.epochSecond)
            .put("version", maxOf(1L, metadata.lastModifiedTime.toEpochMilli())).put("deleted", false).put("payload", payload)
    }
    private fun sourceName(packageName: String): String = try {
        val manager = NativeServices.context.packageManager
        manager.getApplicationLabel(manager.getApplicationInfo(packageName, 0)).toString()
    } catch (_: android.content.pm.PackageManager.NameNotFoundException) { packageName }
}
