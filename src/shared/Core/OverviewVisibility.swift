import Foundation

public nonisolated enum OverviewVisibility {
    public static func isVisible(_ kind: String, preferences: JSONValue) -> Bool {
        let favorites = preferences["favorite_metrics"].arrayValue
        let showAll = preferences["overview_show_all"] == .null ? favorites.isEmpty : preferences["overview_show_all"].boolValue
        return showAll || favorites.contains(.string(kind))
    }
    public static func change(_ kind: String, visible: Bool, available: [String], preferences: JSONValue) -> [String: JSONValue] {
        var selected = Set(available.filter { isVisible($0, preferences: preferences) })
        if visible { selected.insert(kind) } else { selected.remove(kind) }
        let all = available.allSatisfy { selected.contains($0) }
        return ["overview_show_all": .bool(all), "favorite_metrics": .array(all ? [] : available.filter { selected.contains($0) }.map(JSONValue.string))]
    }
}
