import Foundation

public nonisolated enum HealthSyncProgress {
    public static func name(_ identifier: String) -> String {
        let aliases = [
            "HKQuantityTypeIdentifierActiveEnergyBurned": "Active Energy",
            "HKQuantityTypeIdentifierBasalEnergyBurned": "Resting Energy",
            "active_energy": "Active Energy", "active_energy_burned": "Active Energy",
            "basal_energy": "Resting Energy", "basal_energy_burned": "Resting Energy",
            "hrv_sdnn": "Heart Rate Variability (SDNN)", "vo2_max": "VO₂ Max",
            "electrocardiogram": "ECG", "activity_summary": "Activity Rings",
            "cda_document": "Clinical Documents", "user_annotated_medications": "Medications"
        ]
        if let alias = aliases[identifier] { return alias }
        var value = identifier
        for prefix in ["HKQuantityTypeIdentifier", "HKCategoryTypeIdentifier", "HKCorrelationTypeIdentifier", "HKClinicalTypeIdentifier", "HKCharacteristicTypeIdentifier"] {
            if value.hasPrefix(prefix) { value.removeFirst(prefix.count); break }
        }
        value = value.replacingOccurrences(of: "([A-Z]+)([A-Z][a-z])", with: "$1 $2", options: .regularExpression)
            .replacingOccurrences(of: "([a-z0-9])([A-Z])", with: "$1 $2", options: .regularExpression)
            .replacingOccurrences(of: "_", with: " ")
        return value.split(separator: " ").map { word in
            let text = String(word)
            return text == text.uppercased() ? text : text.prefix(1).uppercased() + text.dropFirst()
        }.joined(separator: " ")
    }

    public static func totalRecords(in response: JSONValue) throws -> Int {
        guard let number = response["total_records"].numberValue,
              let total = Int(exactly: number), total >= 0 else { throw APIError.invalidResponse }
        return total
    }

    public static func status(recordType: String, total: Int?) -> String {
        guard let total else { return "Syncing - \(name(recordType))" }
        return "Syncing \(total.formatted()) - \(name(recordType))"
    }
}
