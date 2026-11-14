import Testing
@testable import LongLiveHealthCore

@Test func bulkConfirmationIncludesOnlyPendingOCRResultsWithoutIssues() {
    let payload: JSONValue = .object([
        "raw_name": .string("Example marker"), "raw_result": .string("Negative"),
        "sampled_at": .string("2026-11-01"), "source": .object(["quote": .string("Example marker: Negative")])
    ])
    let ready: JSONValue = .object([
        "observation_id": .string("ready"), "revision": .number(3),
        "status": .string("pending"), "payload": payload
    ])
    var uncertain = ready
    uncertain["payload"]["ocr_mapping_note"] = .string("Specimen could not be verified")
    var invalid = ready
    invalid["browse_issues"] = .array([.object(["scope": .string("chart"), "code": .string("invalid_date")])])
    var manual = ready
    manual["payload"]["source"]["quote"] = .string("")
    var missingDate = ready
    missingDate["payload"]["sampled_at"] = .null
    let confirmed = ReportReview.change(ready, to: "confirmed")
    let rejected = ReportReview.change(ready, to: "rejected")
    let eligible = [ready, uncertain, invalid, manual, missingDate, confirmed, rejected].filter {
        ReportReview.canBulkConfirm($0)
    }
    #expect(eligible == [ready])
    #expect(confirmed["payload"] == payload)
    #expect(confirmed["expected_revision"] == .number(3))
}
