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

@Test func successiveSavedMatchesMoveToReadyWhileIndependentIssuesRemain() {
    let observations: [JSONValue] = ["first", "second", "blocked"].map { id in
        .object([
            "observation_id": .string(id), "status": .string("pending"), "revision": .number(1),
            "payload": .object([
                "raw_name": .string(id), "raw_result": .string("4.6"),
                "sampled_at": .string("2026-11-01"), "source": .object(["quote": .string(id)]),
                "ocr_mapping_note": .string("Check the measurement identity")
            ]),
            "metric_suggestions": .array([.object(["metric_id": .string("example")])])
        ])
    }
    var report = observations
    #expect(report.filter(ReportReview.canBulkConfirm).isEmpty)
    for index in report.indices {
        let id = ReportReview.defaultMetricID(report[index])
        #expect(id == "example")
        report[index]["payload"]["metric_id"] = .string(id ?? "")
        report[index]["resolved_metric_id"] = .string(id ?? "")
        report[index]["revision"] = .number(2)
        if index == 2 {
            report[index]["browse_issues"] = .array([
                .object(["scope": .string("review"), "code": .string("inconsistent_subcount")])
            ])
        }
        #expect(report.filter(ReportReview.canBulkConfirm).count == min(index + 1, 2))
    }
    #expect(report.allSatisfy { $0["status"] == .string("pending") })
}
