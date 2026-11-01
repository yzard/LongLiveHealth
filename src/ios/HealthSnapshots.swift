import Foundation
@preconcurrency import HealthKit

@MainActor enum HealthSnapshots {
    static let characteristics: [HKCharacteristicTypeIdentifier] = [.biologicalSex, .bloodType, .dateOfBirth, .fitzpatrickSkinType, .wheelchairUse, .activityMoveMode]
    static var readTypes: Set<HKObjectType> {
        var result = Set(characteristics.compactMap { HKObjectType.characteristicType(forIdentifier: $0) as HKObjectType? })
        result.insert(HKObjectType.activitySummaryType())
        if let type = HKObjectType.documentType(forIdentifier: .CDA) { result.insert(type) }
        if #available(iOS 26.0, *) { result.insert(HKObjectType.userAnnotatedMedicationType()) }
        return result
    }

    struct Value: Sendable { let id: String; let payload: JSONValue }
    static func kinds(includeSelectedRecords: Bool) -> [String] {
        var kinds = characteristics.map(\.rawValue) + ["activity_summary", "cda_document"]
        if #available(iOS 26.0, *), includeSelectedRecords { kinds.append("user_annotated_medications") }
        return kinds
    }

    struct Changes {
        let records: [JSONValue]
        let status: String
        let writes: [String: JSONValue]
    }

    static func changes(_ kind: String, store: HKHealthStore, archive: LocalArchive, installationID: UUID) async throws -> Changes {
        let values = try await read(kind, store: store)
        let savedName = "snapshot-\(kind).json"
        let previous = try await archive.loadAsync([JSONValue].self, name: savedName) ?? []
        let byID = Dictionary(previous.map { ($0["record_id"].stringValue, $0) }, uniquingKeysWith: { _, new in new })
        let now = Date().timeIntervalSince1970
        var records: [JSONValue] = []
        var changed: [JSONValue] = []
        for value in values {
            let old = byID[value.id]
            if let old, old["payload"] == value.payload { records.append(old); continue }
            let record: JSONValue = .object(["record_id": .string(value.id), "source_id": .string("snapshot:\(installationID.uuidString)"),
                "record_type": .string(kind), "start_at": .number(floor(now)), "end_at": .number(floor(now)),
                "version": .number(max(floor(now * 1000), (old?["version"].numberValue ?? 0) + 1)), "deleted": .bool(false), "payload": value.payload])
            records.append(record); changed.append(record)
        }
        let visibleIDs = Set(records.map { $0["record_id"].stringValue })
        records += previous.filter { !visibleIDs.contains($0["record_id"].stringValue) }
        return Changes(records: changed, status: values.isEmpty ? "no_visible_samples" : "observed", writes: [savedName: .array(records)])
    }

    private static func read(_ kind: String, store: HKHealthStore) async throws -> [Value] {
        switch kind {
        case HKCharacteristicTypeIdentifier.biologicalSex.rawValue:
            return [Value(id: kind, payload: .object(["value": .number(Double(try store.biologicalSex().biologicalSex.rawValue))]))]
        case HKCharacteristicTypeIdentifier.bloodType.rawValue:
            return [Value(id: kind, payload: .object(["value": .number(Double(try store.bloodType().bloodType.rawValue))]))]
        case HKCharacteristicTypeIdentifier.fitzpatrickSkinType.rawValue:
            return [Value(id: kind, payload: .object(["value": .number(Double(try store.fitzpatrickSkinType().skinType.rawValue))]))]
        case HKCharacteristicTypeIdentifier.wheelchairUse.rawValue:
            return [Value(id: kind, payload: .object(["value": .number(Double(try store.wheelchairUse().wheelchairUse.rawValue))]))]
        case HKCharacteristicTypeIdentifier.activityMoveMode.rawValue:
            return [Value(id: kind, payload: .object(["value": .number(Double(try store.activityMoveMode().activityMoveMode.rawValue))]))]
        case HKCharacteristicTypeIdentifier.dateOfBirth.rawValue:
            let components = try store.dateOfBirthComponents()
            return [Value(id: kind, payload: .object(["date_components_archive": .string(try NSKeyedArchiver.archivedData(withRootObject: components as NSDateComponents, requiringSecureCoding: true).base64EncodedString())]))]
        case "activity_summary":
            let summaries = try await HKActivitySummaryQueryDescriptor(predicate: nil).result(for: store)
            let calendar = Calendar(identifier: .gregorian)
            return try summaries.map { summary in
                let date = summary.dateComponents(for: calendar)
                guard let year = date.year, let month = date.month, let day = date.day else { throw APIError.invalidResponse }
                let dayID = String(format: "%04d-%02d-%02d", year, month, day)
                return Value(id: dayID, payload: .object(["date": .string(dayID), "timezone": .string(calendar.timeZone.identifier),
                    "active_energy_kcal": .number(summary.activeEnergyBurned.doubleValue(for: .kilocalorie())),
                    "exercise_minutes": .number(summary.appleExerciseTime.doubleValue(for: .minute())),
                    "stand_hours": .number(summary.appleStandHours.doubleValue(for: .count())),
                    "move_minutes": .number(summary.appleMoveTime.doubleValue(for: .minute())), "raw_archive": try archive(summary)]))
            }
        case "cda_document":
            return try await HealthSeriesReader.documents(store: store).map { document in
                Value(id: document["record_id"].stringValue, payload: .object(["original_record": document]))
            }
        case "user_annotated_medications":
            if #available(iOS 26.0, *) {
                let medications = try await HKUserAnnotatedMedicationQueryDescriptor(predicate: nil, limit: nil).result(for: store)
                guard !medications.isEmpty else { return [] }
                return [Value(id: kind, payload: .object(["raw_medications": .array(try medications.map { try archive($0) })]))]
            }
            throw APIError.invalidResponse
        default: throw APIError.invalidResponse
        }
    }
    private static func archive(_ value: Any) throws -> JSONValue {
        .object(["format": .string("nskeyedarchiver-secure-v1"), "encoding": .string("base64"),
                 "data": .string(try NSKeyedArchiver.archivedData(withRootObject: value, requiringSecureCoding: true).base64EncodedString())])
    }
}
