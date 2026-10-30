/// Axis bounds for the currently visible, enabled series in display units.
public nonisolated enum ChartValueScale {
    public static func domain(values: [Double]) -> ClosedRange<Double> {
        let finite = values.filter(\.isFinite)
        guard let lower = finite.min(), let upper = finite.max() else { return 0...1 }
        let span = upper - lower
        // Flat series and single points still need a nonzero axis interval.
        let padding = span > 0 ? span * 0.1 : max(abs(lower) * 0.05, 0.01)
        return (lower - padding)...(upper + padding)
    }
}
