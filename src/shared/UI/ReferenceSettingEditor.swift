import SwiftUI

enum ReferenceSetting: String, Identifiable {
    case birthDate, biologicalSex
    var id: String { rawValue }
    var title: String { self == .birthDate ? "Date of Birth" : "Biological Sex" }
}

struct ReferenceSettingEditor: View {
    var model: AppModel
    let setting: ReferenceSetting
    private func date(_ value: JSONValue) -> Date {
        let formatter = DateFormatter(); formatter.dateFormat = "yyyy-MM-dd"; formatter.timeZone = .gmt
        return formatter.date(from: value.stringValue) ?? Date()
    }
    private func dateString(_ value: Date) -> String {
        let formatter = DateFormatter(); formatter.dateFormat = "yyyy-MM-dd"; formatter.timeZone = .gmt
        return formatter.string(from: value)
    }
    var body: some View {
        AutoSavedPreferencesForm(model: model, title: setting.title) { preferences, update in
            let profile = preferences["reference_profile"]
            Section {
                if setting == .birthDate {
                Toggle("Use date of birth", isOn: Binding(get: { profile["birth_date"] != .null }, set: { enabled in
                    var changed = profile == .null ? JSONValue.object([:]) : profile
                    changed["birth_date"] = enabled ? .string(dateString(Date())) : .null
                    update(["reference_profile": changed])
                }))
                if profile["birth_date"] != .null {
                    DatePicker("Date of Birth", selection: Binding(get: { date(profile["birth_date"]) }, set: { value in
                        var changed = profile; changed["birth_date"] = .string(dateString(min(value, Date()))); update(["reference_profile": changed])
                    }), displayedComponents: .date).environment(\.timeZone, .gmt)
                }
                } else {
                Picker("Biological Sex", selection: Binding(get: { profile["reference_sex"].stringValue }, set: { value in
                    var changed = profile == .null ? JSONValue.object([:]) : profile
                    changed["reference_sex"] = value.isEmpty ? .null : .string(value)
                    update(["reference_profile": changed])
                })) {
                    Text("Not specified").tag("")
                    Text("Female").tag("female")
                    Text("Male").tag("male")
                }.accessibilityIdentifier("biological-sex-picker")
                }
            }
        }
    }
}
