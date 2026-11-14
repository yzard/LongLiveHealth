import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func sleepPresentationUsesHoursMinutesAndNeverSeconds() {
    #expect(MetricPresentation.duration(27000) == "7 hr 30 min")
    #expect(MetricPresentation.duration(3599) == "1 hr 0 min")
    #expect(MetricPresentation.duration(0) == "0 hr 0 min")
    #expect(MetricPresentation.duration(.nan) == "Not available")
    #expect(MetricPresentation.duration(-1) == "Not available")
}

