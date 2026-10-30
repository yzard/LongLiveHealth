import Testing
@testable import LongLiveHealthCore

@Test func bulkConfirmationKeepsPayloadsAndPreviousReviewDecisions() {
    let payload: JSONValue = .object(["raw_name": .string("Example marker"), "raw_result": .string("< 1"), "raw_unit": .string("mg/L")])
    let rows: [JSONValue] = (0..<128).map { index in
        .object(["observation_id": .string("row-\(index)"), "revision": .number(3), "payload": payload,
                 "status": .string(index == 126 ? "rejected" : index == 127 ? "confirmed" : "pending")])
    }
    let changes = ReportReview.pendingChanges(in: .object(["observations": .array(rows)]))
    #expect(changes.count == 126)
    #expect(changes.allSatisfy { $0["payload"] == payload && $0["expected_revision"] == .number(3) && $0["status"] == .string("confirmed") })
    #expect(!changes.contains { ["row-126", "row-127"].contains($0["observation_id"].stringValue) })
    #expect(ReportReview.pendingChanges(in: .object(["observations": .array(changes)])).isEmpty)
}
