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
