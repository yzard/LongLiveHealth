import Foundation
import Observation
import CryptoKit
@preconcurrency import HealthKit

@MainActor @Observable
final class HealthBridge {
    var status = "Not connected"
    var isSyncing = false
    private(set) var activity: HealthSyncActivity?
    var coverage: [JSONValue] = []
    private(set) var totalRecords: Int?
    private let store = HKHealthStore()

    struct Definition {
        let kind: String
        let type: HKSampleType
        let unit: String?
    }
    struct Page: Sendable {
        let records: [JSONValue]
        let anchor: Data
        let count: Int
    }

    var definitions: [Definition] { HealthCatalog.definitions }

    func writeReviewedGlucose(_ observation: JSONValue, server: URL, userID: String, sampledAt: Date) async throws {
        guard HKHealthStore.isHealthDataAvailable() else { throw APIError.invalidResponse }
        let sample = try Self.reviewedGlucoseSample(observation, server: server, userID: userID, sampledAt: sampledAt)
        try await store.requestAuthorization(toShare: [sample.quantityType], read: [])
        try await store.save(sample)
        status = "Reviewed glucose saved to Apple Health"
    }

    nonisolated static func reviewedGlucoseSample(_ observation: JSONValue, server: URL, userID: String, sampledAt: Date) throws -> HKQuantitySample {
        guard observation["status"] == .string("confirmed"),
              observation["payload"]["metric_id"] == .string("glucose"),
              let value = Double(observation["payload"]["raw_result"].stringValue.trimmingCharacters(in: .whitespacesAndNewlines)),
              value.isFinite, value >= 0,
              let type = HKObjectType.quantityType(forIdentifier: .bloodGlucose),
              let revision = observation["revision"].numberValue, revision >= 1 else { throw APIError.invalidResponse }
        let unit = observation["payload"]["raw_unit"].stringValue
        guard ["mg/dL", "mmol/L"].contains(unit), !observation["observation_id"].stringValue.isEmpty else { throw APIError.invalidResponse }
        let identity = "\(server.absoluteString)|\(userID)|\(observation["observation_id"].stringValue)"
        let identifier = SHA256.hash(data: Data(identity.utf8)).map { String(format: "%02x", $0) }.joined()
        let healthUnit = unit == "mmol/L"
            ? HKUnit.moleUnit(with: .milli, molarMass: HKUnitMolarMassBloodGlucose).unitDivided(by: .liter())
            : HKUnit(from: "mg/dL")
        return HKQuantitySample(type: type, quantity: HKQuantity(unit: healthUnit, doubleValue: value), start: sampledAt, end: sampledAt,
            metadata: [HKMetadataKeySyncIdentifier: "longlivehealth:\(identifier)", HKMetadataKeySyncVersion: NSNumber(value: revision),
                       "longlivehealth_observation_id": observation["observation_id"].stringValue])
    }

    static func bulkReadTypes(supportsClinical: Bool) -> Set<HKObjectType> {
        let samples = HealthCatalog.definitions.filter {
            !($0.type is HKCorrelationType) && (!($0.type is HKClinicalType) || supportsClinical)
        }.map { $0.type as HKObjectType }
        return Set(samples).union(HealthSnapshots.readTypes).filter {
            !$0.requiresPerObjectAuthorization() && $0.identifier != "HKMedicationDoseEventTypeIdentifierMedicationDoseEvent"
        }
    }

    static var selectedReadTypes: [HKObjectType] {
        var types: [HKObjectType] = [HKObjectType.visionPrescriptionType()]
        if #available(iOS 26.0, *) { types.append(HKObjectType.userAnnotatedMedicationType()) }
        return types
    }

