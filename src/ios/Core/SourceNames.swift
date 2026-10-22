import Foundation

public nonisolated enum SourceNames {
    public static func label(key: String, fallback: String, aliases: JSONValue, catalog: [JSONValue]) -> String {
        let alias = aliases[key].stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        if !alias.isEmpty { return alias }
        if key == "manual:longlivehealth" { return "Long Live Health" }
        let name = (catalog.first { $0["source_key"].stringValue == key }?["name"].stringValue ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        if !name.isEmpty && name != key { return name }
        let original = fallback.trimmingCharacters(in: .whitespacesAndNewlines)
        return original.isEmpty ? key : original
    }
    public static func reportKey(name: String, reportID: String) -> String {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? "report:" + reportID : "report-source:" + trimmed.lowercased()
    }
    public static func key(_ record: JSONValue) -> String {
        if !record["source_key"].stringValue.isEmpty { return record["source_key"].stringValue }
        let id = record["source_id"].stringValue
        if id.hasPrefix("report:") || id.hasPrefix("report-source:") { return id }
        if !record["platform"].stringValue.isEmpty { return record["platform"].stringValue + ":" + id }
        let original = record["original"]["source_id"].stringValue
        return "apple_health:" + (id == "apple_health" && !original.isEmpty ? original : id)
    }
}
