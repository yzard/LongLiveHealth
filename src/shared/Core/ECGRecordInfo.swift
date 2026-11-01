import Foundation

/// HealthKit source enums (HKElectrocardiogram.h); unknown future codes stay unclassified.
public nonisolated enum ECGRecordInfo {
    public static func classification(_ value: JSONValue) -> String {
        switch value.numberValue {
        case 0: "Not set"
        case 1: "Sinus rhythm"
        case 2: "Atrial fibrillation"
        case 3: "Inconclusive — low heart rate"
        case 4: "Inconclusive — high heart rate"
        case 5: "Inconclusive — poor reading"
        case 6: "Inconclusive"
        case 100: "Unrecognized"
        default: "Not available"
        }
    }
    public static func symptoms(_ value: JSONValue) -> String {
        switch value.numberValue {
        case 0: "Not reported"
        case 1: "None reported"
        case 2: "Reported"
        default: "Not available"
        }
    }
}