    func synchronize(client: APIClient, archive: LocalArchive, requestAccess: Bool, requestSelectedAccess: Bool) async throws {
        guard !isSyncing else { return }
        guard HKHealthStore.isHealthDataAvailable() else { status = "Health data unavailable on this device"; return }
        if !requestAccess, try archive.load(Bool.self, name: "health-enabled.json") != true { return }
        isSyncing = true; totalRecords = nil; status = "Preparing Apple Health sync"
        defer { isSyncing = false; activity = nil }
        do {
            if requestAccess {
                try await store.requestAuthorization(toShare: [], read: Self.bulkReadTypes(supportsClinical: store.supportsHealthRecords()))
                try archive.save(true, name: "health-enabled.json")
            }
            if requestSelectedAccess {
                for type in Self.selectedReadTypes {
                    try await store.requestPerObjectReadAuthorization(for: type, predicate: nil)
                }
            }
            let installation = try archive.load(UUID.self, name: "installation.json") ?? UUID()
            try archive.save(installation, name: "installation.json")
            let connection = try await client.post("health/connect", body: .object(["platform": .string("apple_health"), "installation_id": .string(installation.uuidString)]))
            let connectionID = connection["connection_id"].stringValue
            guard !connectionID.isEmpty else { throw APIError.invalidResponse }
            totalRecords = try HealthSyncProgress.totalRecords(in: await client.post("health/coverage", body: .object([:])))
            var failedTypes: [String] = []
            for definition in definitions {
                // These records are read only after the user explicitly selects them.
                if !requestSelectedAccess, definition.type.requiresPerObjectAuthorization() || definition.kind == "medication_dose_event" { continue }
                if definition.type is HKClinicalType, !store.supportsHealthRecords() {
                    _ = try await client.post("health/sync", body: .object(["connection_id": .string(connectionID), "batch_id": .string(UUID().uuidString), "record_type": .string(definition.kind), "coverage_status": .string("unsupported"), "records": .array([])]))
                    continue
                }
                try Task.checkCancellation()
                status = HealthSyncProgress.status(recordType: definition.kind, total: totalRecords)
                activity = HealthSyncActivity()
                let pendingName = "pending-\(definition.kind).json"
                let anchorName = "anchor-\(definition.kind).json"
                if let pending = try archive.load(SyncCheckpoint.self, name: pendingName) {
                    for request in pending.requests {
                        try Task.checkCancellation()
                        try await send(request, client: client)
                        status = HealthSyncProgress.status(recordType: definition.kind, total: totalRecords)
                    }
                    try archive.save(pending.nextAnchor, name: anchorName)
                    try archive.remove(name: pendingName)
                }
                let encoded = try archive.load(Data.self, name: anchorName)
                let anchor = try encoded.flatMap { try NSKeyedUnarchiver.unarchivedObject(ofClass: HKQueryAnchor.self, from: $0) }
                // Read failures remain type-specific; upload failures pause the entire sync.
                var readFailure: (any Error)?
                let read: @MainActor @Sendable (HKQueryAnchor?) async throws -> Page? = { [self] cursor in
                    do {
                        try Task.checkCancellation()
                        activity?.readingSince = Date()
                        defer { activity?.readingSince = nil }
                        let page = try await query(definition, anchor: cursor)
                        return page
                    } catch is CancellationError { throw CancellationError() }
                    catch {
                        try Task.checkCancellation()
                        if Self.isProtectedDataError(error) { throw error }
                        readFailure = error
                        return nil
                    }
                }
                let first = try await read(anchor)
                try await HealthSyncPipeline.run(first: first, next: { page in
                    guard page.count == HealthSyncBatch.maximumRecords else { return nil }
                    let cursor = try NSKeyedUnarchiver.unarchivedObject(ofClass: HKQueryAnchor.self, from: page.anchor)
                    return try await read(cursor)
                }, consume: { [self] page in
                    let requests = try HealthSyncBatch.requests(connectionID: connectionID, recordType: definition.kind,
                        coverageStatus: page.count == 0 ? "no_visible_samples" : "observed", records: page.records, maximumRecords: HealthSyncBatch.maximumRecords, maximumBytes: 40 * 1024 * 1024)
                    // Only the uploading page is durable; prefetched data can be read again.
                    // Never advance the committed anchor before every split request succeeds.
                    try archive.save(SyncCheckpoint(requests: requests, nextAnchor: page.anchor), name: pendingName)
                    for request in requests {
                        try Task.checkCancellation()
                        try await send(request, client: client)
                        status = HealthSyncProgress.status(recordType: definition.kind, total: totalRecords)
                    }
                    try archive.save(page.anchor, name: anchorName)
                    try archive.remove(name: pendingName)
                })
                if readFailure != nil {
                    _ = try await client.post("health/sync", body: .object(["connection_id": .string(connectionID), "batch_id": .string(UUID().uuidString), "record_type": .string(definition.kind), "coverage_status": .string("error"), "records": .array([])]))
                    failedTypes.append(definition.kind)
                }
            }
            activity = nil
            status = "Syncing activity summaries and other Health records"
            failedTypes += try await HealthSnapshots.synchronize(store: store, client: client, archive: archive, connectionID: connectionID, installationID: installation, includeSelectedRecords: requestSelectedAccess) { kind, total in
                if let total { totalRecords = total }
                status = HealthSyncProgress.status(recordType: kind, total: totalRecords)
            }
            status = "Loading sync coverage"
            let summary = try await client.post("health/coverage", body: .object([:]))
            coverage = summary["coverage"].arrayValue
            totalRecords = try HealthSyncProgress.totalRecords(in: summary)
            status = failedTypes.isEmpty ? "Sync complete · \(totalRecords?.formatted() ?? ""). Read permission remains private to Health." : "Some types could not be read: \(failedTypes.map(HealthSyncProgress.name).joined(separator: ", ")). Retry when available."
        } catch {
            status = "Sync paused; saved batches retained. Tap Sync Now to resume."
            throw error
        }
    }

