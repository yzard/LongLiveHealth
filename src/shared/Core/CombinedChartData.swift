import Foundation

/// Combines server-reduced series without merging different measurements or units.
public nonisolated enum CombinedChartData {
    public static func units(_ response: JSONValue) -> [String] {
        Array(Set(response["components"].arrayValue.map { $0["unit"].stringValue })).sorted()
    }
    public static func ranges(_ response: JSONValue, unit: String) -> JSONValue {
        let components = response["components"].arrayValue.filter { $0["unit"].stringValue == unit }
        var result: JSONValue = .object(["components": .array(components), "latest_reading": response["latest_reading"]])
        for period in ["H", "D", "W", "M", "6M", "Y", "reference_days"] {
            result[period] = .array(components.flatMap { component in
                component["chart_ranges"][period].arrayValue.map { value in
                    var row = value
                    row["component"] = component["id"]
                    return row
                }
            })
        }
        return result
    }
}
