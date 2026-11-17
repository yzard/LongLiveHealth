import Testing
@testable import LongLiveHealthCore

@Test func bulkConfirmationIncludesOnlyPendingOCRResultsWithoutIssues() {
    let payload: JSONValue = .object([
        "metric_id": .string("example"),
        "raw_name": .string("Example marker"), "raw_result": .string("Negative"),
        "sampled_at": .string("2026-11-01"), "source": .object(["quote": .string("Example marker: Negative")])
    ])
    let ready: JSONValue = .object([
        "observation_id": .string("ready"), "revision": .number(3),
        "status": .string("pending"), "payload": payload, "resolved_metric_id": .string("example")
    ])
    var uncertain = ready
    uncertain["payload"]["metric_id"] = .null
    uncertain["resolved_metric_id"] = .null
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

@Test func rescanQueuesOnlyUnblockedServerResolvedMappings() {
    let resolved: JSONValue = .object([
        "observation_id": .string("scan"), "status": .string("pending"), "revision": .number(4),
        "resolved_metric_id": .string("example"),
        "payload": .object(["raw_name": .string("Example marker"), "raw_result": .string("4.6"),
            "sampled_at": .string("2026-11-01"), "source": .object(["quote": .string("Example marker 4.6")])]),
    ])
    var suggestion = resolved
    suggestion["resolved_metric_id"] = .null
    suggestion["payload"]["ocr_metric_candidate"] = .string("example")
    suggestion["metric_suggestions"] = .array([.object(["metric_id": .string("example")])])
    var blocked = resolved
    blocked["browse_issues"] = .array([.object(["scope": .string("chart"), "code": .string("unsupported_unit")])])
    var deliberatelyUnmapped = resolved
    deliberatelyUnmapped["payload"]["unmapped_choice"] = .object(["kind": .string("unmapped")])
    var mismatched = resolved
    mismatched["payload"]["metric_id"] = .string("another")
    let rows = [resolved, suggestion, blocked, deliberatelyUnmapped, mismatched]
    #expect(rows.filter(ReportReview.canBulkConfirm) == [resolved])
    #expect(rows.filter(ReportReview.canConfirmIndividually) == [resolved])
    #expect(ReportReview.change(resolved, to: "confirmed")["payload"] == resolved["payload"])
    #expect(resolved["status"] == .string("pending"))
}

@Test func successiveSavedMatchesMoveToReadyWhileIndependentIssuesRemain() {
    let observations: [JSONValue] = ["first", "second", "blocked"].map { id in
        .object([
            "observation_id": .string(id), "status": .string("pending"), "revision": .number(1),
            "payload": .object([
                "raw_name": .string(id), "raw_result": .string("4.6"),
                "sampled_at": .string("2026-11-01"), "source": .object(["quote": .string(id)]),
                "ocr_mapping_note": .string("Check the measurement identity"),
                "ocr_metric_candidate": .string("example")
            ]),
            "metric_suggestions": .array([
                .object(["metric_id": .string("similar")]), .object(["metric_id": .string("example")])
            ])
        ])
    }
    var report = observations
    #expect(report.filter(ReportReview.canBulkConfirm).isEmpty)
    var withoutCandidate = observations[0]
    withoutCandidate["payload"]["ocr_metric_candidate"] = .null
    #expect(ReportReview.defaultMetricID(withoutCandidate) == nil)
    #expect(!ReportReview.canConfirmIndividually(withoutCandidate))
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

    var newMetric = observations[0]
    newMetric["payload"]["unmapped_choice"] = .object(["kind": .string("new"),
        "definition": .object(["name": .string("New test"), "standard_unit": .string("1")])])
    #expect(ReportReview.defaultMetricID(newMetric) == nil)
    #expect(ReportReview.suggestedMetricID(newMetric) == nil)
    #expect(ReportReview.canBulkConfirm(newMetric))
    #expect(ReportReview.suggestionNote(newMetric) == "Check the measurement identity")
    let advisory: JSONValue = .object(["code": .string("negative_with_unit")])
    newMetric["payload"]["raw_result"] = .string("Negative")
    newMetric["browse_issues"] = .array([advisory])
    #expect(ReportReview.canConfirmIndividually(newMetric))
    #expect(!ReportReview.canBulkConfirm(newMetric))
    newMetric["browse_issues"] = .array([advisory, .object(["code": .string("unsupported_unit")])])
    #expect(!ReportReview.canConfirmIndividually(newMetric))
    newMetric["browse_issues"] = .array([])
    newMetric["payload"]["unmapped_choice"] = .object(["kind": .string("unmapped")])
    #expect(!ReportReview.canBulkConfirm(newMetric))
    #expect(!ReportReview.canConfirmIndividually(newMetric))
}