    private func send(_ request: JSONValue, client: APIClient) async throws {
        activity?.beginSending(records: request["records"].arrayValue.count, at: Date())
        defer { activity?.endSending() }
        totalRecords = try HealthSyncProgress.totalRecords(in: await client.post("health/sync", body: request))
        activity?.confirmSending()
    }

    nonisolated static func isProtectedDataError(_ error: Error) -> Bool {
        let value = error as NSError
        return value.domain == HKErrorDomain && value.code == HKError.Code.errorDatabaseInaccessible.rawValue
    }

    private func query(_ definition: Definition, anchor: HKQueryAnchor?) async throws -> Page {
        let descriptor = HKAnchoredObjectQueryDescriptor(predicates: [.sample(type: definition.type, predicate: nil)], anchor: anchor, limit: HealthSyncBatch.maximumRecords)
        let result = try await descriptor.result(for: store)
        var records: [JSONValue] = []
        for sample in result.addedSamples {
            try Task.checkCancellation()
            var record = try Self.encode(sample, kind: definition.kind, unit: definition.unit)
            if let series = try await HealthSeriesReader.read(sample, store: store) { record["payload"]["series"] = .array(series) }
            guard try JSONEncoder().encode(record["payload"]).count <= 32 * 1024 * 1024 else { throw APIError.status(413, "A Health record exceeds the archive limit") }
            records.append(record)
        }
        records += result.deletedObjects.map { deletion in
            .object(["record_id": .string(deletion.uuid.uuidString), "source_id": .string("*"), "record_type": .string(definition.kind),
                     "start_at": .number(0), "end_at": .number(0), "version": .number(1), "deleted": .bool(true), "payload": .object([:])])
        }
        let anchor = try NSKeyedArchiver.archivedData(withRootObject: result.newAnchor, requiringSecureCoding: true)
        return Page(records: records, anchor: anchor, count: result.addedSamples.count + result.deletedObjects.count)
    }

