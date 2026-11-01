import Testing
@testable import LongLiveHealthCore

@Test func characteristicSnapshotsUseLatestTimestampNotSourceOrder() {
    let older: JSONValue = .object(["at": .number(10), "version": .number(99)])
    let newer: JSONValue = .object(["at": .number(20), "version": .number(1)])
    #expect(CharacteristicValue.latest([newer, older, .null]) == newer)
    #expect(CharacteristicValue.latest([.null]) == nil)
}

@Test func characteristicValuesAreLabelsAndCalendarDates() {
    let prefix = "HKCharacteristicTypeIdentifier"
    for (kind, code, label) in [("BiologicalSex", 2.0, "Male"), ("BloodType", 8.0, "O−"), ("FitzpatrickSkinType", 4.0, "Type IV"), ("WheelchairUse", 2.0, "Yes"), ("ActivityMoveMode", 1.0, "Active energy")] {
        #expect(CharacteristicValue.text(kind: prefix + kind, details: .object(["value": .number(code)])) == label)
        #expect(CharacteristicValue.text(kind: prefix + kind, details: .null) == "—")
    }
    #expect(CharacteristicValue.text(kind: prefix + "BloodType", details: .object(["value": .string("A+")])) == "A+")
    #expect(CharacteristicValue.text(kind: prefix + "BiologicalSex", details: .object(["value": .number(900)])) == "Unknown value")
    let date: JSONValue = .object(["date_of_birth": .object(["NS.year": .number(1980), "NS.month": .number(2), "NS.day": .number(3)])])
    let result = CharacteristicValue.text(kind: prefix + "DateOfBirth", details: date)
    #expect(result.contains("1980"))
    var invalid = date
    invalid["date_of_birth"]["NS.day"] = .number(31)
    #expect(CharacteristicValue.text(kind: prefix + "DateOfBirth", details: invalid) == "—")
}
