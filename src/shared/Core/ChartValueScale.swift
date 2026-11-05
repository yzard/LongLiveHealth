import Foundation

/// Axis bounds for the currently visible, enabled series in display units.
public nonisolated enum ChartValueScale {
    /// Keep adjacent axis labels distinct even for very small concentrations.
    public static func tickDecimals(domain: ClosedRange<Double>) -> Int {
        let step = (domain.upperBound - domain.lowerBound) / 2
        guard step.isFinite, step > 0 else { return 1 }
        return max(1, min(8, Int(ceil(-log10(step)))))
    }

    public static func domain(values: [Double]) -> ClosedRange<Double> {
        let finite = values.filter(\.isFinite)
        guard let lower = finite.min(), let upper = finite.max() else { return 0...1 }
        let span = upper - lower
        // Flat series and single points still need a nonzero axis interval.
        let padding = span > 0 ? span * 0.1 : max(abs(lower) * 0.05, 0.01)
        return (lower - padding)...(upper + padding)
    }
}