    nonisolated static func encode(_ sample: HKSample, kind: String, unit: String?) throws -> JSONValue {
        var payload: [String: JSONValue] = [
            "platform_identifier": .string(sample.sampleType.identifier),
            "raw_archive": .object(["format": .string("nskeyedarchiver-secure-v1"), "encoding": .string("base64"),
                                    "data": .string(try NSKeyedArchiver.archivedData(withRootObject: sample, requiringSecureCoding: true).base64EncodedString())]),
            "start_at_precise": .number(sample.startDate.timeIntervalSince1970),
            "end_at_precise": .number(sample.endDate.timeIntervalSince1970),
            "source_name": .string(sample.sourceRevision.source.name),
            "source_version": .optional(sample.sourceRevision.version ?? ""),
            "metadata": encodeMetadata(sample.metadata ?? [:])
        ]
        if let sample = sample as? HKQuantitySample {
            payload["raw_quantity"] = .string(sample.quantity.description)
            payload["quantity_count"] = .number(Double(sample.count))
            if let unit {
                payload["value"] = .number(sample.quantity.doubleValue(for: HKUnit(from: unit)))
                payload["unit"] = .string(unit)
            }
        }
        if let ecg = sample as? HKElectrocardiogram {
            payload["ecg_classification"] = .number(Double(ecg.classification.rawValue))
            payload["ecg_symptoms_status"] = .number(Double(ecg.symptomsStatus.rawValue))
            if let frequency = ecg.samplingFrequency { payload["sampling_frequency_hz"] = .number(frequency.doubleValue(for: HKUnit.hertz())) }
        }
        if let clinical = sample as? HKClinicalRecord, let resource = clinical.fhirResource {
            payload["fhir"] = .object(["resource_type": .string(resource.resourceType.rawValue), "identifier": .string(resource.identifier),
                                        "data_base64": .string(resource.data.base64EncodedString())])
        }
        if let correlation = sample as? HKCorrelation {
            payload["related_sample_ids"] = .array(correlation.objects.map { .string($0.uuid.uuidString) }.sorted { $0.stringValue < $1.stringValue })
        }
        if let sample = sample as? HKCategorySample { payload["category"] = .number(Double(sample.value)) }
        if let workout = sample as? HKWorkout {
            payload["activity_type"] = .number(Double(workout.workoutActivityType.rawValue))
            payload["duration_seconds"] = .number(workout.duration)
            if let energy = workout.totalEnergyBurned { payload["energy_kcal"] = .number(energy.doubleValue(for: .kilocalorie())) }
            if let distance = workout.totalDistance { payload["distance_m"] = .number(distance.doubleValue(for: .meter())) }
            payload["events"] = .array((workout.workoutEvents ?? []).map { event in
                .object(["type": .number(Double(event.type.rawValue)), "start_at": .number(event.dateInterval.start.timeIntervalSince1970), "end_at": .number(event.dateInterval.end.timeIntervalSince1970), "metadata": encodeMetadata(event.metadata ?? [:])])
            })
        }
        if let device = sample.device {
            payload["device"] = .object(["name": .optional(device.name ?? ""), "manufacturer": .optional(device.manufacturer ?? ""), "model": .optional(device.model ?? ""), "hardware_version": .optional(device.hardwareVersion ?? ""), "software_version": .optional(device.softwareVersion ?? "")])
        }
        let version = (sample.metadata?[HKMetadataKeySyncVersion] as? NSNumber)?.doubleValue ?? 1
        return .object(["record_id": .string(sample.uuid.uuidString), "source_id": .string(sample.sourceRevision.source.bundleIdentifier),
                        "record_type": .string(kind), "start_at": .number(floor(sample.startDate.timeIntervalSince1970)), "end_at": .number(floor(sample.endDate.timeIntervalSince1970)),
                        "version": .number(max(1, version)), "deleted": .bool(false), "payload": .object(payload)])
    }

    nonisolated private static func encodeMetadata(_ metadata: [String: Any]) -> JSONValue {
        .object(metadata.mapValues { value in
            if let text = value as? String { return .string(text) }
            if let number = value as? NSNumber { return .number(number.doubleValue) }
            if let date = value as? Date { return .string(ISO8601DateFormatter().string(from: date)) }
            if let bytes = value as? Data { return .object(["encoding": .string("base64"), "value": .string(bytes.base64EncodedString())]) }
            return .object(["representation": .string(String(describing: value)), "type": .string(String(describing: type(of: value)))])
        })
    }
}
