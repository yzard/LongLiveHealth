import Foundation

/// Splits display envelopes into colored segments without adding or changing measurements.
public nonisolated enum ChartRangeSegments {
    public static func segments(rows: [JSONValue], names: [String: String]) -> [JSONValue] {
        let groups = Dictionary(grouping: rows) { row in
            "\(row["start_at"])|\(row["end_at"])|\(row["unit"])"
        }
        return groups.values.flatMap { group -> [JSONValue] in
            let ranked = group.sorted { a, b in
                let ak = a["source"].stringValue, bk = b["source"].stringValue
                let ap = OverviewSummary.devicePriority(name: names[ak] ?? ak)
                let bp = OverviewSummary.devicePriority(name: names[bk] ?? bk)
                if ap != bp { return ap < bp }
                let at = a["last_at"].numberValue ?? 0, bt = b["last_at"].numberValue ?? 0
                if at != bt { return at > bt }
                return ak < bk
            }
            let bounds = Set(group.flatMap { [$0["min"].numberValue, $0["max"].numberValue].compactMap { $0 }.filter(\.isFinite) }).sorted()
            var result: [JSONValue] = []
            for (lower, upper) in zip(bounds, bounds.dropFirst()) {
                let middle = lower + (upper - lower) / 2
                guard var winner = ranked.first(where: { ($0["min"].numberValue ?? .infinity) <= middle && ($0["max"].numberValue ?? -.infinity) >= middle }) else { continue }
                winner["display_min"] = .number(lower)
                winner["display_max"] = .number(upper)
                if let last = result.last, last["source"] == winner["source"], last["display_max"] == .number(lower) {
                    result[result.count - 1]["display_max"] = .number(upper)
                } else { result.append(winner) }
            }
            for row in ranked where row["min"] == row["max"] {
                guard let value = row["min"].numberValue,
                      let winner = ranked.first(where: { ($0["min"].numberValue ?? .infinity) <= value && ($0["max"].numberValue ?? -.infinity) >= value }),
                      winner["source"] == row["source"] else { continue }
                var point = row
                point["display_min"] = .number(value)
                point["display_max"] = .number(value)
                result.append(point)
            }
            return result
        }.sorted {
            let a = $0["start_at"].numberValue ?? 0, b = $1["start_at"].numberValue ?? 0
            return a == b ? ($0["display_min"].numberValue ?? 0) < ($1["display_min"].numberValue ?? 0) : a < b
        }
    }
}
