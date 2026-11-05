import Testing
@testable import LongLiveHealthCore

@Test func chartAxisFitsVisibleValuesWithHeadroom() {
    let domain = ChartValueScale.domain(values: [32, 36, 40])
    #expect(domain == 31.2...40.8)
    // A different visible time window or enabled-source set has its own bounds.
    #expect(ChartValueScale.domain(values: [35, 36]) == 34.9...36.1)
    #expect(ChartValueScale.domain(values: [32, 40, 100]).upperBound > domain.upperBound)
    // Converted display units (sleep hours rather than seconds).
    #expect(ChartValueScale.domain(values: [21600.0 / 3600, 28800.0 / 3600]) == 5.8...8.2)
}

@Test func chartAxisHandlesFlatNegativeAndEmptySeries() {
    let flat = ChartValueScale.domain(values: [36, 36])
    #expect(flat.lowerBound < 36 && flat.upperBound > 36)
    let zero = ChartValueScale.domain(values: [0])
    #expect(zero.lowerBound < 0 && zero.upperBound > 0)
    let negative = ChartValueScale.domain(values: [-5, -2])
    #expect(negative == -5.3 ... -1.7)
    #expect(ChartValueScale.domain(values: []) == 0...1)
    #expect(ChartValueScale.domain(values: [.nan, .infinity]) == 0...1)
}

@Test func smallTrendAxisTicksRemainDistinct() {
    #expect(ChartValueScale.tickDecimals(domain: -0.01...0.01) == 2)
    #expect(ChartValueScale.tickDecimals(domain: 0.0001...0.0003) >= 4)
    #expect(ChartValueScale.tickDecimals(domain: 60...140) == 1)
}
