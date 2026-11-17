import Testing
@testable import LongLiveHealthCore

@Test func referenceKeepsOriginalAndStandardUnitsSeparate() {
    let payload: JSONValue = .object(["reference_range": .string("<2.586")])
    let reference: JSONValue = .object(["original_unit": .string("mmol/L"), "unit_origin": .string("observation"), "display": .string("< 100 mg/dL")])
    #expect(LaboratoryPresentation.originalReference(payload, reference: reference).contains("mmol/L (result-column unit)"))
    #expect(LaboratoryPresentation.standardizedReference(reference) == "Standardized reference: < 100 mg/dL")
    #expect(LaboratoryPresentation.originalReference(payload, reference: reference).contains("<2.586"))
}

@Test func unsupportedReferenceDoesNotInventAStandardRange() {
    let reference: JSONValue = .object(["reason": .string("Contextual reference requires review.")])
    #expect(LaboratoryPresentation.standardizedReference(reference) == "Reference retained as printed: Contextual reference requires review.")
    #expect(LaboratoryPresentation.originalReference(.object(["reference_range": .string("Age dependent")]), reference: reference).contains("unit unknown"))
}

@Test func printedNamesKeepQualifiersAndShowLoincSeparately() {
    #expect(LaboratoryPresentation.printedName("TSH (3rd Generation) ( 11580-8 )", code: "11580-8") == "TSH (3rd Generation)")
    #expect(LaboratoryPresentation.printedName("TSH (3rd Generation)", code: "11580-8") == "TSH (3rd Generation)")
    #expect(LaboratoryPresentation.printedName("TSH (11580-8)", code: "") == "TSH (11580-8)")
}

@Test func renamedMetricsMoveInAlphabeticalOrderWithStableTies() {
    let entries = [
        (id: "z", name: "Alpha"),
        (id: "a", name: "Alpha"),
        (id: "c", name: "beta"),
        (id: "b", name: "Vitamin D"),
    ]
    let ordered = entries.sorted { left, right in
        LaboratoryPresentation.nameSortsBefore(left.name, id: left.id, right.name, otherID: right.id)
    }
    #expect(ordered.map(\.id) == ["a", "z", "c", "b"])
}

@Test func reportReviewTitlesFollowTheCurrentNameOfTheSuggestedMetricID() {
    var draft: JSONValue = .object([
        "status": .string("pending"),
        "display_name": .string("Ketones - urine"),
        "lab_test_code": .string("2514-8"),
        "resolved_metric_id": .string("urine_ketones"),
        "payload": .object(["raw_name": .string("Ketone, Urine (2514-8)")]),
    ])
    let renamed = ["urine_ketones": "Urine - Ketones"]
    let metrics: [JSONValue] = [.object(["metric_id": .string("urine_ketones"), "name": .string("Urine Ketones")])]
    #expect(ReportReview.defaultMetricID(draft) == "urine_ketones")
    #expect(LaboratoryPresentation.resultTitle(draft, metrics: metrics, renamedMetrics: [:]) == "Urine Ketones")
    draft["resolved_metric_id"] = .null
    draft["metric_suggestions"] = .array([
        .object(["metric_id": .string("blood_ketones"), "name": .string("Blood Ketones")]),
        .object(["metric_id": .string("urine_ketones"), "name": .string("Ketones - urine")]),
    ])
    draft["payload"]["ocr_new_metric"] = .object(["name": .string("OCR proposed name")])
    #expect(LaboratoryPresentation.resultTitle(draft, metrics: metrics, renamedMetrics: renamed) == "Ketone, Urine")
    #expect(ReportReview.defaultMetricID(draft) == nil)
    draft["payload"]["ocr_new_metric"] = .null
    draft["payload"]["ocr_metric_candidate"] = .string("urine_ketones")
    #expect(ReportReview.defaultMetricID(draft) == "urine_ketones")
    #expect(LaboratoryPresentation.resultTitle(draft, metrics: [], renamedMetrics: [:]) == "Ketones - urine")
    #expect(LaboratoryPresentation.resultTitle(draft, metrics: metrics, renamedMetrics: renamed) == "Urine - Ketones")

    draft["payload"]["unmapped_choice"] = .object([
        "kind": .string("new"),
        "definition": .object(["name": .string("  Urinary ketone measurement  ")])
    ])
    #expect(LaboratoryPresentation.resultTitle(draft, metrics: metrics, renamedMetrics: renamed) == "Ketone, Urine")
    draft["payload"]["unmapped_choice"] = .object(["kind": .string("unmapped")])
    #expect(LaboratoryPresentation.resultTitle(draft, metrics: metrics, renamedMetrics: renamed) == "Ketone, Urine")

    draft["payload"]["unmapped_choice"] = .null
    draft["payload"]["metric_id"] = .string("urine_ketones")
    #expect(ReportReview.defaultMetricID(draft) == "urine_ketones")
    #expect(LaboratoryPresentation.resultTitle(draft, metrics: metrics, renamedMetrics: renamed) == "Urine - Ketones")
    #expect(draft["payload"]["raw_name"] == .string("Ketone, Urine (2514-8)"))
    draft["status"] = .string("submitted")
    #expect(LaboratoryPresentation.resultTitle(draft, metrics: metrics, renamedMetrics: renamed) == "Urine - Ketones")
}
