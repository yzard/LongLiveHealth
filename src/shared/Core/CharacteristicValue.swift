import Foundation

/// Profile snapshots are discrete values, never a numeric time series.
public nonisolated enum CharacteristicValue {
    public static func latest(_ records: [JSONValue]) -> JSONValue? {
        records.filter { $0["at"].numberValue != nil }.max {
            let left = $0["at"].numberValue ?? 0
            let right = $1["at"].numberValue ?? 0
            if left != right { return left < right }
            return ($0["version"].numberValue ?? 0) < ($1["version"].numberValue ?? 0)
        }
    }

    public static func text(kind: String, details: JSONValue) -> String {
        if kind == "HKCharacteristicTypeIdentifierDateOfBirth" {
            let components = details["date_of_birth"]
            guard let year = components["NS.year"].numberValue,
                  let month = components["NS.month"].numberValue,
                  let day = components["NS.day"].numberValue,
                  year >= 1, year <= 9999, month >= 1, month <= 12, day >= 1, day <= 31 else { return "—" }
            var calendar = Calendar(identifier: .gregorian)
            calendar.timeZone = TimeZone(secondsFromGMT: 0)!
            let values = DateComponents(year: Int(year), month: Int(month), day: Int(day))
            guard let date = calendar.date(from: values), calendar.dateComponents([.year, .month, .day], from: date) == values else { return "—" }
            let formatter = DateFormatter()
            formatter.calendar = calendar
            formatter.timeZone = calendar.timeZone
            formatter.dateStyle = .long
            formatter.timeStyle = .none
            return formatter.string(from: date)
        }
        let value = details["value"]
        if case .string(let label) = value, !label.isEmpty { return label }
        guard let code = value.numberValue, code.isFinite else { return "—" }
        let labels: [String]
        switch kind {
        case "HKCharacteristicTypeIdentifierBiologicalSex": labels = ["Not set", "Female", "Male", "Other"]
        case "HKCharacteristicTypeIdentifierBloodType": labels = ["Not set", "A+", "A−", "B+", "B−", "AB+", "AB−", "O+", "O−"]
        case "HKCharacteristicTypeIdentifierFitzpatrickSkinType": labels = ["Not set", "Type I", "Type II", "Type III", "Type IV", "Type V", "Type VI"]
        case "HKCharacteristicTypeIdentifierWheelchairUse": labels = ["Not set", "No", "Yes"]
        case "HKCharacteristicTypeIdentifierActivityMoveMode": labels = ["Not set", "Active energy", "Move time"]
        default: return "—"
        }
        guard code >= 0, code < Double(labels.count), code.rounded() == code else { return "Unknown value" }
        return labels[Int(code)]
    }
}
