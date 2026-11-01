import Foundation

public nonisolated enum HistoricalValue {
    private static let formatter: DateFormatter = {
        let value = DateFormatter()
        value.locale = Locale(identifier: "en_US_POSIX")
        value.calendar = Calendar(identifier: .gregorian)
        value.timeZone = .autoupdatingCurrent
        value.dateFormat = "yyyy-MM-dd hh:mm a"
        return value
    }()
    public static func date(timestamp: Double?, text: String) -> String {
        if let timestamp, timestamp.isFinite { return formatter.string(from: Date(timeIntervalSince1970: timestamp)) }
        if text.count == 10 {
            let parser = DateFormatter()
            parser.locale = Locale(identifier: "en_US_POSIX")
            parser.dateFormat = "yyyy-MM-dd"; parser.isLenient = false
            if parser.date(from: text) != nil { return text + " —" }
        }
        let parser = ISO8601DateFormatter()
        parser.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        if let date = parser.date(from: text) { return formatter.string(from: date) }
        parser.formatOptions = [.withInternetDateTime]
        if let date = parser.date(from: text) { return formatter.string(from: date) }
        return "—"
    }
    public static func newestFirst(_ rows: [JSONValue], timestamps: [String], dates: [String]) -> [JSONValue] {
        let iso = ISO8601DateFormatter()
        let day = DateFormatter(); day.locale = Locale(identifier: "en_US_POSIX")
        day.calendar = Calendar(identifier: .gregorian); day.timeZone = .gmt
        day.dateFormat = "yyyy-MM-dd"; day.isLenient = false
        func time(_ row: JSONValue) -> Double? {
            for key in timestamps { if let value = row[key].numberValue, value.isFinite { return value } }
            for key in dates {
                let text = row[key].stringValue
                if text.count == 10, let value = day.date(from: text) { return value.timeIntervalSince1970 }
                iso.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
                if let value = iso.date(from: text) { return value.timeIntervalSince1970 }
                iso.formatOptions = [.withInternetDateTime]
                if let value = iso.date(from: text) { return value.timeIntervalSince1970 }
            }
            return nil
        }
        return rows.enumerated().map { (index: $0.offset, row: $0.element, time: time($0.element)) }.sorted {
            let left = $0.time ?? -.infinity, right = $1.time ?? -.infinity
            return left == right ? $0.index < $1.index : left > right
        }.map(\.row)
    }
    public struct Row: Identifiable, Sendable {
        public let id: String
        public let value: JSONValue
    }
    public static func keyedRows(_ rows: [JSONValue], namespace: String) -> [Row] {
        rows.enumerated().map { Row(id: "\(namespace):\($0.offset)", value: $0.element) }
    }
    public static func sleepSessions(_ sources: [JSONValue]) -> [Row] {
        let sessions = sources.flatMap { source in
            source["sessions"].arrayValue.map { session in
                var value = session
                value["source"] = source["source"]
                return value
            }
        }
        return keyedRows(newestFirst(sessions, timestamps: ["end_at", "start_at"], dates: []), namespace: "sleep-session")
    }
    public static func dailyRows(_ history: [JSONValue]) -> [Row] {
        newestFirst(history, timestamps: [], dates: ["date"]).enumerated().flatMap { index, day in
            keyedRows(day["alternatives"].arrayValue, namespace: "daily-history:\(index)").map { row in
                var value = row.value
                value["date"] = day["date"]
                return Row(id: row.id, value: value)
            }
        }
    }
    public static func quantity(_ value: JSONValue, unit: String) -> String {
        let text = value.numberValue?.formatted(.number.precision(.fractionLength(0...4))) ?? value.stringValue
        return text.isEmpty ? "—" : text + (unit.isEmpty ? "" : " " + unit)
    }
    /// Rate units are presentation aliases; stored quantities and values are unchanged.
    public static func displayUnit(_ unit: String, kind: String) -> String {
        guard kind.lowercased().contains("heart") else { return unit }
        return switch unit.lowercased().replacingOccurrences(of: " ", with: "") {
        case "count/min", "count/minute", "counts/min", "countperminute", "countsperminute", "beats/min", "beats/minute", "bpm": "BPM"
        default: unit
        }
    }
    public static func record(_ record: JSONValue) -> String {
        let values = record["values"].arrayValue
        let kind = record["record_type"].stringValue
        if kind == "HKCorrelationTypeIdentifierBloodPressure" || kind == "blood_pressure" || kind == "blood_pressure_systolic" || kind == "blood_pressure_diastolic" {
            let content = record["entry"]["content"]
            let sys = values.first { $0["id"] == .string("blood_pressure_systolic") }?["value"] ?? (kind == "blood_pressure_systolic" ? record["details"]["value"] : content["systolic_mmhg"])
            let dia = values.first { $0["id"] == .string("blood_pressure_diastolic") }?["value"] ?? (kind == "blood_pressure_diastolic" ? record["details"]["value"] : content["diastolic_mmhg"])
            return "\(quantity(sys, unit: "")) / \(quantity(dia, unit: "")) mmHg"
        }
        if !values.isEmpty { return values.map { quantity($0["value"], unit: displayUnit($0["unit"].stringValue, kind: kind)) }.joined(separator: " · ") }
        if kind.hasPrefix("HKCharacteristic") { return CharacteristicValue.text(kind: kind, details: record["details"]) }
        for key in ["value", "category", "classification"] where record["details"][key] != .null {
            return quantity(record["details"][key], unit: displayUnit(record["details"]["unit"].stringValue, kind: kind))
        }
        return "—"
    }
}
