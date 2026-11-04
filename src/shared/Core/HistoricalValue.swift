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
        return text.isEmpty ? "—" : text + (unit.isEmpty ? "" : " " + displayUnit(unit, kind: ""))
    }
    /// Labels preserve the magnitude of the supplied value. Conversion belongs to the server.
    public static func displayUnit(_ unit: String, kind: String) -> String {
        let metric = kind.lowercased()
        let trimmed = unit.trimmingCharacters(in: .whitespaces)
        let compact = trimmed.replacingOccurrences(of: " ", with: "").replacingOccurrences(of: "μ", with: "µ")
        switch compact {
        case "Notreported", "NotReported", "Unitunavailable": return "Unit Not Specified"
        case "degC", "Cel", "[degC]", "°C": return "°C"
        case "degF", "[degF]", "°F": return "°F"
        case "l": return "L"
        case "ml", "ML", "10^-3L", "10*-3.L", "10⁻³L": return "mL"
        case "ul", "uL", "µL", "10^-6L", "10*-6.L", "10⁻⁶L": return "µL"
        case "nl", "10^-9L", "10*-9.L", "10⁻⁹L": return "nL"
        case "pl", "10^-12L", "10*-12.L", "10⁻¹²L": return "pL"
        case "fl", "10^-15L", "10*-15.L", "10⁻¹⁵L": return "fL"
        case "ml/kg*min", "mL/kg/min", "ml/(kg*min)", "ml/min·kg": return "mL/(kg·min)"
        case "mL/min/1.73m2", "ml/min/1.73m2", "mL/min/{1.73_m2}": return "mL/min per 1.73 m²"
        case "kcal/(kg*hr)", "kcal/hr·kg": return "kcal/(kg·h)"
        case "kg/m2", "kg/m^2": return "kg/m²"
        case "10^9/L", "10⁹/L", "10*9/L", "x10^9/L", "x10(9)/L", "×10⁹/L", "×10^9/L", "10^9{cells}/L", "10*9{cells}/L":
            return "×10⁹ cells/L"
        case "10^12/L", "10¹²/L", "10*12/L", "10^12{cells}/L", "10*12{cells}/L":
            return "×10¹² cells/L"
        case "K/uL", "K/µL", "Thousand/uL", "Thousand/µL", "x10(3)/uL", "x10(3)/µL", "10*3/uL", "10*3/µL", "10^3/uL", "10^3/µL", "10³/uL", "10³/µL", "x10^3/uL", "x10^3/µL", "×10³/uL", "×10³/µL", "×10^3/uL", "×10^3/µL":
            return "×10³ cells/µL"
        case "M/uL", "M/µL", "Million/uL", "Million/µL", "x10(6)/uL", "x10(6)/µL", "10*6/uL", "10*6/µL", "10^6/uL", "10^6/µL":
            return "×10⁶ cells/µL"
        case "cells/uL", "cell/uL", "cells/µL", "cell/µL", "#/uL", "count/uL", "{cells}/uL": return "cells/µL"
        case "cell/L", "count/L", "{cells}/L", "{count}/L": return "cells/L"
        case "cells/ml", "cell/mL", "count/mL", "{cells}/mL": return "cells/mL"
        case "/100WBC's", "/100WBC’s", "/100WBC": return "per 100 white blood cells"
        case "mcg/L", "ug/L", "UG/L": return "µg/L"
        case "mcg/dL", "ug/dL", "ug/dl": return "µg/dL"
        case "mcg/dL(calc)", "ug/dL(calc)": return "µg/dL (calculated)"
        case "g/dL(calc)", "g/dl(calc)": return "g/dL (calculated)"
        case "%(calc)": return "% (calculated)"
        case "%bywt": return "% by weight"
        case "ug/mL", "ug/ml", "UG/ML": return "µg/mL"
        case "umol/L", "umol/l", "UMOL/L": return "µmol/L"
        case "uIU/mL", "uIU/ml", "µIU/ml": return "µIU/mL"
        case "NG/ML", "NG/mL", "NG/ml", "ng/ml": return "ng/mL"
        case "NG/DL", "NG/dL", "NG/dl", "ng/dl": return "ng/dL"
        case "NG/L", "ng/l": return "ng/L"
        case "MG/DL", "mg/dl": return "mg/dL"
        case "MG/L", "mg/l": return "mg/L"
        case "G/DL", "g/dl": return "g/dL"
        case "G/L", "g/l": return "g/L"
        case "MMOL/L", "mmol/l": return "mmol/L"
        case "mIU/l": return "mIU/L"
        case "mIU/ml": return "mIU/mL"
        case "IU/ml": return "IU/mL"
        case "U/ml": return "U/mL"
        case "PG/ML", "PG/mL", "pg/ml": return "pg/mL"
        case "PG": return "pg"
        case "Angstrom", "angstrom": return "Å"
        case "sec", "seconds", "second(s)": return "s"
        case "dBASPL": return "dB(A) SPL"
        case "appleEffortScore": return "effort score"
        case "1": return "unitless"
        case "count" where metric.contains("bodymassindex") || metric == "bmi": return "kg/m²"
        case "count" where metric.contains("nikefuel"): return "points"
        case "count/min", "count/minute", "counts/min", "countperminute", "countsperminute":
            if metric.contains("heart") { return "BPM" }
            if metric.contains("respiratory") { return "breaths/min" }
            if metric.contains("cadence") { return "rpm" }
            return "counts/min"
        case "beats/min", "beats/minute", "bpm": return "BPM"
        default: return trimmed
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
