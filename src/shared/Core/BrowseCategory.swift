import Foundation

/// A single, source-independent home for every Browse metric on both clients.
public enum BrowseCategory: String, CaseIterable, Identifiable, Sendable {
    case activity = "Activity", body = "Body Measurements", cycle = "Cycle Tracking"
    case hearing = "Hearing", heart = "Heart", medications = "Medications"
    case wellbeing = "Mental Wellbeing", mobility = "Mobility", nutrition = "Nutrition"
    case respiratory = "Respiratory", sleep = "Sleep", symptoms = "Symptoms"
    case vitals = "Vitals", bloodWork = "Blood Work", vision = "Vision", other = "Other Data"
    case allergies = "Allergies", notes = "Clinical Notes", conditions = "Conditions"
    case immunizations = "Immunizations", procedures = "Procedures"

    public var id: String { rawValue }
    public var symbol: String {
        switch self {
        case .activity: "flame.fill"
        case .body: "figure.stand"
        case .cycle: "circle.dotted"
        case .hearing: "ear.fill"
        case .heart: "heart.fill"
        case .medications: "pills.fill"
        case .wellbeing: "brain.head.profile"
        case .mobility: "figure.walk"
        case .nutrition: "carrot.fill"
        case .respiratory: "lungs.fill"
        case .sleep: "bed.double.fill"
        case .symptoms: "figure"
        case .vitals: "waveform.path.ecg"
        case .bloodWork: "drop.fill"
        case .vision: "eye.fill"
        case .other: "cross.fill"
        case .allergies: "allergens"
        case .notes: "doc.text.fill"
        case .conditions: "stethoscope"
        case .immunizations: "syringe.fill"
        case .procedures: "cross.case.fill"
        }
    }
    public var isClinical: Bool { [.allergies, .notes, .conditions, .immunizations, .procedures].contains(self) }

    public static func classify(id: String, name: String, laboratory: Bool) -> Self {
        let key = id.lowercased().filter { $0.isLetter || $0.isNumber }
        let title = name.lowercased().filter { $0.isLetter || $0.isNumber }
        func has(_ parts: [String]) -> Bool { parts.contains { key.contains($0) || title.contains($0) } }
        // Dietary cholesterol is nutrition; a blood lipid result belongs to Heart.
        if has(["dietary", "correlationtypeidentifierfood"]) { return .nutrition }
        if has(["heartrate", "heartbeat", "heartrhythm", "hrv", "cardio", "vo2", "electrocardiogram", "bloodpressure", "atrialfibrillation", "hypertension", "peripheralperfusion"]) { return .heart }
        if laboratory {
            if ["apob", "apoa1", "ldl", "hdl", "ldlc", "hdlc", "totalcholesterol", "cholesterol", "triglycerides", "nonhdl", "lpa", "hscrp", "homocysteine", "atrialrate", "ventricularrate", "qrsduration", "qtc", "qtinterval", "printerval"].contains(key)
                || has(["cholesterol", "lipoprotein", "apolipoprotein", "triglyceride", "troponin", "natriuretic"]) { return .heart }
            return .bloodWork
        }
        if has(["allergyrecord"]) { return .allergies }
        if has(["clinicalnote", "cdadocument"]) { return .notes }
        if has(["conditionrecord"]) { return .conditions }
        if has(["immunization"]) { return .immunizations }
        if has(["procedure"]) { return .procedures }
        if has(["labresult", "bloodtype", "bloodglucose", "bloodalcohol"]) { return .bloodWork }
        if has(["medication", "insulindelivery", "inhalerusage"]) { return .medications }
        if has(["sleep"]) { return .sleep }
        if has(["menstr", "menopaus", "pregnan", "ovulation", "progesterone", "cervicalmucus", "contraceptive", "lactation", "sexualactivity", "basalbodytemperature", "vaginaldryness"]) { return .cycle }
        if has(["audio", "audiogram", "hearing", "soundreduction"]) { return .hearing }
        if has(["vision"]) { return .vision }
        if has(["mindful", "stateofmind", "mood", "electrodermal"]) { return .wellbeing }
        if has(["respiratory", "oxygensaturation", "forcedexpiratory", "forcedvital", "peakexpiratory"]) { return .respiratory }
        if has(["bodymass", "bodyfat", "height", "waist", "biologicalsex", "dateofbirth", "skintype"]) { return .body }
        if has(["bodytemperature", "vitalsign"]) { return .vitals }
        if has(["walkingsteadiness", "walkingasymmetry", "walkingdouble", "walkingstep", "walkingspeed", "stairascent", "stairdescent", "sixminutewalk", "timesfallen", "wheelchairuse"]) { return .mobility }
        if has(["alcoholicbeverages"]) { return .nutrition }
        if has(["workout", "activity", "energy", "exercise", "standhour", "standtime", "movetime", "steps", "stepcount", "flights", "distance", "cycling", "running", "swimming", "rowing", "paddle", "skiing", "physicaleffort", "pushcount", "nikefuel"]) { return .activity }
        if key.hasPrefix("hkcategorytypeidentifier") && !has(["handwashing", "toothbrushing"]) { return .symptoms }
        return .other
    }
}
